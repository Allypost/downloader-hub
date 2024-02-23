use axum::{middleware, Router};

use crate::server::{routes::v1::middleware::auth::require_admin, AppRouter};

mod clients;
mod download;

pub(super) fn router() -> AppRouter {
    Router::new()
        .nest("/clients", clients::router())
        .nest("/download", download::router())
        .route_layer(middleware::from_fn(require_admin))
}
