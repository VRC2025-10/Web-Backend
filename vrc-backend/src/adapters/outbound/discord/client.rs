use crate::domain::ports::services::discord_client::{
    DiscordClient, DiscordGuild, DiscordTokenResponse, DiscordUser,
};
use crate::errors::infrastructure::InfraError;

pub struct ReqwestDiscordClient {
    http: reqwest::Client,
    client_id: String,
    client_secret: String,
}

impl ReqwestDiscordClient {
    pub fn new(http: reqwest::Client, client_id: String, client_secret: String) -> Self {
        Self {
            http,
            client_id,
            client_secret,
        }
    }
}

impl DiscordClient for ReqwestDiscordClient {
    async fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
    ) -> Result<DiscordTokenResponse, InfraError> {
        let params = [
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
        ];

        let resp = self
            .http
            .post("https://discord.com/api/v10/oauth2/token")
            .form(&params)
            .send()
            .await
            .map_err(|e| InfraError::DiscordApi(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            tracing::error!(status = %status, body = %body, "Discord token exchange failed");
            return Err(InfraError::TokenExchange);
        }

        resp.json::<DiscordTokenResponse>()
            .await
            .map_err(|e| InfraError::DiscordApi(e.to_string()))
    }

    async fn get_user(&self, access_token: &str) -> Result<DiscordUser, InfraError> {
        let resp = self
            .http
            .get("https://discord.com/api/v10/users/@me")
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| InfraError::DiscordApi(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            tracing::error!(status = %status, body = %body, "Discord get user failed");
            return Err(InfraError::DiscordApi("Failed to fetch user".to_owned()));
        }

        resp.json::<DiscordUser>()
            .await
            .map_err(|e| InfraError::DiscordApi(e.to_string()))
    }

    async fn get_user_guilds(
        &self,
        access_token: &str,
    ) -> Result<Vec<DiscordGuild>, InfraError> {
        let resp = self
            .http
            .get("https://discord.com/api/v10/users/@me/guilds")
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| InfraError::DiscordApi(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            tracing::error!(status = %status, body = %body, "Discord get guilds failed");
            return Err(InfraError::DiscordApi("Failed to fetch guilds".to_owned()));
        }

        resp.json::<Vec<DiscordGuild>>()
            .await
            .map_err(|e| InfraError::DiscordApi(e.to_string()))
    }
}
