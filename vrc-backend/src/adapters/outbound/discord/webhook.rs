use crate::domain::ports::services::webhook_sender::{EmbedField, WebhookSender};
use crate::errors::infrastructure::InfraError;
use serde_json::json;

/// Discord webhook implementation using Discord's Execute Webhook API.
///
/// Sends rich embed messages to a configured Discord channel webhook URL.
/// See: <https://discord.com/developers/docs/resources/webhook#execute-webhook>
pub struct DiscordWebhookSender {
    http: reqwest::Client,
    webhook_url: String,
}

impl DiscordWebhookSender {
    pub fn new(http: reqwest::Client, webhook_url: String) -> Self {
        Self { http, webhook_url }
    }
}

impl WebhookSender for DiscordWebhookSender {
    async fn send_embed(
        &self,
        title: &str,
        description: &str,
        color: u32,
        fields: Vec<EmbedField>,
    ) -> Result<(), InfraError> {
        let embed_fields: Vec<serde_json::Value> = fields
            .into_iter()
            .map(|f| {
                json!({
                    "name": f.name,
                    "value": f.value,
                    "inline": f.inline,
                })
            })
            .collect();

        let payload = json!({
            "embeds": [{
                "title": title,
                "description": description,
                "color": color,
                "fields": embed_fields,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }]
        });

        let resp = self
            .http
            .post(&self.webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| InfraError::Webhook(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            tracing::error!(
                status = %status,
                body = %body,
                "Discord webhook delivery failed"
            );
            return Err(InfraError::Webhook(format!(
                "Discord returned {status}"
            )));
        }

        Ok(())
    }
}
