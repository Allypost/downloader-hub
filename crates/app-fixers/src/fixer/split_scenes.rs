use std::{
    convert::Into,
    path::{Path, PathBuf},
    process::Command,
};

use app_config::Config;
use app_helpers::dirs::create_temp_dir;
use thiserror::Error;

use crate::{error::FixerError, Fixer, FixerOptions, FixerReturn, IntoFixerReturn};

#[derive(Debug)]
pub struct SplitScenes;
#[async_trait::async_trait]
impl Fixer for SplitScenes {
    fn name(&self) -> &'static str {
        "split-scenes"
    }

    fn description(&self) -> &'static str {
        "Detect scenes in a video and split them into separate files."
    }

    /// Options:
    /// - `output-dir`: The output directory. Defaults to a temporary directory.
    fn run(&self, file_path: &Path, options: &FixerOptions) -> FixerReturn {
        let path = match options.get("output-dir").map(PathBuf::from) {
            Some(x) => x,
            None => create_temp_dir().map_err(SplitScenesError::TempDirCreate)?,
        };

        do_split_video_into_scenes(file_path, &path).into_fixer_return()
    }

    fn can_run(&self, _file_path: &Path, _options: &FixerOptions) -> bool {
        Config::global()
            .dependency_paths
            .scenedetect_path()
            .is_some()
    }
}

fn do_split_video_into_scenes(file_path: &Path, output_dir: &Path) -> impl IntoFixerReturn {
    split_into_scenes(&SplitVideoConfig::new(output_dir, file_path)).into_fixer_return()
}

#[derive(Debug, Clone)]
pub struct SplitVideoConfig<'a> {
    pub download_dir: &'a Path,
    pub file_path: &'a Path,
    pub file_template: Option<&'a str>,
}

impl<'a> SplitVideoConfig<'a> {
    #[must_use]
    pub const fn new(download_dir: &'a Path, file_path: &'a Path) -> Self {
        Self {
            download_dir,
            file_path,
            file_template: None,
        }
    }

    #[must_use]
    pub fn with_file_template(&mut self, file_template: &'a str) -> &Self {
        self.file_template = Some(file_template);
        self
    }
}

pub fn split_into_scenes(config: &SplitVideoConfig) -> impl IntoFixerReturn {
    let scenedetect_path = match Config::global().dependency_paths.scenedetect_path() {
        Some(x) => x,
        None => return Err(SplitScenesError::ScenedetectNotFound),
    };

    let mut cmd = Command::new(scenedetect_path);
    let cmd = cmd
        .args(["--input", config.file_path.to_str().unwrap_or_default()])
        .arg("detect-adaptive")
        .arg("split-video")
        .arg("--high-quality")
        .args(["--preset", "medium"])
        .args(["--output", config.download_dir.to_str().unwrap_or_default()])
        .args([
            "--filename",
            config.file_template.unwrap_or("$VIDEO_NAME.$SCENE_NUMBER"),
        ]);

    let output = cmd.output().map_err(SplitScenesError::ScenedetectRun)?;

    if !output.status.success() {
        return Err(SplitScenesError::ScenedetectExited(output.status.code()));
    }

    let scenes = std::fs::read_dir(config.download_dir)
        .map_err(SplitScenesError::TempDirRead)?
        .filter_map(std::result::Result::ok)
        .map(|x| x.path())
        .collect::<Vec<_>>();

    Ok(scenes)
}

#[derive(Debug, Error)]
pub enum SplitScenesError {
    #[error("Error while creating temp dir: {0:?}")]
    TempDirCreate(anyhow::Error),
    #[error("Error while reading temp dir: {0:?}")]
    TempDirRead(std::io::Error),
    #[error("Scenedetect not found")]
    ScenedetectNotFound,
    #[error("Error while running scenedetect: {0}")]
    ScenedetectRun(std::io::Error),
    #[error("Scenedetect exited with error code {0:?}")]
    ScenedetectExited(Option<i32>),
}

impl From<SplitScenesError> for FixerError {
    fn from(val: SplitScenesError) -> Self {
        Self::FailedFix(val.into())
    }
}
