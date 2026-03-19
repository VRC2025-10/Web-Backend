use std::env;

use axum::http::HeaderValue;
use reqwest::Url;
use secrecy::SecretString;

const MIN_SECRET_BYTES: usize = 32;

/// Application configuration loaded from environment variables.
#[derive(Clone)]
pub struct AppConfig {
    // Server
    pub bind_address: String,

    // Database
    pub database_url: SecretString,
    pub database_max_connections: u32,

    // Discord OAuth2
    pub discord_client_id: String,
    pub discord_client_secret: SecretString,
    pub discord_guild_id: String,

    // URLs
    pub backend_base_url: String,
    pub frontend_origin: String,
    pub frontend_origin_header: HeaderValue,

    // Security
    pub session_secret: SecretString,
    pub system_api_token: SecretString,
    pub session_max_age_secs: i64,
    pub session_cleanup_interval_secs: u64,
    pub event_archival_interval_secs: u64,

    // Optional
    pub super_admin_discord_id: Option<String>,
    pub discord_webhook_url: Option<String>,

    // Feature flags
    pub cookie_secure: bool,
    pub trust_x_forwarded_for: bool,
}

impl AppConfig {
    /// Load configuration from environment variables.
    ///
    /// # Errors
    ///
    /// Returns an error if required environment variables are missing.
    pub fn from_env() -> Result<Self, ConfigError> {
        let backend_base_url = parse_origin_env("BACKEND_BASE_URL")?;
        let frontend_origin = parse_origin_env("FRONTEND_ORIGIN")?;
        let frontend_origin_header =
            HeaderValue::from_str(&frontend_origin).map_err(|error| ConfigError::InvalidEnv {
                key: "FRONTEND_ORIGIN".to_owned(),
                value: frontend_origin.clone(),
                reason: format!("must be a valid header value: {error}"),
            })?;

        Ok(Self {
            bind_address: env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:3000".to_owned()),
            database_url: SecretString::from(require_env("DATABASE_URL")?),
            database_max_connections: parse_positive_u32_env("DATABASE_MAX_CONNECTIONS", 10)?,
            discord_client_id: require_env("DISCORD_CLIENT_ID")?,
            discord_client_secret: SecretString::from(require_env("DISCORD_CLIENT_SECRET")?),
            discord_guild_id: require_env("DISCORD_GUILD_ID")?,
            backend_base_url,
            frontend_origin,
            frontend_origin_header,
            session_secret: require_secret_env("SESSION_SECRET")?,
            system_api_token: require_secret_env("SYSTEM_API_TOKEN")?,
            session_max_age_secs: parse_positive_i64_env("SESSION_MAX_AGE_SECS", 604_800)?,
            session_cleanup_interval_secs: parse_positive_u64_env(
                "SESSION_CLEANUP_INTERVAL_SECS",
                3600,
            )?,
            event_archival_interval_secs: parse_positive_u64_env(
                "EVENT_ARCHIVAL_INTERVAL_SECS",
                3600,
            )?,
            super_admin_discord_id: optional_env("SUPER_ADMIN_DISCORD_ID")?,
            discord_webhook_url: optional_env("DISCORD_WEBHOOK_URL")?,
            cookie_secure: parse_bool_env("COOKIE_SECURE", true)?,
            trust_x_forwarded_for: parse_bool_env("TRUST_X_FORWARDED_FOR", false)?,
        })
    }
}

fn require_env(key: &str) -> Result<String, ConfigError> {
    optional_env(key)?.ok_or_else(|| ConfigError::MissingEnv(key.to_owned()))
}

fn optional_env(key: &str) -> Result<Option<String>, ConfigError> {
    match env::var(key) {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(_)) => Err(ConfigError::InvalidEnv {
            key: key.to_owned(),
            value: "<non-unicode>".to_owned(),
            reason: "must be valid UTF-8".to_owned(),
        }),
    }
}

fn require_secret_env(key: &str) -> Result<SecretString, ConfigError> {
    let value = require_env(key)?;
    if value.len() < MIN_SECRET_BYTES {
        return Err(ConfigError::InvalidEnv {
            key: key.to_owned(),
            value: "<redacted>".to_owned(),
            reason: format!("must be at least {MIN_SECRET_BYTES} bytes long"),
        });
    }
    Ok(SecretString::from(value))
}

fn parse_origin_env(key: &str) -> Result<String, ConfigError> {
    let value = require_env(key)?;
    normalize_origin(key, &value)
}

fn normalize_origin(key: &str, value: &str) -> Result<String, ConfigError> {
    let parsed = Url::parse(value).map_err(|error| ConfigError::InvalidEnv {
        key: key.to_owned(),
        value: value.to_owned(),
        reason: format!("must be an absolute URL: {error}"),
    })?;

    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(ConfigError::InvalidEnv {
            key: key.to_owned(),
            value: value.to_owned(),
            reason: "must use http or https".to_owned(),
        });
    }

    if parsed.host_str().is_none() {
        return Err(ConfigError::InvalidEnv {
            key: key.to_owned(),
            value: value.to_owned(),
            reason: "must include a host".to_owned(),
        });
    }

    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(ConfigError::InvalidEnv {
            key: key.to_owned(),
            value: value.to_owned(),
            reason: "must not include userinfo".to_owned(),
        });
    }

    if parsed.path() != "/" || parsed.query().is_some() || parsed.fragment().is_some() {
        return Err(ConfigError::InvalidEnv {
            key: key.to_owned(),
            value: value.to_owned(),
            reason: "must be an origin without path, query, or fragment".to_owned(),
        });
    }

    Ok(parsed.origin().ascii_serialization())
}

fn parse_positive_u32_env(key: &str, default: u32) -> Result<u32, ConfigError> {
    match optional_env(key)? {
        Some(value) => {
            let parsed = value
                .parse::<u32>()
                .map_err(|error| ConfigError::InvalidEnv {
                    key: key.to_owned(),
                    value: value.clone(),
                    reason: format!("must be a positive integer: {error}"),
                })?;
            if parsed == 0 {
                return Err(ConfigError::InvalidEnv {
                    key: key.to_owned(),
                    value,
                    reason: "must be greater than zero".to_owned(),
                });
            }
            Ok(parsed)
        }
        None => Ok(default),
    }
}

fn parse_positive_u64_env(key: &str, default: u64) -> Result<u64, ConfigError> {
    match optional_env(key)? {
        Some(value) => {
            let parsed = value
                .parse::<u64>()
                .map_err(|error| ConfigError::InvalidEnv {
                    key: key.to_owned(),
                    value: value.clone(),
                    reason: format!("must be a positive integer: {error}"),
                })?;
            if parsed == 0 {
                return Err(ConfigError::InvalidEnv {
                    key: key.to_owned(),
                    value,
                    reason: "must be greater than zero".to_owned(),
                });
            }
            Ok(parsed)
        }
        None => Ok(default),
    }
}

fn parse_positive_i64_env(key: &str, default: i64) -> Result<i64, ConfigError> {
    match optional_env(key)? {
        Some(value) => {
            let parsed = value
                .parse::<i64>()
                .map_err(|error| ConfigError::InvalidEnv {
                    key: key.to_owned(),
                    value: value.clone(),
                    reason: format!("must be a positive integer: {error}"),
                })?;
            if parsed <= 0 {
                return Err(ConfigError::InvalidEnv {
                    key: key.to_owned(),
                    value,
                    reason: "must be greater than zero".to_owned(),
                });
            }
            Ok(parsed)
        }
        None => Ok(default),
    }
}

fn parse_bool_env(key: &str, default: bool) -> Result<bool, ConfigError> {
    match optional_env(key)? {
        Some(value) => match value.to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => Ok(true),
            "false" | "0" | "no" | "off" => Ok(false),
            _ => Err(ConfigError::InvalidEnv {
                key: key.to_owned(),
                value,
                reason: "must be a boolean (true/false)".to_owned(),
            }),
        },
        None => Ok(default),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing required environment variable: {0}")]
    MissingEnv(String),
    #[error("Invalid environment variable {key}: {reason}")]
    InvalidEnv {
        key: String,
        value: String,
        reason: String,
    },
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::sync::{LazyLock, Mutex};

    use secrecy::ExposeSecret;

    use super::*;

    #[cfg(unix)]
    use std::os::unix::ffi::OsStringExt;

    static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    struct EnvVarGuard {
        key: String,
        original: Option<OsString>,
    }

    impl EnvVarGuard {
        fn set(key: &str, value: Option<&str>) -> Self {
            let original = env::var_os(key);
            match value {
                Some(value) => {
                    // SAFETY: Tests serialize environment mutation via ENV_LOCK and restore the
                    // previous value when the guard is dropped.
                    unsafe { env::set_var(key, value) };
                }
                None => {
                    // SAFETY: Tests serialize environment mutation via ENV_LOCK and restore the
                    // previous value when the guard is dropped.
                    unsafe { env::remove_var(key) };
                }
            }

            Self {
                key: key.to_owned(),
                original,
            }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match &self.original {
                Some(value) => {
                    // SAFETY: Tests serialize environment mutation via ENV_LOCK and restore the
                    // original value before releasing the guard.
                    unsafe { env::set_var(&self.key, value) };
                }
                None => {
                    // SAFETY: Tests serialize environment mutation via ENV_LOCK and restore the
                    // original value before releasing the guard.
                    unsafe { env::remove_var(&self.key) };
                }
            }
        }
    }

    fn required_env_guards() -> Vec<EnvVarGuard> {
        vec![
            EnvVarGuard::set("DATABASE_URL", Some("postgres://user:pass@localhost/db")),
            EnvVarGuard::set("DISCORD_CLIENT_ID", Some("discord-client")),
            EnvVarGuard::set(
                "DISCORD_CLIENT_SECRET",
                Some("0123456789abcdef0123456789abcdef"),
            ),
            EnvVarGuard::set("DISCORD_GUILD_ID", Some("guild-id")),
            EnvVarGuard::set("BACKEND_BASE_URL", Some("https://backend.example/")),
            EnvVarGuard::set("FRONTEND_ORIGIN", Some("https://frontend.example/")),
            EnvVarGuard::set(
                "SESSION_SECRET",
                Some("abcdefghijklmnopqrstuvwxyz012345"),
            ),
            EnvVarGuard::set(
                "SYSTEM_API_TOKEN",
                Some("0123456789abcdefghijklmnopqrstuvwxyz"),
            ),
        ]
    }

    // Spec refs: application-security.md and config contract enforced by AppConfig::from_env.
    // Coverage: origin validation, numeric parsing, boolean parsing, secret validation, and env loading.

    #[test]
    fn test_require_env_returns_missing_error_for_absent_key() {
        let _lock = ENV_LOCK.lock().expect("environment test lock poisoned");
        let _guard = EnvVarGuard::set("UNSET_REQUIRED_ENV", None);

        let error = require_env("UNSET_REQUIRED_ENV").expect_err("missing env must fail");

        assert!(matches!(error, ConfigError::MissingEnv(key) if key == "UNSET_REQUIRED_ENV"));
    }

    #[test]
    fn test_normalize_origin_strips_trailing_slash() {
        let result = normalize_origin("FRONTEND_ORIGIN", "https://example.com/");

        assert!(matches!(result.as_deref(), Ok("https://example.com")));
    }

    #[test]
    fn test_normalize_origin_rejects_path() {
        let error = normalize_origin("FRONTEND_ORIGIN", "https://example.com/app")
            .expect_err("path should be rejected");
        assert!(matches!(
            error,
            ConfigError::InvalidEnv { key, .. } if key == "FRONTEND_ORIGIN"
        ));
    }

    #[test]
    fn test_normalize_origin_rejects_non_http_scheme() {
        assert!(normalize_origin("FRONTEND_ORIGIN", "ftp://example.com").is_err());
    }

    #[test]
    fn test_normalize_origin_rejects_query_string() {
        let error = normalize_origin("FRONTEND_ORIGIN", "https://example.com?next=/dashboard")
            .expect_err("query string must be rejected");

        assert!(matches!(error, ConfigError::InvalidEnv { key, .. } if key == "FRONTEND_ORIGIN"));
    }

    #[test]
    fn test_normalize_origin_rejects_userinfo() {
        let error = normalize_origin("FRONTEND_ORIGIN", "https://alice@example.com")
            .expect_err("userinfo must be rejected");

        assert!(matches!(error, ConfigError::InvalidEnv { key, .. } if key == "FRONTEND_ORIGIN"));
    }

    #[test]
    fn test_parse_bool_env_rejects_invalid_value() {
        let _lock = ENV_LOCK.lock().expect("environment test lock poisoned");
        let _guard = EnvVarGuard::set("COOKIE_SECURE", Some("definitely"));
        let error = parse_bool_env("COOKIE_SECURE", true).expect_err("invalid boolean must fail");

        assert!(matches!(
            error,
            ConfigError::InvalidEnv { key, .. } if key == "COOKIE_SECURE"
        ));
    }

    #[test]
    fn test_require_secret_env_rejects_short_values() {
        let _lock = ENV_LOCK.lock().expect("environment test lock poisoned");
        let _guard = EnvVarGuard::set("SESSION_SECRET", Some("short"));
        let error = require_secret_env("SESSION_SECRET").expect_err("short secret must fail");

        assert!(matches!(
            error,
            ConfigError::InvalidEnv { key, .. } if key == "SESSION_SECRET"
        ));
    }

    #[test]
    fn test_parse_bool_env_accepts_false_alias() {
        let _lock = ENV_LOCK.lock().expect("environment test lock poisoned");
        let _guard = EnvVarGuard::set("COOKIE_SECURE", Some("off"));

        let value = parse_bool_env("COOKIE_SECURE", true).expect("boolean alias must parse");

        assert!(!value);
    }

    #[test]
    fn test_parse_positive_u32_env_rejects_zero() {
        let _lock = ENV_LOCK.lock().expect("environment test lock poisoned");
        let _guard = EnvVarGuard::set("DATABASE_MAX_CONNECTIONS", Some("0"));

        let error = parse_positive_u32_env("DATABASE_MAX_CONNECTIONS", 10)
            .expect_err("zero must be rejected");

        assert!(matches!(error, ConfigError::InvalidEnv { key, .. } if key == "DATABASE_MAX_CONNECTIONS"));
    }

    #[test]
    fn test_parse_positive_u32_env_rejects_non_numeric_value() {
        let _lock = ENV_LOCK.lock().expect("environment test lock poisoned");
        let _guard = EnvVarGuard::set("DATABASE_MAX_CONNECTIONS", Some("ten"));

        let error = parse_positive_u32_env("DATABASE_MAX_CONNECTIONS", 10)
            .expect_err("non numeric value must be rejected");

        assert!(matches!(error, ConfigError::InvalidEnv { key, .. } if key == "DATABASE_MAX_CONNECTIONS"));
    }

    #[test]
    fn test_parse_positive_i64_env_rejects_negative_value() {
        let _lock = ENV_LOCK.lock().expect("environment test lock poisoned");
        let _guard = EnvVarGuard::set("SESSION_MAX_AGE_SECS", Some("-1"));

        let error = parse_positive_i64_env("SESSION_MAX_AGE_SECS", 10)
            .expect_err("negative value must be rejected");

        assert!(matches!(error, ConfigError::InvalidEnv { key, .. } if key == "SESSION_MAX_AGE_SECS"));
    }

    #[cfg(unix)]
    #[test]
    fn test_optional_env_rejects_non_unicode_values() {
        let _lock = ENV_LOCK.lock().expect("environment test lock poisoned");
        let original = env::var_os("NON_UNICODE_ENV");
        // SAFETY: Tests serialize environment mutation via ENV_LOCK and restore the value below.
        unsafe { env::set_var("NON_UNICODE_ENV", OsString::from_vec(vec![0x66, 0x6f, 0x80])) };

        let error = optional_env("NON_UNICODE_ENV").expect_err("non-unicode env must fail");

        match original {
            Some(value) => {
                // SAFETY: Tests serialize environment mutation via ENV_LOCK and restore the value below.
                unsafe { env::set_var("NON_UNICODE_ENV", value) };
            }
            None => {
                // SAFETY: Tests serialize environment mutation via ENV_LOCK and restore the value below.
                unsafe { env::remove_var("NON_UNICODE_ENV") };
            }
        }

        assert!(matches!(error, ConfigError::InvalidEnv { key, .. } if key == "NON_UNICODE_ENV"));
    }

    #[test]
    fn test_require_secret_env_accepts_exact_minimum_length() {
        let _lock = ENV_LOCK.lock().expect("environment test lock poisoned");
        let _guard = EnvVarGuard::set("SESSION_SECRET", Some("12345678901234567890123456789012"));

        let secret = require_secret_env("SESSION_SECRET").expect("32 byte secret must be accepted");

        assert_eq!(secret.expose_secret(), "12345678901234567890123456789012");
    }

    #[test]
    fn test_from_env_loads_defaults_and_normalizes_origins() {
        let _lock = ENV_LOCK.lock().expect("environment test lock poisoned");
        let _guards = required_env_guards();
        let _optional = [
            EnvVarGuard::set("DATABASE_MAX_CONNECTIONS", None),
            EnvVarGuard::set("SESSION_MAX_AGE_SECS", None),
            EnvVarGuard::set("SESSION_CLEANUP_INTERVAL_SECS", None),
            EnvVarGuard::set("EVENT_ARCHIVAL_INTERVAL_SECS", None),
            EnvVarGuard::set("COOKIE_SECURE", None),
            EnvVarGuard::set("TRUST_X_FORWARDED_FOR", None),
        ];

        let config = AppConfig::from_env().expect("required env must build config");

        assert_eq!(config.bind_address, "0.0.0.0:3000");
        assert_eq!(config.database_max_connections, 10);
        assert_eq!(config.backend_base_url, "https://backend.example");
        assert_eq!(config.frontend_origin, "https://frontend.example");
        assert_eq!(config.session_max_age_secs, 604_800);
        assert_eq!(config.session_cleanup_interval_secs, 3600);
        assert_eq!(config.event_archival_interval_secs, 3600);
        assert!(config.cookie_secure);
        assert!(!config.trust_x_forwarded_for);
        assert_eq!(
            config.frontend_origin_header.to_str().expect("header must be valid"),
            "https://frontend.example"
        );
    }

    #[test]
    fn test_from_env_parses_boolean_aliases() {
        let _lock = ENV_LOCK.lock().expect("environment test lock poisoned");
        let _guards = required_env_guards();
        let _optional = [
            EnvVarGuard::set("COOKIE_SECURE", Some("off")),
            EnvVarGuard::set("TRUST_X_FORWARDED_FOR", Some("yes")),
        ];

        let config = AppConfig::from_env().expect("boolean aliases must be accepted");

        assert!(!config.cookie_secure);
        assert!(config.trust_x_forwarded_for);
    }
}
