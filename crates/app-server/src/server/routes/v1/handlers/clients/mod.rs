use axum::{middleware, routing::get, Extension, Router};

use crate::server::{
    routes::v1::{
        middleware::auth::{require_auth, CurrentUser},
        response::V1Response,
    },
    AppRouter,
};

pub(super) fn router() -> AppRouter {
    Router::new()
        .route("/me/info", get(client_info))
        .route_layer(middleware::from_fn(require_auth))
}

async fn client_info(Extension(user): Extension<CurrentUser>) -> V1Response<CurrentUser> {
    V1Response::success(user)
}
