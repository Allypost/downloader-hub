use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process,
    str::FromStr,
};

use anyhow::anyhow;
use app_config::Config;
use app_helpers::{
    ffprobe::{self, FfProbeResult, Stream},
    id::time_thread_id,
    trash::move_to_trash,
};
use app_logger::{debug, error, trace};
use image::ColorType;
use scopeguard::defer;
use thiserror::Error;

use crate::{error::FixerError, util::transferable_file_times, FixerReturn, IntoFixerReturn};

pub fn convert_into_preferred_formats(file_path: &PathBuf) -> FixerReturn {
    debug!("Checking if {file_path:?} has unwanted formats");

    Ok(file_path)
        .and_then(check_and_fix_file)
        .map(|p| {
            debug!("File {file_path:?} done being converted");
            p
        })
        .map_err(FixerError::failed_fix)
}

fn check_and_fix_file(file_path: &PathBuf) -> Result<PathBuf, MediaFormatsError> {
    if !file_path.exists() {
        return Err(MediaFormatsError::NotFound(file_path.clone()));
    }

    let file_format_info = ffprobe::ffprobe(file_path)?;

    trace!(
        "File format info: {file_format_info:?}",
        file_format_info = file_format_info
    );

    let file_image_stream = file_format_info
        .streams
        .iter()
        .find(|s| {
            s.codec_type
                .as_deref()
                .is_some_and(|codec| matches!(codec, "video" | "image"))
        })
        .ok_or_else(|| MediaFormatsError::NoImageStream(file_path.clone()))?;

    let file_stream_codec = file_image_stream
        .codec_name
        .as_deref()
        .ok_or_else(|| MediaFormatsError::CodecName(file_path.clone()))?;

    trace!(
        "File stream codec: {file_stream_codec:?}",
        file_stream_codec = file_stream_codec
    );

    let handler = CODEC_HANDLERS
        .iter()
        .find(|h| (h.can_handle)(file_stream_codec));

    if let Some(handler) = handler {
        trace!("Using handler: {handler:?}", handler = handler);
        return (handler.handle)(&file_format_info, file_image_stream)
            .map_err(MediaFormatsError::CodecFix);
    }

    error!("File {path:?} has unknown codec", path = file_path);

    Err(MediaFormatsError::UnknownCodec(
        file_stream_codec.to_string(),
    ))
}

#[derive(Debug, Clone, PartialEq, Default)]
struct TranscodeInfo {
    extension: &'static str,
    video_codec: &'static str,
    audio_codec: Option<&'static str>,
    additional_args: Vec<&'static str>,
}

impl TranscodeInfo {
    fn new(extension: &'static str, video_codec: &'static str) -> Self {
        Self {
            extension,
            video_codec,
            ..Default::default()
        }
    }

    const fn with_audio_codec(mut self, audio_codec: &'static str) -> Self {
        self.audio_codec = Some(audio_codec);
        self
    }

    // Default codecs

    fn mp4() -> Self {
        Self::new("mp4", "libx264").with_audio_codec("aac")
    }

    fn jpg() -> Self {
        Self::new("jpg", "mjpeg")
    }

    fn png() -> Self {
        Self::new("png", "png")
    }
}

fn transcode_media_into(from_path: &PathBuf, to_format: &TranscodeInfo) -> anyhow::Result<PathBuf> {
    let to_extension = to_format.extension;

    let (cache_folder, cache_from_path) = copy_file_to_cache_folder(from_path)?;
    defer! {
        trace!("Deleting {path:?}", path = cache_folder);
        if let Err(e) = fs::remove_dir_all(&cache_folder) {
            debug!("Failed to delete {cache_folder:?}: {e:?}");
        }
    }

    let cache_to_path = {
        let path = cache_from_path.with_extension(to_extension);

        if path_has_extension(&path, to_extension) {
            let new_file_name =
                file_name_with_suffix_extension(&path, "transcoded").ok_or_else(|| {
                    anyhow!(
                        "Failed to get file name with suffix extension of {path:?}",
                        path = path,
                    )
                })?;
            path.with_file_name(new_file_name)
        } else {
            path
        }
    };

    debug!(
        "Converting {from:?} to {to:?}",
        from = cache_from_path.file_name(),
        to = cache_to_path.file_name(),
    );

    let ffmpeg_path = Config::global().dependency_paths.ffmpeg_path().clone();
    trace!("`ffmpeg' binary: {ffmpeg_path:?}");
    let mut cmd = process::Command::new(ffmpeg_path);
    let mut cmd = cmd
        .arg("-y")
        .arg("-hide_banner")
        .args(["-loglevel", "panic"])
        .args([
            OsString::from_str("-i").unwrap_or_default(),
            cache_from_path.into_os_string(),
        ])
        .args(["-max_muxing_queue_size", "1024"])
        .args(["-vf", "scale=ceil(iw/2)*2:ceil(ih/2)*2"])
        .args(["-ab", "320k"])
        .args(["-map_metadata", "-1"])
        .args(["-preset", "slow"])
        .args(["-c:v", to_format.video_codec]);

    if let Some(audio_codec) = to_format.audio_codec {
        cmd = cmd.args(["-c:a", audio_codec]);
    }

    let cmd = cmd.arg(&cache_to_path);
    debug!("Running `ffmpeg' command: {cmd:?}");

    let cmd_output = cmd.output();
    match cmd_output {
        Ok(process::Output { status, .. }) if status.success() && cache_to_path.exists() => {
            debug!(
                "Converted file {from:?} to {to}",
                from = from_path,
                to = to_extension
            );

            let transfer_file_times = transferable_file_times(from_path.into());

            let new_file_path = from_path.with_extension(to_extension);

            trace!(
                "Copying {cache_path:?} to {new_path:?}",
                cache_path = cache_to_path,
                new_path = new_file_path
            );
            if let Err(e) = fs::copy(&cache_to_path, &new_file_path) {
                return Err(anyhow!(
                    "Failed to copy {from:?} to {to:?}: {e:?}",
                    from = cache_to_path,
                    to = new_file_path,
                ));
            }

            if &new_file_path != from_path {
                trace!("Deleting old file {path:?}", path = from_path);
                if let Err(e) = move_to_trash(from_path) {
                    debug!("Failed to delete {path:?}: {e:?}", path = from_path);
                }
            }

            match transfer_file_times {
                Ok(transfer_file_times_to) => {
                    if let Err(e) = transfer_file_times_to(&new_file_path) {
                        debug!("Failed to transfer file times: {e:?}");
                    }
                }
                Err(e) => {
                    debug!("Failed to transfer file times: {e:?}");
                }
            }

            Ok(new_file_path)
        }
        _ => Err(anyhow!(
            "Failed transforming {from_path:?} into {to_extension}"
        )),
    }
}

fn copy_file_to_cache_folder(file_path: &Path) -> anyhow::Result<(PathBuf, PathBuf)> {
    let id = time_thread_id();

    let cache_folder = Config::global()
        .get_cache_dir()
        .join(format!("transcode-{}", id));

    if !cache_folder.exists() {
        trace!("Creating {path:?}", path = cache_folder);
        fs::create_dir_all(&cache_folder)
            .map_err(|e| anyhow!("Failed to create {path:?}: {e:?}", path = cache_folder))?;
    }
    trace!("Using {path:?} as cache folder", path = cache_folder);

    let cache_file_path = {
        let filename = file_path
            .file_name()
            .ok_or_else(|| anyhow!("Failed to get file name of {path:?}", path = file_path))?;

        cache_folder.join(filename)
    };

    trace!(
        "Copying {from:?} to {to:?}",
        from = file_path,
        to = cache_file_path,
    );
    fs::copy(file_path, &cache_file_path).map_err(|e| {
        anyhow!(
            "Failed to copy {from:?} to {to:?}: {e:?}",
            from = file_path,
            to = cache_file_path,
        )
    })?;

    Ok((cache_folder, cache_file_path))
}

fn path_has_extension(path: &Path, wanted_extension: &str) -> bool {
    path.extension()
        .and_then(std::ffi::OsStr::to_str)
        .map_or(false, |extension| extension == wanted_extension)
}

fn file_name_with_suffix_extension(path: &Path, suffix: &str) -> Option<PathBuf> {
    path.extension()
        .and_then(std::ffi::OsStr::to_str)
        .map(|ext| format!("{suffix}.{ext}", ext = ext, suffix = suffix))
        .map(|ext| path.with_extension(ext))
        .and_then(|p| p.file_name().map(PathBuf::from))
}

fn get_stream_of_type<'a>(
    file_format_info: &'a FfProbeResult,
    stream_type: &'a str,
) -> Option<&'a Stream> {
    file_format_info
        .streams
        .iter()
        .find(|s| s.codec_type.as_deref().is_some_and(|x| x == stream_type))
}

#[derive(Debug, Clone, PartialEq)]
struct CodecHandler {
    pub can_handle: fn(&str) -> bool,
    pub handle: fn(&FfProbeResult, &Stream) -> anyhow::Result<PathBuf>,
}

const CODEC_HANDLERS: &[CodecHandler] = &[
    CodecHandler {
        can_handle: |codec| matches!(codec, "h264"),
        handle: |file_format_info, video_stream| {
            let file_path = PathBuf::from(file_format_info.format.filename.clone());

            let video_codec_ok = video_stream
                .codec_name
                .as_ref()
                .map_or(false, |vcodec| vcodec == "h264");

            let audio_codec_ok =
                get_stream_of_type(file_format_info, "audio").map_or(true, |audio_stream| {
                    audio_stream
                        .codec_name
                        .as_ref()
                        .map_or(false, |acodec| acodec == "aac")
                });

            let extension_ok = path_has_extension(&file_path, "mp4");

            trace!(
                "Video codec ok: {video_codec_ok:?} | Audio codec ok: {audio_codec_ok:?} | \
                 Extension ok: {extension_ok:?}",
                video_codec_ok = video_codec_ok,
                audio_codec_ok = audio_codec_ok,
                extension_ok = extension_ok,
            );

            if video_codec_ok && audio_codec_ok && extension_ok {
                trace!(
                    "File {path:?} is already in preferred format",
                    path = file_path
                );

                return Ok(file_path);
            }

            transcode_media_into(&file_path, &TranscodeInfo::mp4())
        },
    },
    CodecHandler {
        can_handle: |codec| matches!(codec, "mpeg4" | "vp8" | "vp9" | "av1" | "hevc"),
        handle: |file_format_info, _matched_stream| {
            let from_path = PathBuf::from(file_format_info.format.filename.clone());
            trace!("Converting {path:?} into mp4", path = from_path);
            transcode_media_into(&from_path, &TranscodeInfo::mp4())
        },
    },
    CodecHandler {
        can_handle: |codec| matches!(codec, "png" | "mjpeg"),
        handle: |file_format_info, _matched_stream| {
            let from_path = PathBuf::from(file_format_info.format.filename.clone());

            trace!(
                "File {path:?} is already in preferred format",
                path = from_path
            );

            Ok(from_path)
        },
    },
    CodecHandler {
        can_handle: |codec| matches!(codec, "webp"),
        handle: |file_format_info, _matched_stream| {
            let from_path = PathBuf::from(file_format_info.format.filename.clone());
            let img = image::open(&from_path)?;
            let color = img.color();

            match color {
                ColorType::Rgb8 | ColorType::Rgb16 | ColorType::Rgb32F => {
                    trace!("Converting {path:?} into jpg", path = from_path);
                    transcode_media_into(&from_path, &TranscodeInfo::jpg())
                }
                ColorType::Rgba8 | ColorType::Rgba16 | ColorType::Rgba32F => {
                    trace!("Converting {path:?} into png", path = from_path);
                    transcode_media_into(&from_path, &TranscodeInfo::png())
                }

                color_type => {
                    error!(
                        "File {path:?} has unknown color type {color_type:?}",
                        path = from_path,
                        color_type = color_type,
                    );

                    Err(anyhow!(
                        "File has an unknown color type ({color_type:?}), please report this \
                         issue to the developers.",
                        color_type = color_type,
                    ))
                }
            }
        },
    },
];

#[derive(Debug, Error)]
pub enum MediaFormatsError {
    #[error("File not found: {0:?}")]
    NotFound(PathBuf),
    #[error(transparent)]
    FfProbeError(#[from] ffprobe::FfProbeError),
    #[error("Failed to get image stream of {0:?}")]
    NoImageStream(PathBuf),
    #[error("Failed to get codec of {0:?}")]
    CodecName(PathBuf),
    #[error(transparent)]
    CodecFix(anyhow::Error),
    #[error("File has an unknown codec ({0:?}), please report this issue to the developers.")]
    UnknownCodec(String),
}

impl IntoFixerReturn for MediaFormatsError {
    fn into_fixer_return(self) -> FixerReturn {
        Err(FixerError::failed_fix(self))
    }
}
