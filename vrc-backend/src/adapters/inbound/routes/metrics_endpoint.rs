use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use subtle::ConstantTimeEq;

use crate::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/metrics", get(metrics_handler))
}

/// Expose Prometheus-format metrics at `/metrics`.
///
/// Protected by the system API token (Bearer authentication) to prevent
/// information disclosure of request patterns, paths, and status codes.
async fn metrics_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // Require Bearer token matching SYSTEM_API_TOKEN
    let authorized = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .is_some_and(|token| {
            token
                .as_bytes()
                .ct_eq(state.config.system_api_token.as_bytes())
                .into()
        });

    if !authorized {
        return (
            StatusCode::UNAUTHORIZED,
            [(
                axum::http::header::CONTENT_TYPE,
                "text/plain; charset=utf-8",
            )],
            "Unauthorized".to_owned(),
        );
    }

    let body = crate::METRICS_HANDLE
        .get()
        .map(metrics_exporter_prometheus::PrometheusHandle::render)
        .unwrap_or_default();

    (
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
}
