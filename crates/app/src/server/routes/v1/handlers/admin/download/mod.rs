use axum::Router;

use crate::server::AppRouter;

mod download_request;

pub(super) fn router() -> AppRouter {
    Router::new().nest("/requests", download_request::router())
}
