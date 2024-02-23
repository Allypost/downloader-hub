use std::{convert::Into, result::Result};

use app_entities::{
    download_request,
    entity_meta::{common::path::AppPath, download_result::DownloadResultStatus},
};
use app_validators::ip::url_resolves_to_valid_ip;
use sea_orm::{prelude::*, TransactionTrait};

use super::HandlerError;
use crate::{
    db::AppDb,
    queue::{task::Task, TASK_QUEUE},
    service::{
        download_request::{DownloadRequestService, DownloadRequestStatus},
        download_result::{CreateDownloadResultPayload, DownloadResultService},
    },
};

pub(super) async fn handle_download_request(uid: &str) -> Result<(), HandlerError> {
    match download(uid).await {
        Ok((request, paths)) => {
            if let Err(e) = add_metadata(request.id, paths).await {
                app_logger::error!(?request, ?e, "Failed to add metadata");
            }
            Ok(())
        }
        Err(e) if e.is_fatal() => {
            let err = DownloadRequestService::update_status(
                &AppDb::db(),
                uid,
                DownloadRequestStatus::Failed(e.to_string()),
            )
            .await;

            if let Err(e) = err {
                app_logger::error!(?e, "Failed to update download request");
            }

            Err(e)
        }
        Err(e) => {
            let err = DownloadRequestService::update_status(
                &AppDb::db(),
                uid,
                DownloadRequestStatus::Pending,
            )
            .await;

            if let Err(e) = err {
                app_logger::error!(?e, "Failed to update download request");
            }

            Err(e)
        }
    }
}

async fn download(uid: &str) -> Result<(download_request::Model, Vec<AppPath>), HandlerError> {
    app_logger::info!(?uid, "Got download request");

    let db = AppDb::db();
    let (request, client) = DownloadRequestService::find_by_uid_with_client(&db, uid)
        .await
        .map_err(HandlerError::Db)?
        .ok_or_else(|| HandlerError::Fatal("Download request not found".to_string()))?;

    app_logger::debug!(?request, ?client, "Got request and client");

    DownloadRequestService::update_status(&db, uid, DownloadRequestStatus::Processing).await?;

    let download_dir = client
        .resolve_download_folder()
        .map_err(|e| HandlerError::Fatal(e.to_string()))?;
    let download_url = request.url.clone();

    if let Err(e) = url_resolves_to_valid_ip(&download_url) {
        return Err(HandlerError::Fatal(e.to_string()));
    }

    let request_meta = request.meta().unwrap_or_default();
    let results = tokio::task::spawn_blocking(move || {
        app_logger::debug!(dir = ?download_dir, url = ?download_url, "Staring download");

        app_downloader::download_file(&app_downloader::downloaders::DownloadFileRequest {
            download_dir,
            original_url: download_url,
            headers: request_meta.request.headers,
            method: request_meta.request.method,
        })
    })
    .await;

    let results = match results {
        Ok(x) => x,
        Err(e) => {
            return Err(e.into());
        }
    };

    app_logger::debug!(?results, "Download completed successfully");

    let results = db
        .transaction_with_config::<_, _, DbErr>(
            |txn| {
                let uid = uid.to_string();
                Box::pin(async move {
                    DownloadRequestService::update_status(
                        txn,
                        &uid,
                        DownloadRequestStatus::Success,
                    )
                    .await?;

                    DownloadResultService::create_many(
                        txn,
                        results.iter().map(|x| match x {
                            Ok(x) => CreateDownloadResultPayload {
                                request_id: request.id,
                                status: if request_meta.skip_fixing {
                                    DownloadResultStatus::Success
                                } else {
                                    DownloadResultStatus::Pending
                                },
                                path: Some(x.path.clone()),
                                meta: None,
                            },
                            Err(e) => CreateDownloadResultPayload {
                                request_id: request.id,
                                status: DownloadResultStatus::Failed(e.clone()),
                                path: None,
                                meta: None,
                            },
                        }),
                    )
                    .await?;

                    Ok(results)
                })
            },
            Some(sea_orm::IsolationLevel::Serializable),
            Some(sea_orm::AccessMode::ReadWrite),
        )
        .await
        .map_err(|e| match e {
            sea_orm::TransactionError::Transaction(e)
            | sea_orm::TransactionError::Connection(e) => HandlerError::Db(e),
        })?;

    let successful = results
        .into_iter()
        .filter_map(Result::ok)
        .map(|x| x.path)
        .map(AppPath::LocalAbsolute)
        .collect::<Vec<_>>();

    if !request_meta.skip_fixing {
        for item in &successful {
            TASK_QUEUE.push(Task::process_download_result(request.id, item.clone()));
        }
    }

    Ok((request, successful))
}

async fn add_metadata(request_id: i32, paths: Vec<AppPath>) -> Result<(), anyhow::Error> {
    app_logger::debug!(request_id, ?paths, "Adding metadata");
    let db = AppDb::db();

    for file_path in paths {
        let res = DownloadResultService::add_app_meta(&db, request_id, file_path).await;

        if let Err(e) = res {
            app_logger::warn!(?e, "Failed to update app meta");
        }
    }

    Ok(())
}
