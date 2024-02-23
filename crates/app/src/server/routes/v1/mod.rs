use axum::{
    extract::OriginalUri,
    http::{Method, StatusCode},
    routing::any,
};

use crate::server::AppRouter;

mod handlers;
mod middleware;
pub mod response;

pub(super) fn router() -> AppRouter {
    handlers::router().route("/*path", any(handle_404))
}

async fn handle_404(method: Method, OriginalUri(uri): OriginalUri) -> response::V1Error {
    response::V1Response::error(
        StatusCode::NOT_FOUND,
        format!("Unknown route: [{method:?}] {uri:?}"),
    )
}
