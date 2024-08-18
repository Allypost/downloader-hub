use axum::{extract::State, http::StatusCode, routing::any, Router};
use sea_orm::{prelude::*, Statement};
use tracing::debug;

use crate::server::{app_response::ApiResponse, AppRouter, AppState};

pub(super) fn router() -> AppRouter {
    Router::new()
        .route("/ping", any(ping))
        .route("/ping/db", any(db_ping))
}

async fn ping() -> ApiResponse<&'static str> {
    "pong".into()
}

async fn db_ping(State(state): State<AppState>) -> ApiResponse<i32> {
    let res = {
        let start = std::time::Instant::now();
        let res = state
            .db
            .conn
            .query_one(Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT 1 as \"val\";",
            ))
            .await;
        let elapsed = start.elapsed();

        debug!("Database ping took {elapsed:?}");

        res
    };

    let res = match res {
        Ok(Some(res)) => res,
        Ok(None) => {
            return ApiResponse::new("v0", StatusCode::INTERNAL_SERVER_ERROR)
                .with_error_body("Failed to query".into());
        }
        Err(e) => {
            return ApiResponse::new("v0", StatusCode::INTERNAL_SERVER_ERROR)
                .with_error_body(e.to_string().into());
        }
    };

    res.try_get::<i32>("", "val").unwrap_or_default().into()
}
