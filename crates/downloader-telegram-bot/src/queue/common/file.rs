use std::{
    collections::HashMap,
    fs::Metadata,
    path::{Path, PathBuf},
    sync::Arc,
};

use app_actions::fixers::{handlers::file_extensions::FileExtension, FixRequest, Fixer};
use app_helpers::{
    file_type::{infer_file_type, mime},
    id::time_thread_id,
};
use futures::{stream::FuturesUnordered, StreamExt};
use parking_lot::Mutex;
use teloxide::{
    net::Download,
    prelude::*,
    types::{
        InputFile, InputMedia, InputMediaAudio, InputMediaDocument, InputMediaPhoto,
        InputMediaVideo, MediaKind, MessageKind, PhotoSize,
    },
};
use tokio::fs::File;
use tracing::{debug, trace};

use crate::bot::TelegramBot;

pub const MAX_PAYLOAD_SIZE_BYTES: u64 = {
    let kb = 1000;
    let mb = kb * 1000;

    50 * mb
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileId(String);

impl FileId {
    pub fn from_message(message: &Message) -> Option<Self> {
        file_id_from_message(message)
    }

    #[tracing::instrument]
    pub async fn download(&self, download_dir: &Path) -> Result<PathBuf, String> {
        debug!("Downloading file from telegram");

        let file_id = self.0.as_str();

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

        trace!("Finished syncing file");

        trace!("Setting proper file extension");

        let final_file_path = FileExtension
            .run(&FixRequest::new(&download_file_path))
            .await
            .map(|x| x.file_path)
            .unwrap_or(download_file_path);

        debug!(path = ?final_file_path, "Downloaded file");

        Ok(final_file_path)
    }
}

impl std::fmt::Display for FileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<String> for FileId {
    fn from(x: String) -> Self {
        Self(x)
    }
}
impl TryFrom<&Message> for FileId {
    type Error = ();

    fn try_from(value: &Message) -> Result<Self, Self::Error> {
        file_id_from_message(value).ok_or(())
    }
}
impl TryFrom<Message> for FileId {
    type Error = ();

    fn try_from(value: Message) -> Result<Self, Self::Error> {
        (&value).try_into()
    }
}

#[tracing::instrument(skip_all, fields(msg = %msg.id))]
pub fn file_id_from_message(msg: &Message) -> Option<FileId> {
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
    .map(FileId)
}

#[tracing::instrument(skip_all)]
pub async fn files_to_input_media_groups<TFiles, TFile>(
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
        max_size_bytes: u64,
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

            if size > max_size_bytes {
                trace!(?path, ?size, ?max_size_bytes, "File is too large");
                {
                    failed.push((
                        path,
                        format!("file is too large: {} > {}", size, max_size_bytes),
                    ));
                }
                continue;
            }

            if size + res_size > MAX_PAYLOAD_SIZE_BYTES {
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
    let file_info = files
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
        })
        .collect::<FuturesUnordered<_>>()
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    trace!(?file_info, "Got file infos");

    trace!("Converting to media files");
    let media_files = file_info.into_iter().map(|file_info| {
        let input_file = InputFile::file(file_info.path.clone());

        // Handle the GIFs as animations because Telegram
        // Also handle PNGs as documents to prevent Telegram from converting them to jpgs
        // Optional todo: Also handle silent videos as animations
        if file_info
            .mime
            .as_ref()
            .is_some_and(|x| matches!(x.essence_str(), "image/gif" | "image/png"))
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
