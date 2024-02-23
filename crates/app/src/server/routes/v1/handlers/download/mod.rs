use axum::Router;

use crate::server::AppRouter;

mod download_request;
mod download_result;

pub(super) fn router() -> AppRouter {
    Router::new()
        .nest("/requests", download_request::router())
        .nest("/results", download_result::router())
}
