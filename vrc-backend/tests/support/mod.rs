#![allow(dead_code)]

use std::sync::Arc;
use std::time::Instant;

use axum::body::{Body, to_bytes};
use secrecy::SecretString;
use vrc_backend::AppState;
use vrc_backend::adapters::inbound::routes;
use vrc_backend::config::AppConfig;

pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub fn test_config() -> AppConfig {
    AppConfig {
        bind_address: "127.0.0.1:0".to_owned(),
        database_url: SecretString::from("postgres://test:test@localhost/test".to_owned()),
        database_max_connections: 5,
        discord_client_id: "discord-client-id".to_owned(),
        discord_client_secret: SecretString::from(
            "0123456789abcdef0123456789abcdef".to_owned(),
        ),
        discord_guild_id: "guild-id".to_owned(),
        backend_base_url: "https://backend.example".to_owned(),
        frontend_origin: "https://frontend.example".to_owned(),
        frontend_origin_header: "https://frontend.example".parse().expect("valid header"),
        gallery_storage_dir: std::env::temp_dir().join("vrc-gallery-test-support"),
        gallery_max_upload_bytes: 10 * 1024 * 1024,
        session_secret: SecretString::from("abcdefghijklmnopqrstuvwxyz012345".to_owned()),
        system_api_token: SecretString::from(
            "0123456789abcdefghijklmnopqrstuvwxyz".to_owned(),
        ),
        session_max_age_secs: 604_800,
        session_cleanup_interval_secs: 3600,
        event_archival_interval_secs: 3600,
        super_admin_discord_id: None,
        discord_webhook_url: None,
        cookie_secure: false,
        trust_x_forwarded_for: false,
    }
}

pub fn build_app() -> TestResult<axum::Router> {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://test:test@localhost/test")?;
    let state = Arc::new(AppState {
        db_pool: pool,
        http_client: reqwest::Client::new(),
        config: test_config(),
        start_time: Instant::now(),
        webhook: None,
    });

    Ok(routes::build_router(state)?)
}

pub async fn parse_json(response: axum::http::Response<Body>) -> TestResult<serde_json::Value> {
    Ok(serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await?)?)
}