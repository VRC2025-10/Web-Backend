use crate::errors::infrastructure::InfraError;

/// Outbound port for sending webhook notifications.
///
/// Implementations deliver structured payloads to external services
/// (e.g., Discord webhook API). The trait is optional at runtime —
/// if no webhook URL is configured, callers skip invocation entirely.
pub trait WebhookSender: Send + Sync {
    fn send_embed(
        &self,
        title: &str,
        description: &str,
        color: u32,
        fields: Vec<EmbedField>,
    ) -> impl std::future::Future<Output = Result<(), InfraError>> + Send;
}

/// A single field inside a webhook embed.
#[derive(Debug, Clone)]
pub struct EmbedField {
    pub name: String,
    pub value: String,
    pub inline: bool,
}
