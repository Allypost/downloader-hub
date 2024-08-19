use app_config::Config;
use app_helpers::file_type;
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tracing::warn;

use crate::fixers::{FixRequest, FixResult, Fixer, FixerError, FixerReturn};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CropImage;

#[async_trait::async_trait]
#[typetag::serde]
impl Fixer for CropImage {
    fn can_run(&self) -> bool {
        Config::global()
            .dependency_paths
            .imagemagick_path()
            .is_some()
    }

    fn description(&self) -> &'static str {
        "Crops dead space around the image. Any solid color surrounding the image will be cropped \
         quite aggressively."
    }

    fn enabled_by_default(&self) -> bool {
        false
    }

    async fn can_run_for(&self, request: &FixRequest) -> bool {
        let path = request.file_path.clone();
        tokio::task::spawn_blocking(move || file_type::infer_file_type(&path).ok())
            .await
            .ok()
            .flatten()
            .is_some_and(|x| x.type_() == file_type::mime::IMAGE)
    }

    async fn run(&self, request: &FixRequest) -> FixerReturn {
        let output_file_path = {
            let path = request.file_path.clone();
            let mut file_name = path.file_stem().unwrap_or_default().to_os_string();
            file_name.push(".ac");
            let file_extension = path.extension().unwrap_or_default().to_os_string();
            file_name.push(".");
            file_name.push(file_extension);

            path.with_file_name(file_name)
        };

        let mut cmd = Command::new(
            Config::global()
                .dependency_paths
                .imagemagick_path()
                .expect("Imagemagick not found"),
        );
        cmd.arg(&request.file_path);
        cmd.args(["-fuzz", "15%"]);
        cmd.arg("-trim");
        cmd.arg("+repage");
        cmd.arg(&output_file_path);

        let output = cmd
            .output()
            .await
            .map_err(|e| FixerError::CommandError(e.into()))?;

        if !output.status.success() {
            return Err(FixerError::CommandError(
                anyhow::anyhow!(format!(
                    "Imagemagick returned an error: {}",
                    String::from_utf8_lossy(&output.stderr)
                ))
                .into(),
            ));
        }

        if !output_file_path.exists() {
            warn!(
                "Imagemagick did not create the output file {path:?}",
                path = output_file_path
            );
            return Err(FixerError::FailedFix(
                anyhow::anyhow!(format!(
                    "Imagemagick finished but the output file {path:?} does not exist",
                    path = output_file_path,
                ))
                .into(),
            ));
        }

        Ok(FixResult::new(request.clone(), output_file_path))
    }
}
