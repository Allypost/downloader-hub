use std::{env, fs, io, path::PathBuf};

use app_logger::debug;

pub fn move_to_trash(f: &PathBuf) -> Result<(), io::Error> {
    if env::var_os("MEME_DOWNLOADER_TRASH_DISABLED").is_some() {
        debug!("Deleting file {f:?}");
        return fs::remove_file(f);
    }

    debug!("Sending {f:?} into trash");

    trash::delete(f)
        .or_else(|e| {
            debug!("Failed to put {f:?} into trash: {e:?}");
            debug!("Deleting old file {f:?}");
            fs::remove_file(f)
        })
        .map_err(|e| {
            debug!("Failed to delete old file {f:?}: {e:?}");
            e
        })
}
