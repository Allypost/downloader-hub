use std::{
    collections::HashMap,
    fs::Metadata,
    path::{Path, PathBuf},
    sync::Arc,
};

use app_actions::{download_file, fix_file};
use app_config::Config;
use app_helpers::{
    file_type::{infer_file_type, mime},
    id::time_thread_id,
    temp_dir::TempDir,
};
use parking_lot::Mutex;
use teloxide::{
    net::Download,
    payloads::SendMediaGroupSetters,
    prelude::{Request, Requester},
    types::{
        InputFile, InputMedia, InputMediaAudio, InputMediaDocument, InputMediaPhoto,
        InputMediaVideo, MediaKind, Message, MessageEntityKind, MessageKind, PhotoSize,
    },
};
use tokio::fs::File;
use tracing::{debug, field, info, trace, Span};
use url::Url;

const MAX_PAYLOAD_SIZE: u64 = {
    let kb = 1000;
    let mb = kb * 1000;

    50 * mb
};

use super::{Handler, HandlerError, HandlerReturn};
use crate::{
    bot::TelegramBot,
    queue::task::{Task, TaskInfo},
};

#[derive(Debug)]
pub struct DownloadRequestHandler;
#[async_trait::async_trait]
impl Handler for DownloadRequestHandler {
    fn name(&self) -> &'static str {
        "download-request"
    }

    fn can_handle(&self, task: &Task) -> bool {
        matches!(task.info(), TaskInfo::DownloadRequest { .. })
    }

    async fn handle(&self, task: &Task) -> Result<HandlerReturn, HandlerError> {
        trace!(?task, "Handling download request");

        task.update_status_message("Processing the request...")
            .await;

        let msg = match task.info() {
            TaskInfo::DownloadRequest { message } => message,

            #[allow(unreachable_patterns)]
            _ => return Err(HandlerError::Fatal("Invalid task info".to_string())),
        };

        trace!(?msg, "Got message from task");

        if let Some(user) = msg.from() {
            Span::current().record("uid", field::display(user.id.0));
            Span::current().record("name", field::debug(user.full_name()));

            if let Some(username) = user.username.as_deref() {
                Span::current().record("username", field::debug(username));
            }
        }

        info!(task_id = ?task.id(), "Handling download request");

        let temp_download_dir = TempDir::in_tmp_with_prefix(&format!(
            "downloader-hub.telegram-download.{}.",
            task.id()
        ))?;

        debug!("Downloading files");
        let paths_to_fix = download_files(temp_download_dir.path(), task, msg).await?;
        debug!("Downloaded files");
        trace!(?paths_to_fix, "Downloaded files");

        if paths_to_fix.is_empty() {
            task.update_status_message("No supported URL or file found in message")
                .await;

            return Ok(HandlerReturn::default().cleanup_status_message(false));
        }

        task.update_status_message("Fixing files...").await;

        trace!(?paths_to_fix, "Fixing files");
        debug!("Fixing files");
        let (fixed_file_paths, msg_to_send) = fix_files(&paths_to_fix).await?;

        if let Some(msg) = msg_to_send {
            task.send_additional_status_message(&msg).await;
        }
        debug!("Fixed files");
        trace!(?fixed_file_paths, "Fixed files");

        if let Some(owner_id) = Config::global().telegram_bot().owner_id {
            if msg.from().is_some_and(|user| user.id.0 == owner_id) {
                task.update_status_message("Copying files to download directory...")
                    .await;

                debug!("Copying files to download directory");
                copy_files_to_save_dir(fixed_file_paths.clone()).await?;
                debug!("Copied files to download directory");
            }
        }

        send_files_to_telegram(task, fixed_file_paths).await?;

        trace!("Deleting status message");
        let _ = task.status_message().delete_message().await;
        trace!("Status message deleted");

        Ok(HandlerReturn::default())
    }
}

#[tracing::instrument(skip_all)]
async fn copy_files_to_save_dir(fixed_file_paths: Vec<PathBuf>) -> Result<(), HandlerError> {
    let download_dir = match Config::global().telegram_bot().owner_download_dir.as_ref() {
        Some(x) => x,
        None => return Ok(()),
    };

    for file in fixed_file_paths {
        let Some(file_name) = file.file_name() else {
            continue;
        };
        let dest = download_dir.join(file_name);

        trace!(?file, ?dest, "Copying file to download directory");

        tokio::fs::copy(&file, &dest)
            .await
            .map_err(|e| HandlerError::Fatal(e.to_string()))?;

        trace!(?file, ?dest, "Copied file to download directory");
    }

    Ok(())
}

#[tracing::instrument(skip_all)]
async fn send_files_to_telegram(
    task: &Task,
    fixed_file_paths: Vec<PathBuf>,
) -> Result<(), HandlerError> {
    trace!("Chunking files by size");
    let (media_groups, failed_files) =
        files_to_input_media_groups(fixed_file_paths, MAX_PAYLOAD_SIZE / 10 * 8).await;
    trace!(?media_groups, ?failed_files, "Chunked files by size");

    debug!("Uploading files to Telegram");
    for media_group in media_groups {
        trace!(?media_group, "Uploading media group");

        TelegramBot::instance()
            .send_media_group(task.status_message().chat_id(), media_group)
            .reply_to_message_id(task.status_message().msg_replying_to_id())
            .allow_sending_without_reply(true)
            .send()
            .await
            .map_err(|x| HandlerError::Fatal(x.to_string()))?;

        trace!("Uploaded media group");
    }
    debug!("Uploaded files to Telegram");

    if !failed_files.is_empty() {
        debug!(?failed_files, "Failed to chunk some files to size");
        trace!("Generating failed files message");
        let failed_files_msg = {
            let mut msg = "Failed to upload some files:\n\n".to_string();

            msg += failed_files
                .into_iter()
                .map(|(file, reason)| {
                    format!(
                        " - File: {}\n   Reason: {}\n",
                        file.file_name().unwrap_or_default().to_string_lossy(),
                        reason
                    )
                })
                .reduce(|a, b| a + "\n" + &b)
                .unwrap_or_default()
                .as_str();

            msg
        };
        trace!(msg = ?failed_files_msg, "Failed files message generated");

        trace!("Sending failed files message");
        task.send_additional_status_message(failed_files_msg.trim())
            .await;
        trace!("Failed files message sent");
    }

    Ok(())
}

#[tracing::instrument(skip_all)]
async fn fix_files(
    paths_to_fix: &[PathBuf],
) -> Result<(Vec<PathBuf>, Option<String>), HandlerError> {
    let mut fixed_file_paths = vec![];
    let mut fix_errors = vec![];
    for path in paths_to_fix {
        debug!(?path, "Fixing file");

        if !path.exists() {
            return Err(HandlerError::Fatal(format!(
                "Downloaded file not found: {:?}",
                path
            )));
        }

        trace!(?path, "Fixing file");
        let res = fix_file(path).await;
        trace!(?res, "Fixed file");

        match res {
            Ok(fixed) => fixed_file_paths.push(fixed.file_path),
            Err(e) => fix_errors.push(e.to_string()),
        }
    }

    let msg_text = if fix_errors.is_empty() {
        None
    } else {
        let text = format!(
            "Failed to fix some files:\n\n{errs}",
            errs = fix_errors
                .iter()
                .map(|x| format!("- {err}", err = x))
                .collect::<Vec<_>>()
                .join("\n"),
        );

        Some(text)
    };

    return Ok((fixed_file_paths, msg_text));
}

#[tracing::instrument(skip_all)]
async fn download_files(
    download_dir: &Path,
    task: &Task,
    msg: &Message,
) -> Result<Vec<PathBuf>, HandlerError> {
    let file_id = file_id_from_message(msg);
    let file_urls = file_urls_from_message(msg);

    if file_id.is_none() && file_urls.is_empty() {
        return Ok(vec![]);
    }

    trace!(?file_id, ?file_urls, "Found message parts to process");

    let mut paths_to_fix = vec![];

    if let Some(file_id) = file_id {
        debug!(?file_id, "Downloading file from telegram");
        task.update_status_message("Downloading file from Telegram...")
            .await;

        trace!(?file_id, "Downloading file from telegram");
        let download_file_path = download_file_by_id(&file_id, download_dir)
            .await
            .map_err(HandlerError::Fatal)?;
        trace!(?download_file_path, "Downloaded file from telegram");

        paths_to_fix.push(download_file_path);
    }

    if !file_urls.is_empty() {
        debug!(?file_urls, "Downloading files from URLs");
        task.update_status_message("Downloading files from URLs...")
            .await;

        trace!(?file_urls, "Downloading files from URLs");

        let (downloaded_file_paths, download_errors) =
            download_files_from_urls(&file_urls, download_dir).await;

        for error in download_errors {
            task.send_additional_status_message(&error).await;
        }

        trace!(?downloaded_file_paths, "Downloaded files from URLs");

        paths_to_fix.extend(downloaded_file_paths);
    }

    Ok(paths_to_fix)
}

#[tracing::instrument(skip_all)]
async fn files_to_input_media_groups<TFiles, TFile>(
    files: TFiles,
    max_size: u64,
) -> (Vec<Vec<InputMedia>>, Vec<(PathBuf, String)>)
where
    TFiles: IntoIterator<Item = TFile> + Send + std::fmt::Debug,
    TFile: AsRef<Path> + Into<PathBuf> + Clone,
{
    #[derive(Debug)]
    struct FileInfo {
        path: PathBuf,
        metadata: Metadata,
        mime: Option<mime::Mime>,
    }

    #[derive(Debug)]
    struct FileInfoWithMedia {
        file_info: FileInfo,
        media: InputMedia,
    }

    fn chunk(
        items: Vec<FileInfoWithMedia>,
        max_size: u64,
    ) -> (Vec<Vec<FileInfoWithMedia>>, Vec<(PathBuf, String)>) {
        let mut failed = vec![];
        let mut res = vec![];
        let mut res_size = 0_u64;
        let mut res_item = vec![];
        for item in items {
            let path = item.file_info.path.clone();
            let size = item.file_info.metadata.len();

            if res_item.len() >= 10 {
                res.push(res_item);
                res_item = vec![];
                res_size = 0;
            }

            if size > max_size {
                trace!(?path, ?size, ?max_size, "File is too large");
                {
                    failed.push((path, format!("file is too large: {} > {}", size, max_size)));
                }
                continue;
            }

            if size + res_size > MAX_PAYLOAD_SIZE {
                res.push(res_item);
                res_size = 0;
                res_item = vec![];
            }

            res_item.push(item);
            res_size += size;
        }

        if !res_item.is_empty() {
            res.push(res_item);
        }

        (res, failed)
    }

    let failed = Arc::new(Mutex::new(Vec::new()));
    trace!(?files, "Getting file infos");
    let file_info = {
        let futs = files
            .into_iter()
            .map(|x| x.as_ref().to_path_buf())
            .map(|file_path| {
                let failed = failed.clone();

                async move {
                    let mime = {
                        let file_path = file_path.clone();

                        tokio::task::spawn_blocking(move || infer_file_type(&file_path).ok())
                            .await
                            .ok()?
                    };

                    let metadata = match tokio::fs::metadata(&file_path).await {
                        Ok(meta) => Some(meta),
                        Err(e) => {
                            trace!(?e, "Failed to get metadata for file");
                            {
                                failed.lock().push((
                                    file_path.clone(),
                                    "failed to get metadata for file".to_string(),
                                ));
                            }

                            None
                        }
                    }?;

                    Some(FileInfo {
                        path: file_path,
                        mime,
                        metadata,
                    })
                }
            });

        futures::future::join_all(futs)
            .await
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
    };
    trace!(?file_info, "Got file infos");

    trace!("Converting to media files");
    let media_files = file_info.into_iter().map(|file_info| {
        let input_file = InputFile::file(file_info.path.clone());

        // Handle the GIFs as animations because Telegram
        // Optional todo: Also handle silent videos as animations
        if file_info
            .mime
            .as_ref()
            .is_some_and(|x| x.essence_str() == "image/gif")
        {
            return FileInfoWithMedia {
                file_info,
                media: InputMedia::Document(InputMediaDocument::new(input_file)),
            };
        }

        let file_type = file_info
            .mime
            .as_ref()
            .map(|f| f.type_().as_str().to_lowercase());
        let media = match file_type.as_deref() {
            Some("audio") => InputMedia::Audio(InputMediaAudio::new(input_file)),
            Some("image") => InputMedia::Photo(InputMediaPhoto::new(input_file)),
            Some("video") => InputMedia::Video(InputMediaVideo::new(input_file)),
            _ => InputMedia::Document(InputMediaDocument::new(input_file)),
        };

        FileInfoWithMedia { file_info, media }
    });
    trace!(?media_files, "Converted to media files");

    let chunkable_groups = {
        #[derive(Debug, Eq, PartialEq, Hash)]
        enum ChunkGroup {
            Document,
            Audio,
            Other,
        }

        let mut groups: HashMap<ChunkGroup, Vec<FileInfoWithMedia>> = HashMap::new();
        for f in media_files {
            let group_name = match f.media {
                InputMedia::Audio(_) => ChunkGroup::Audio,
                InputMedia::Document(_) => ChunkGroup::Document,
                _ => ChunkGroup::Other,
            };

            if let Some(group) = groups.get_mut(&group_name) {
                group.push(f);
            } else {
                groups.insert(group_name, vec![f]);
            }
        }

        groups.into_values().collect::<Vec<_>>()
    };
    trace!(?chunkable_groups, "Partitioned files");

    let mut res = vec![];
    for group in chunkable_groups {
        let (chunks, failed_inner) = chunk(group, max_size);
        failed.lock().extend(failed_inner);
        res.extend(
            chunks
                .into_iter()
                .map(|x| x.into_iter().map(|x| x.media).collect()),
        );
    }
    trace!(?res, "Got file groupings");

    let failed = failed.lock().iter().cloned().collect();

    trace!(?failed, "Got final failed paths");

    (res, failed)
}

#[tracing::instrument(skip_all, fields(download_dir))]
async fn download_files_from_urls(
    file_urls: &[Url],
    download_dir: &Path,
) -> (Vec<PathBuf>, Vec<String>) {
    let results = {
        let futs = file_urls.iter().map(|url| async move {
            let res = download_file(url, download_dir).await;

            (url.to_string(), res)
        });

        futures::future::join_all(futs).await
    };

    let mut downloaded_paths = vec![];
    let mut errors = vec![];
    for (url, url_results) in &results {
        let errs = url_results
            .iter()
            .filter_map(|x| x.as_ref().err())
            .collect::<Vec<_>>();

        if !errs.is_empty() {
            let text = format!(
                "Failed to download file from URL: {url}\n\nErrors:\n{errs}",
                url = url,
                errs = errs
                    .iter()
                    .map(|x| format!("- {err}", err = x))
                    .collect::<Vec<_>>()
                    .join("\n"),
            );

            errors.push(text);
        }

        let paths = url_results
            .iter()
            .filter_map(|x| x.as_ref().ok())
            .map(|x| x.path.clone());

        downloaded_paths.extend(paths);
    }

    (downloaded_paths, errors)
}

fn file_id_from_message(msg: &Message) -> Option<String> {
    let px = |x: &PhotoSize| u64::from(x.width) * u64::from(x.height);

    let msg_data = match &msg.kind {
        MessageKind::Common(x) => x,
        _ => return None,
    };

    match &msg_data.media_kind {
        MediaKind::Video(x) => Some(x.video.file.id.clone()),
        MediaKind::Animation(x) => Some(x.animation.file.id.clone()),
        MediaKind::Audio(x) => Some(x.audio.file.id.clone()),
        MediaKind::VideoNote(x) => Some(x.video_note.file.id.clone()),
        MediaKind::Photo(x) if !x.photo.is_empty() => {
            let mut photos = x.photo.clone();
            photos.sort_unstable_by(|lt, gt| {
                let pixels = px(gt).cmp(&px(lt));
                if pixels != std::cmp::Ordering::Equal {
                    return pixels;
                }

                gt.width.cmp(&lt.width)
            });

            photos.first().map(|x| x.file.id.clone())
        }
        MediaKind::Document(x) => {
            let Some(mime_type) = &x.document.mime_type else {
                return None;
            };

            if !matches!(mime_type.type_().as_str(), "image" | "video" | "audio") {
                return None;
            }

            Some(x.document.file.id.clone())
        }
        _ => None,
    }
}

fn file_urls_from_message(msg: &Message) -> Vec<Url> {
    let entities = msg
        .parse_entities()
        .or_else(|| msg.parse_caption_entities())
        .unwrap_or_default();

    entities
        .iter()
        .filter_map(|x| match x.kind() {
            MessageEntityKind::Url => Url::parse(x.text()).ok(),
            MessageEntityKind::TextLink { url } => Some(url.clone()),
            _ => None,
        })
        .collect()
}

#[tracing::instrument]
async fn download_file_by_id(file_id: &str, download_dir: &Path) -> Result<PathBuf, String> {
    trace!("Downloading file from telegram");

    let f = TelegramBot::instance()
        .get_file(file_id)
        .await
        .map_err(|e| format!("Error while getting file: {e:?}"))?;

    trace!("Got file: {:?}", f);

    let download_file_path = download_dir.join(format!(
        "{rand_id}.{id}.bin",
        rand_id = time_thread_id(),
        id = f.meta.unique_id
    ));

    trace!(
        "Downloading message file {:?} to: {:?}",
        file_id,
        &download_file_path
    );

    let mut file = File::create(&download_file_path)
        .await
        .map_err(|e| format!("Error while creating file: {e:?}"))?;

    TelegramBot::pure_instance()
        .download_file(&f.path, &mut file)
        .await
        .map_err(|e| format!("Error while downloading file: {e:?}"))?;

    trace!("Downloaded file: {:?}", file);

    file.sync_all()
        .await
        .map_err(|e| format!("Error while syncing file: {e:?}"))?;

    Ok(download_file_path)
}
