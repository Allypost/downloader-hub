use std::{
    path::{Path, PathBuf},
    process::Command,
};

use app_config::Config;
use app_helpers::dirs::create_temp_dir;

pub fn split_video_into_scenes(file_path: &Path) -> Result<Vec<PathBuf>, String> {
    let tmp_dir = create_temp_dir().map_err(|e| format!("Error while getting temp dir: {e:?}"))?;

    split_into_scenes(&SplitVideoConfig::new(&tmp_dir, file_path))
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

pub fn split_into_scenes(config: &SplitVideoConfig) -> Result<Vec<PathBuf>, String> {
    let Some(scenedetect_path) = &Config::global().dependency_paths.scenedetect_path() else {
        return Err("scenedetect not found".into());
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
            config
                .file_template
                .unwrap_or("$SCENE_NUMBER.$START_FRAME-$END_FRAME"),
        ]);

    let output = cmd
        .output()
        .map_err(|e| format!("Error while running scenedetect: {e:?}"))?;

    if !output.status.success() {
        return Err(format!(
            "Scenedetect exited with code: {code:?}",
            code = output.status.code()
        ));
    }

    let scenes = std::fs::read_dir(config.download_dir)
        .map_err(|x| format!("Error while reading temp dir: {x:?}"))?
        .filter_map(std::result::Result::ok)
        .map(|x| x.path())
        .collect::<Vec<_>>();

    Ok(scenes)
}
