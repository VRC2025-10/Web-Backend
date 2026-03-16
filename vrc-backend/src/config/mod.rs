use std::env;

/// Application configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct AppConfig {
    // Server
    pub bind_address: String,

    // Database
    pub database_url: String,
    pub database_max_connections: u32,

    // Discord OAuth2
    pub discord_client_id: String,
    pub discord_client_secret: String,
    pub discord_guild_id: String,

    // URLs
    pub backend_base_url: String,
    pub frontend_origin: String,

    // Security
    pub session_secret: String,
    pub system_api_token: String,
    pub session_max_age_secs: i64,
    pub session_cleanup_interval_secs: u64,

    // Optional
    pub super_admin_discord_id: Option<String>,
    pub discord_webhook_url: Option<String>,

    // Feature flags
    pub cookie_secure: bool,
}

impl AppConfig {
    /// Load configuration from environment variables.
    ///
    /// # Errors
    ///
    /// Returns an error if required environment variables are missing.
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            bind_address: env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:3000".to_owned()),
            database_url: require_env("DATABASE_URL")?,
            database_max_connections: env::var("DATABASE_MAX_CONNECTIONS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            discord_client_id: require_env("DISCORD_CLIENT_ID")?,
            discord_client_secret: require_env("DISCORD_CLIENT_SECRET")?,
            discord_guild_id: require_env("DISCORD_GUILD_ID")?,
            backend_base_url: require_env("BACKEND_BASE_URL")?,
            frontend_origin: require_env("FRONTEND_ORIGIN")?,
            session_secret: require_env("SESSION_SECRET")?,
            system_api_token: require_env("SYSTEM_API_TOKEN")?,
            session_max_age_secs: env::var("SESSION_MAX_AGE_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(604_800), // 7 days
            session_cleanup_interval_secs: env::var("SESSION_CLEANUP_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3600),
            super_admin_discord_id: env::var("SUPER_ADMIN_DISCORD_ID").ok(),
            discord_webhook_url: env::var("DISCORD_WEBHOOK_URL").ok(),
            cookie_secure: env::var("COOKIE_SECURE")
                .map(|v| v != "false")
                .unwrap_or(true),
        })
    }
}

fn require_env(key: &str) -> Result<String, ConfigError> {
    env::var(key).map_err(|_| ConfigError::MissingEnv(key.to_owned()))
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing required environment variable: {0}")]
    MissingEnv(String),
}
