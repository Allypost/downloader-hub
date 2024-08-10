use std::path::Path;

use validator::ValidationError;

fn is_directory(path: &Path) -> Result<(), &'static str> {
    if !path.exists() {
        return Err("Directory does not exist");
    }

    if !path.is_dir() {
        return Err("Path is not a directory");
    }

    Ok(())
}

pub fn validate_is_directory(path: &Path) -> Result<(), ValidationError> {
    if let Err(e) = is_directory(path) {
        return Err(ValidationError::new(e));
    }

    Ok(())
}

pub fn validate_is_writable_directory(path: &Path) -> Result<(), ValidationError> {
    validate_is_directory(path)?;

    let Ok(metadata) = path.metadata() else {
        return Err(ValidationError::new("Failed to get metadata"));
    };

    if metadata.permissions().readonly() {
        return Err(ValidationError::new("Directory is read-only"));
    }

    Ok(())
}

#[must_use]
pub fn value_parser_parse_valid_directory() -> impl clap::builder::TypedValueParser {
    move |s: &str| {
        let path = Path::new(s);

        is_directory(path)?;

        let path = path
            .to_path_buf()
            .canonicalize()
            .map_err(|_| "Failed to canonicalize path")?;

        Ok::<_, &str>(path)
    }
}
