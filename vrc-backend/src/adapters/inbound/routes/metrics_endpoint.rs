use std::sync::Arc;

use axum::Router;
use axum::response::IntoResponse;
use axum::routing::get;

use crate::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/metrics", get(metrics_handler))
}

/// Expose Prometheus-format metrics at `/metrics`.
///
/// The `metrics-exporter-prometheus` recorder is installed globally
/// in `main.rs` on startup. This handler renders the current snapshot.
async fn metrics_handler() -> impl IntoResponse {
    // The PrometheusHandle is stored as a global — we retrieve it via the recorder.
    // We use the `metrics_exporter_prometheus` global handle approach.
    let body = crate::METRICS_HANDLE
        .get()
        .map(|h| h.render())
        .unwrap_or_default();

    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
}
