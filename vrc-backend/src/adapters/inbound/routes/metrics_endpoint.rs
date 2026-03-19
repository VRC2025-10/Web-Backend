use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::AppState;

use secrecy::ExposeSecret;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/metrics", get(metrics_handler))
}

/// Expose Prometheus-format metrics at `/metrics`.
///
/// Protected by the system API token (Bearer authentication) with SHA-256
/// hashing + constant-time comparison (NFR-SEC-004) to prevent timing attacks
/// and information disclosure of request patterns, paths, and status codes.
#[vrc_macros::handler(method = GET, path = "/metrics", summary = "Prometheus metrics")]
async fn metrics_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // Require Bearer token matching SYSTEM_API_TOKEN via SHA-256 + constant-time eq
    let authorized = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .is_some_and(|token| {
            let token_hash = Sha256::digest(token.as_bytes());
            let expected_hash = Sha256::digest(state.config.system_api_token.expose_secret().as_bytes());
            token_hash.ct_eq(&expected_hash).into()
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
