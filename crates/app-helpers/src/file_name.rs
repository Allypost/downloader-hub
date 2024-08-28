use std::path::{Path, PathBuf};

#[must_use]
pub fn file_name_with_suffix(file_path: &Path, suffix: &str) -> PathBuf {
    let file_stem = match file_path.file_stem() {
        Some(x) => x,
        None => return file_path.to_path_buf(),
    };

    let mut file_name = file_stem.to_os_string();
    file_name.push(".");
    file_name.push(suffix);

    if let Some(ext) = file_path.extension() {
        file_name.push(".");
        file_name.push(ext);
    }

    file_path.with_file_name(file_name)
}
