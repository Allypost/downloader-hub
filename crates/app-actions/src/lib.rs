use std::path::Path;

use app_logger::debug;

pub mod actions;
pub(crate) mod common;
pub mod downloaders;
pub mod extractors;
pub mod fixers;

#[tracing::instrument]
pub async fn download_file<R>(request: R, download_dir: &Path) -> Vec<downloaders::DownloaderReturn>
where
    R: Into<extractors::ExtractInfoRequest> + Send + Sync + std::fmt::Debug,
{
    let request = request.into();

    debug!(?request, "Extracting info");

    let info = match extractors::extract_info(&request).await {
        Ok(x) => x,
        Err(e) => {
            return vec![Err(format!("Failed to extract info from {request:?}: {e}"))];
        }
    };

    debug!(?info, "Extracted info");

    let download_requests = downloaders::DownloadRequest::from_extracted_info(&info, download_dir);

    debug!(?download_requests, "Download requests");

    let download_results = {
        let futs = download_requests
            .into_iter()
            .map(|x| async move { downloaders::download_file(&x).await });

        futures::future::join_all(futs).await
    };

    debug!(?download_results, "Download results");

    download_results
}

#[tracing::instrument]
pub async fn fix_file<R>(request: R) -> fixers::FixerReturn
where
    R: Into<fixers::FixRequest> + Send + Sync + std::fmt::Debug,
{
    fixers::fix_file(request.into()).await
}
