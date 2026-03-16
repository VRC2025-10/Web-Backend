use std::time::Instant;

pub mod adapters;
pub mod auth;
pub mod background;
pub mod config;
pub mod domain;
pub mod errors;

pub struct AppState {
    pub db_pool: sqlx::PgPool,
    pub http_client: reqwest::Client,
    pub config: config::AppConfig,
    pub start_time: Instant,
}
