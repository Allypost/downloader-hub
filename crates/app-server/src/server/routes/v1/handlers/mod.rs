use axum::Router;

use crate::server::AppRouter;

mod admin;
mod clients;
mod download;

pub(super) fn router() -> AppRouter {
    Router::new()
        .nest("/clients", clients::router())
        .nest("/download", download::router())
        .nest("/admin", admin::router())
}
