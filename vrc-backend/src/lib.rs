use std::sync::OnceLock;
use std::time::Instant;

use adapters::outbound::discord::webhook::DiscordWebhookSender;

pub mod adapters;
pub mod auth;
pub mod background;
pub mod config;
pub mod domain;
pub mod errors;

/// Global Prometheus metrics handle, initialised once in `main()`.
pub static METRICS_HANDLE: OnceLock<metrics_exporter_prometheus::PrometheusHandle> =
    OnceLock::new();

pub struct AppState {
    pub db_pool: sqlx::PgPool,
    pub http_client: reqwest::Client,
    pub config: config::AppConfig,
    pub start_time: Instant,
    /// Optional Discord webhook sender; `None` when `DISCORD_WEBHOOK_URL` is not set.
    pub webhook: Option<DiscordWebhookSender>,
}
