use std::{
    ffi::OsStr,
    fmt::Display,
    path::{Path, PathBuf},
};

use app_config::Config;
use app_helpers::{ffprobe, file_time::transfer_file_times, trash::move_to_trash};
use futures::{stream::FuturesUnordered, StreamExt};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::process::Command;
use tracing::{debug, trace, warn};

use crate::fixers::{
    common::{command::CmdError, FixRequest, FixResult, FixerError},
    Fixer, FixerReturn, IntoFixerReturn,
};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CropVideoBars;

#[async_trait::async_trait]
#[typetag::serde]
impl Fixer for CropVideoBars {
    async fn can_run_for(&self, request: &FixRequest) -> bool {
        get_video_stream(&request.file_path)
            .await
            .ok()
            .flatten()
            .is_some()
    }

    fn description(&self) -> &'static str {
        "Crops dead space around the video. Supports both black and white outlines."
    }

    /// Options:
    ///
    async fn run(&self, request: &FixRequest) -> FixerReturn {
        do_auto_crop_video(&request.file_path)
            .await
            .map(|x| FixResult::new(request.clone(), x))
            .into_fixer_return()
    }
}

async fn do_auto_crop_video(file_path: &Path) -> Result<PathBuf, CropError> {
    debug!("Auto cropping video {file_path:?}");

    let file_path_str = file_path.to_str().ok_or_else(|| {
        CropError::InvalidPath(
            file_path.to_path_buf(),
            "Failed to convert path to string".to_string(),
        )
    })?;

    let video_stream = get_video_stream(file_path).await?;

    let (w, h) = {
        let video_stream = if let Some(s) = video_stream {
            trace!("Found video stream");
            s
        } else {
            debug!("File does not contain a video stream, skipping");
            return Ok(file_path.into());
        };

        if let (Some(w), Some(h)) = (video_stream.width, video_stream.height) {
            trace!("Video width: {w}, height: {h}");
            (w, h)
        } else {
            return Err(CropError::NoDimensions(file_path.to_path_buf()));
        }
    };

    let crop_filters = {
        let crop_filters = vec![BorderColor::White, BorderColor::Black]
            .into_iter()
            .map(|color| async move { get_crop_filter(file_path_str, &color).await.ok() })
            .collect::<FuturesUnordered<_>>()
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .flatten()
            .collect::<Option<Vec<_>>>();

        if let Some(fs) = crop_filters {
            trace!("Crop filters: {fs:?}");
            fs
        } else {
            trace!("No crop filters found, skipping");
            return Ok(file_path.into());
        }
    };

    let final_crop_filter = CropFilter::intersect_all(crop_filters)
        .ok_or_else(|| CropError::NoFilters(file_path.to_path_buf()))?;

    debug!("Final crop filter: {final_crop_filter:?}");

    if final_crop_filter.width >= w && final_crop_filter.height >= h {
        debug!("Video is already cropped, skipping");
        return Ok(file_path.into());
    }

    let new_filename = {
        let file_name = file_path
            .file_stem()
            .and_then(OsStr::to_str)
            .ok_or_else(|| CropError::NoFilePart("stem".to_string(), file_path.to_path_buf()))?;

        let file_extension = file_path
            .extension()
            .and_then(OsStr::to_str)
            .ok_or_else(|| {
                CropError::NoFilePart("extension".to_string(), file_path.to_path_buf())
            })?;

        file_path.with_file_name(format!("{file_name}.ac.{file_extension}"))
    };

    let mut cmd = Command::new(Config::global().dependency_paths.ffmpeg_path());
    let res = cmd
        .arg("-y")
        .args(["-loglevel", "panic"])
        .args(["-i", file_path_str])
        .args(["-vf", &final_crop_filter.to_string()])
        .args(["-map_metadata", "0", "-movflags", "use_metadata_tags"])
        .args(["-preset", "slow"])
        .arg(&new_filename)
        .output()
        .await
        .map_err(|e| CropError::CommandError(CmdError::Run(e)))?;

    if !res.status.success() {
        return Err(CropError::CommandError(CmdError::Failed(
            format!("Command exited with non-zero exit code, {:?}", res.status),
            res.into(),
        )));
    }

    if let Err(e) = transfer_file_times(file_path, &new_filename) {
        warn!("Failed to transfer file times of {file_path:?}: {e:?}");
    }

    if let Err(e) = move_to_trash(file_path) {
        warn!("Failed to move file {file_path:?} to trash: {e:?}");
    }

    Ok(new_filename)
}

async fn get_video_stream(file_path: &Path) -> Result<Option<ffprobe::Stream>, CropError> {
    let media_info = ffprobe::ffprobe_async(file_path)
        .await
        .map_err(CropError::FfProbeError)?;

    let video_stream = media_info
        .streams
        .iter()
        .find(|s| s.codec_type.as_ref().is_some_and(|x| x == "video"));

    Ok(video_stream.cloned())
}

#[derive(Debug, Clone)]
struct CropFilter {
    width: i64,
    height: i64,
    x: i64,
    y: i64,
}

impl CropFilter {
    fn union(&mut self, other: &Self) {
        self.width = self.width.max(other.width);
        self.height = self.height.max(other.height);
        self.x = self.x.min(other.x);
        self.y = self.y.min(other.y);
    }

    fn union_all(filters: Vec<Self>) -> Option<Self> {
        if filters.is_empty() {
            return None;
        }

        let mut res = Self {
            width: i64::MIN,
            height: i64::MIN,
            x: i64::MAX,
            y: i64::MAX,
        };

        for filter in filters {
            res.union(&filter);
        }

        Some(res)
    }

    fn intersect(&mut self, other: &Self) {
        self.width = self.width.min(other.width);
        self.height = self.height.min(other.height);
        self.x = self.x.max(other.x);
        self.y = self.y.max(other.y);
    }

    fn intersect_all(filters: Vec<Self>) -> Option<Self> {
        if filters.is_empty() {
            return None;
        }

        let mut res = Self {
            width: i64::MAX,
            height: i64::MAX,
            x: i64::MIN,
            y: i64::MIN,
        };

        for filter in filters {
            res.intersect(&filter);
        }

        Some(res)
    }
}

impl Display for CropFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "crop={width}:{height}:{x}:{y}",
            width = self.width,
            height = self.height,
            x = self.x,
            y = self.y
        )
    }
}

#[derive(Debug, Clone)]
enum BorderColor {
    White,
    Black,
}

async fn get_crop_filter(
    file_path: &str,
    border_color: &BorderColor,
) -> Result<Option<CropFilter>, String> {
    let cropdetect_filter = {
        let mut filters = vec![];

        // filters.push("eq=contrast=3.0");

        match border_color {
            BorderColor::White => {
                filters.push("negate");
            }
            BorderColor::Black => {}
        };

        // skip=0
        filters.push("cropdetect=mode=black:limit=24:round=2:reset=0");

        filters.join(",")
    };

    let mut cmd = Command::new(Config::global().dependency_paths.ffmpeg_path());
    let cmd = cmd
        .arg("-hide_banner")
        .args(["-i", file_path])
        .args(["-vf", cropdetect_filter.as_str()])
        .args(["-f", "null", "-"]);
    trace!("Running command {cmd:?}");

    let cmd_output = cmd
        .output()
        .await
        .map_err(|e| format!("Failed to run command {cmd:?}: {e:?}"))?;
    trace!("Command output: {:?}", &cmd_output);
    let stderr = String::from_utf8(cmd_output.stderr)
        .map_err(|e| format!("Failed to convert command output to UTF-8: {e:?}"))?;

    let mut res = stderr
        .split('\n')
        .filter(|s| s.starts_with("[Parsed_cropdetect") && s.contains("crop="))
        .map(str::trim)
        .map(|s| {
            s.split("crop=")
                .nth(1)
                .ok_or_else(|| format!("Failed to parse cropdetect output from {s:?}"))
        })
        .collect::<Result<Vec<_>, _>>()?;

    res.sort_unstable();
    res.dedup();

    let res = res
        .iter()
        .map(|s| {
            let mut s = s.split(':');
            let mut next_s = || {
                s.next()
                    .and_then(|x| x.to_string().parse::<i64>().ok())
                    .ok_or_else(|| format!("Failed to parse width from {s:?}"))
            };

            Ok(CropFilter {
                width: next_s()?,
                height: next_s()?,
                x: next_s()?,
                y: next_s()?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(CropFilter::union_all(res))
}

#[derive(Debug, Error)]
pub enum CropError {
    #[error("Invalid path: {0:?}")]
    InvalidPath(PathBuf, String),
    #[error(transparent)]
    FfProbeError(#[from] ffprobe::FfProbeError),
    #[error(transparent)]
    CommandError(#[from] CmdError),
    #[error("Failed to get width and height of media")]
    NoDimensions(PathBuf),
    #[error("Failed to get file {0} from {1:?}")]
    NoFilePart(String, PathBuf),
    #[error("No filters found for {0:?}")]
    NoFilters(PathBuf),
}

impl From<CropError> for FixerError {
    fn from(val: CropError) -> Self {
        Self::FailedFix(val.into())
    }
}

impl IntoFixerReturn for CropError {
    fn into_fixer_return(self) -> FixerReturn {
        Err(FixerError::FailedFix(self.into()))
    }
}
