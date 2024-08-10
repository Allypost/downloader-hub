use std::path::{Path, PathBuf};

use validator::ValidationError;

fn is_file(path: &Path) -> Result<(), &'static str> {
    if !path.exists() {
        return Err("File does not exist");
    }

    if !path.is_file() {
        return Err("Path is not a valid file");
    }

    Ok(())
}

pub fn validate_is_files(paths: &Vec<PathBuf>) -> Result<(), ValidationError> {
    for path in paths {
        if let Err(e) = is_file(path) {
            return Err(ValidationError::new(e));
        }
    }

    Ok(())
}

pub fn validate_is_file(path: &Path) -> Result<(), ValidationError> {
    if let Err(e) = is_file(path) {
        return Err(ValidationError::new(e));
    }

    Ok(())
}

#[must_use]
pub fn value_parser_parse_valid_file() -> impl clap::builder::TypedValueParser {
    move |s: &str| {
        let path = Path::new(s);
        is_file(path)?;

        let path = path
            .to_path_buf()
            .canonicalize()
            .map_err(|_| "Failed to canonicalize path")?;

        Ok::<_, &str>(path)
    }
}
