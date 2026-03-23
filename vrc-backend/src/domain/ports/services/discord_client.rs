use crate::errors::infrastructure::InfraError;

pub trait DiscordClient: Send + Sync {
    fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
    ) -> impl std::future::Future<Output = Result<DiscordTokenResponse, InfraError>> + Send;

    fn get_user(
        &self,
        access_token: &str,
    ) -> impl std::future::Future<Output = Result<DiscordUser, InfraError>> + Send;

    fn get_user_guilds(
        &self,
        access_token: &str,
    ) -> impl std::future::Future<Output = Result<Vec<DiscordGuild>, InfraError>> + Send;

    fn refresh_token(
        &self,
        refresh_token: &str,
    ) -> impl std::future::Future<Output = Result<DiscordTokenResponse, InfraError>> + Send;

    fn get_current_guild_member(
        &self,
        access_token: &str,
        guild_id: &str,
    ) -> impl std::future::Future<Output = Result<DiscordGuildMember, InfraError>> + Send;
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DiscordTokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: i64,
    pub token_type: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DiscordUser {
    pub id: String,
    pub username: String,
    pub global_name: Option<String>,
    pub avatar: Option<String>,
}

impl DiscordUser {
    /// Build the Discord CDN avatar URL.
    pub fn avatar_url(&self) -> Option<String> {
        self.avatar.as_ref().map(|hash| {
            format!(
                "https://cdn.discordapp.com/avatars/{}/{}.png",
                self.id, hash
            )
        })
    }

    /// Return display name: prefer `global_name`, fallback to username.
    pub fn display_name(&self) -> &str {
        self.global_name.as_deref().unwrap_or(&self.username)
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DiscordGuild {
    pub id: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DiscordGuildMember {
    #[serde(default)]
    pub roles: Vec<String>,
    pub nick: Option<String>,
}
