use std::path::Path;

use app_helpers::encoding::to_base64;
use file_format::FileFormat;
use infer::get_from_path as infer_from_path;
use sha2::Digest;
use tree_magic_mini::from_filepath as magic_infer_from_filepath;

pub struct FileService;

impl FileService {
    pub async fn file_hash(file: &Path) -> anyhow::Result<String> {
        let f = file.to_path_buf();

        tokio::task::spawn_blocking(|| -> anyhow::Result<String> {
            let input = std::fs::File::open(f)?;
            let mut reader = std::io::BufReader::new(input);
            let mut hasher = sha2::Sha384::new();

            std::io::copy(&mut reader, &mut hasher)
                .map_err(|e| anyhow::anyhow!("Failed to hash file: {}", e))?;

            Ok(format!(
                "sha384:{digest}",
                digest = to_base64(hasher.finalize())
            ))
        })
        .await?
    }

    pub async fn infer_file_type(file: &Path) -> anyhow::Result<String> {
        let file = file.to_path_buf();
        tokio::task::spawn_blocking(move || {
            infer_from_path(&file)?
                .map(|x| x.mime_type().to_string())
                .or_else(|| {
                    FileFormat::from_file(&file)
                        .map(|x| x.media_type().to_string())
                        .ok()
                })
                .or_else(|| magic_infer_from_filepath(&file).map(ToString::to_string))
                .ok_or_else(|| anyhow::anyhow!("Could not infer file type for file: {:?}", &file))
        })
        .await?
    }
}
