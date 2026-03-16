/// Infrastructure failures. Not exposed to clients.
#[derive(Debug, thiserror::Error)]
pub enum InfraError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Discord API error: {0}")]
    DiscordApi(String),
    #[error("Webhook delivery failed: {0}")]
    Webhook(String),
    #[error("Token exchange failed")]
    TokenExchange,
}
