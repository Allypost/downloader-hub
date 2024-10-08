use std::path::Path;

use filetime::FileTime;
use tracing::trace;

pub fn transfer_file_times(path_from: &Path, path_to: &Path) -> Result<(), TransferTimesError> {
    transferable_file_times(path_from)?(path_to)
}

pub fn transferable_file_times(
    path_from: &Path,
) -> Result<impl Fn(&Path) -> Result<(), TransferTimesError>, TransferTimesError> {
    trace!("Getting file times of {path:?}", path = path_from);

    let old_meta = path_from.metadata().map_err(TransferTimesError::Metadata)?;

    Ok(move |path_to: &Path| {
        trace!("Setting file times of {new:?}", new = path_to);
        filetime::set_file_times(
            path_to,
            FileTime::from_last_access_time(&old_meta),
            FileTime::from_last_modification_time(&old_meta),
        )
        .map_err(TransferTimesError::SetTime)
    })
}

#[derive(Debug, thiserror::Error)]
pub enum TransferTimesError {
    #[error("Failed to get metadata: {0:?}")]
    Metadata(std::io::Error),
    #[error("Failed to set file times: {0:?}")]
    SetTime(std::io::Error),
}
