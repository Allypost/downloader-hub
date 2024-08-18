use std::convert::Into;

use app_helpers::file_time::transferable_file_times;
pub use common::{FixRequest, FixResult, FixerError, FixerReturn};
use handlers::FixerInstance;
pub use handlers::AVAILABLE_FIXERS;
use tracing::{debug, trace, warn};

mod common;
pub mod handlers;

#[async_trait::async_trait]
pub trait Fixer: std::fmt::Debug + Send + Sync {
    fn name(&self) -> &'static str;

    fn description(&self) -> &'static str;

    fn can_run(&self) -> bool {
        true
    }

    #[allow(unused_variables)]
    fn can_run_for(&self, request: &FixRequest) -> bool {
        true
    }

    async fn run(&self, request: &FixRequest) -> FixerReturn;
}

pub trait IntoFixerReturn {
    fn into_fixer_return(self) -> FixerReturn;
}
impl<T, E> IntoFixerReturn for Result<T, E>
where
    T: Into<FixResult>,
    E: Into<FixerError>,
{
    fn into_fixer_return(self) -> FixerReturn {
        self.map(Into::into).map_err(Into::into)
    }
}

pub async fn fix_file(request: FixRequest) -> FixerReturn {
    fix_file_with(AVAILABLE_FIXERS.clone(), request).await
}

#[tracing::instrument(skip(fixers))]
pub async fn fix_file_with(fixers: Vec<FixerInstance>, request: FixRequest) -> FixerReturn {
    let request = request.resolve_path()?.check_path()?;
    debug!(?request, "Fixing file");

    let transfer_file_times = transferable_file_times(&request.file_path);

    let mut req = request.clone();
    for fixer in fixers {
        trace!(?fixer, "Trying fixer");

        if !fixer.can_run_for(&req) {
            continue;
        }

        trace!("Running fixer {fixer:?} on {req:?}");

        let result = match fixer.run(&req).await {
            Ok(x) => x,
            Err(e) => {
                warn!("Failed to run fixer {fixer:?} on {req:?}: {e:?}");
                continue;
            }
        };

        trace!(?result, "Fixer result");

        req = req.clone_with_path(result.file_path);
    }

    if let Ok(transfer_file_times) = transfer_file_times {
        if req.file_path.as_os_str() != request.file_path.as_os_str() {
            if let Err(e) = transfer_file_times(&req.file_path) {
                warn!("Failed to transfer file times of {request:?} to {req:?}: {e:?}");
            }
        }
    }

    debug!(?req, "Fixed file");

    Ok(FixResult::new(request.clone(), req.file_path))
}
