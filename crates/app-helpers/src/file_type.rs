use std::{path::Path, str::FromStr};

use file_format::FileFormat;
use infer::get_from_path as infer_from_path;
use mime::Mime;
use tree_magic_mini::from_filepath as magic_infer_from_filepath;

pub fn infer_file_type(file: &Path) -> anyhow::Result<Mime> {
    let file = file.to_path_buf();
    let mime_type = infer_from_path(&file)?
        .map(|x| x.mime_type().to_string())
        .or_else(|| {
            FileFormat::from_file(&file)
                .map(|x| x.media_type().to_string())
                .ok()
        })
        .or_else(|| magic_infer_from_filepath(&file).map(ToString::to_string))
        .ok_or_else(|| anyhow::anyhow!("Could not infer file type for file: {:?}", &file))?;

    Mime::from_str(&mime_type)
        .map_err(|e| anyhow::anyhow!("Failed to parse mime type: {:?}, error: {:?}", mime_type, e))
}
