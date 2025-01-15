use std::{
    ffi::OsStr,
    fmt::Display,
    path::{Path, PathBuf},
    process::Stdio,
};

use app_config::Config;
use app_helpers::{ffprobe, temp_dir::TempDir, trash::move_to_trash};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{
    fs,
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};
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
    fn can_run(&self) -> bool {
        Config::global()
            .dependency_paths
            .imagemagick_path()
            .is_some()
    }

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

    let final_crop_filter = match generate_crop_filter(file_path).await {
        Ok(mut f) => {
            f.intersect(&CropFilter {
                width: w,
                height: h,
                x: 0,
                y: 0,
            });
            f
        }
        Err(e) => {
            debug!("Failed to generate crop filter: {e:?}");
            return Err(e);
        }
    };

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

    trace!(?new_filename, "Using new filename for file");

    let mut cmd = Command::new(Config::global().dependency_paths.ffmpeg_path());
    let res = cmd
        .arg("-y")
        .args(["-loglevel", "panic"])
        .arg("-i")
        .arg(file_path)
        .args(["-vf", &final_crop_filter.to_string()])
        .args(["-map_metadata", "0", "-movflags", "use_metadata_tags"])
        .args(["-preset", "slow"])
        .arg(&new_filename)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .status()
        .await
        .map_err(|e| CropError::CommandError(CmdError::Run(e)))?;

    if !res.success() {
        return Err(CropError::CommandError(CmdError::FailedStatus(
            "Failed to extract crop data from video frames".into(),
            res,
        )));
    }

    if let Err(e) = move_to_trash(file_path) {
        warn!("Failed to move file {file_path:?} to trash: {e:?}");
    }

    Ok(new_filename)
}

async fn generate_crop_filter(file_path: &Path) -> Result<CropFilter, CropError> {
    debug!(?file_path, "Generating crop filter");

    let tmp_dir = match TempDir::in_tmp_with_prefix("downloader-hub.crop-video-bars.") {
        Ok(d) => d,
        Err(e) => return Err(CropError::TempDirError(e)),
    };
    trace!(?tmp_dir, "Created temp dir to write frames to");

    let mut cmd = Command::new(Config::global().dependency_paths.ffmpeg_path());
    let res = cmd
        .arg("-y")
        .arg("-i")
        .arg(file_path)
        .args(["-vf", "fps=1"])
        .arg(format!("{}/%0d.jpg", tmp_dir.path().to_string_lossy()))
        .kill_on_drop(true);
    debug!(cmd = ?res.as_std(), "Running command to split video into frames");
    let res = res
        .output()
        .await
        .map_err(|e| CropError::CommandError(CmdError::Run(e)))?;

    if !res.status.success() {
        return Err(CropError::CommandError(CmdError::Failed(
            "Failed to convert video to frames".into(),
            res.into(),
        )));
    }

    debug!("Generated frames successfully");

    let files_in_tmp_dir = {
        let tmp_path = tmp_dir.path();
        debug!(?tmp_path, "Listing frame files in temp dir");
        let mut dir_iter = fs::read_dir(tmp_path)
            .await
            .map_err(CropError::TempDirError)?;

        let mut entries = Vec::new();
        while let Ok(Some(entry)) = dir_iter.next_entry().await {
            let path_in_temp_dir = entry.path();

            let file_name = match path_in_temp_dir.file_name() {
                Some(x) => x,
                None => continue,
            };

            let path_in_dir = tmp_path.join(file_name);

            entries.push(path_in_dir);
        }
        trace!(?entries, "Found all frame files in directory");
        entries
    };

    let filter = generate_crop_filter_for_files(&files_in_tmp_dir, None).await?;
    debug!(?filter, "Got crop filter");

    Ok(filter)
}

async fn generate_crop_filter_for_files(
    files: &[PathBuf],
    initial_filter: Option<&CropFilter>,
) -> Result<CropFilter, CropError> {
    const TRIM_PASSES: u8 = 2;
    const FUZZ_PERCENTAGE: u8 = 15;
    const SHAVE_BORDER_PIXELS: u8 = 2;
    const MIN_WIDTH: i64 = 4;
    const MIN_HEIGHT: i64 = 4;
    let mut cmd = Command::new(
        Config::global()
            .dependency_paths
            .imagemagick_path()
            .expect("Imagemagick not found"),
    );
    let res = {
        let mut res = cmd.args(files);
        if let Some(filter) = initial_filter {
            res = res.arg("-crop").arg(filter.to_imagemagick_dimensions());
        } else {
            // Remove small outside border to encourage better trimmming
            res = res.args([
                "-shave".to_string(),
                format!("{px}x{px}", px = SHAVE_BORDER_PIXELS),
            ]);
        }
        for _ in 0..TRIM_PASSES {
            res = res
                .args(["-fuzz".to_string(), format!("{}%", FUZZ_PERCENTAGE)])
                .arg("-trim");
        }
        res.args(["-format", "%w:%h:%X:%Y\n"]).arg("info:-")
    };
    debug!(
        cmd = ?res.as_std(),
        "Running command to generate crop filters"
    );
    let mut res = res
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| CropError::CommandError(CmdError::Run(e)))?;

    let mut res_stdout_lines = {
        let stdout = res.stdout.take().expect("stdout should be available");
        let reader = BufReader::new(stdout);
        reader.lines()
    };

    let filter = {
        let parse_line = |line: &str| {
            let mut s = line.split(':');
            let mut next_s = || {
                s.next()
                    .and_then(|x| x.trim_start_matches('+').to_string().parse::<i64>().ok())
            };

            Some(CropFilter {
                width: next_s()?,
                height: next_s()?,
                x: next_s()?,
                y: next_s()?,
            })
        };

        trace!("Reading crop filter command output lines");
        let mut filter = CropFilter::new_min();
        while let Ok(Some(line)) = res_stdout_lines.next_line().await {
            let Some(line_filter) = parse_line(&line) else {
                trace!(?line, "Couldn't parse line, skipping");
                continue;
            };

            if line_filter.width < MIN_WIDTH || line_filter.height < MIN_HEIGHT {
                trace!(?line, ?line_filter, "Filter is too small, skipping");
                continue;
            }

            if line_filter.x < 0 || line_filter.y < 0 {
                trace!(?line, ?line_filter, "Filter has negative offset, skipping");
                continue;
            }

            trace!(?line, ?line_filter, "Parsed line to filter");
            filter.union(&line_filter);
        }

        debug!(?filter, "Generated filter from output");

        filter
    };

    let res = res
        .wait()
        .await
        .map_err(|e| CropError::CommandError(CmdError::Run(e)))?;

    if res.success() {
        Ok(filter)
    } else {
        Err(CropError::CommandError(CmdError::FailedStatus(
            format!("Command exited with non-zero exit code, {:?}", res),
            res,
        )))
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct CropFilter {
    width: i64,
    height: i64,
    x: i64,
    y: i64,
}

impl CropFilter {
    const fn new_min() -> Self {
        Self {
            width: 0,
            height: 0,
            x: i64::MAX,
            y: i64::MAX,
        }
    }

    fn union(&mut self, other: &Self) {
        self.width = self.width.max(other.width);
        self.height = self.height.max(other.height);
        self.x = self.x.min(other.x);
        self.y = self.y.min(other.y);
    }

    fn intersect(&mut self, other: &Self) {
        self.width = self.width.min(other.width);
        self.height = self.height.min(other.height);
        self.x = self.x.max(other.x);
        self.y = self.y.max(other.y);
    }

    fn to_imagemagick_dimensions(&self) -> String {
        let mut x = self.x.to_string();
        if self.x >= 0 {
            x = format!("+{}", x);
        }

        let mut y = self.y.to_string();
        if self.y >= 0 {
            y = format!("+{}", y);
        }

        format!("{}x{}{}{}", self.width, self.height, x, y)
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
    #[error("Failed to create temp dir")]
    TempDirError(std::io::Error),
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
