use std::process::ExitCode;
use std::time::{Duration, Instant};

use tokio::net::TcpListener;
use tokio::signal;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use vrc_backend::AppState;
use vrc_backend::adapters::inbound::routes;
use vrc_backend::adapters::outbound::discord::webhook::DiscordWebhookSender;
use vrc_backend::background::scheduler;
use vrc_backend::config::AppConfig;

use secrecy::ExposeSecret;

#[tokio::main]
async fn main() -> ExitCode {
    dotenvy::dotenv().ok();

    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(error = %error, "Application startup failed");
            eprintln!("application startup failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum StartupError {
    #[error("Failed to parse default log filter: {0}")]
    InvalidDefaultLogFilter(String),
    #[error("Failed to install Prometheus recorder: {0}")]
    MetricsRecorder(String),
    #[error("Metrics handle already initialised")]
    MetricsHandleAlreadyInitialised,
    #[error("{0}")]
    Config(#[from] vrc_backend::config::ConfigError),
    #[error("Failed to connect to database: {0}")]
    DatabaseConnect(#[source] sqlx::Error),
    #[error("Failed to run database migrations: {0}")]
    DatabaseMigration(#[source] sqlx::migrate::MigrateError),
    #[error("Failed to create HTTP client: {0}")]
    HttpClient(#[source] reqwest::Error),
    #[error("{0}")]
    Router(#[from] vrc_backend::adapters::inbound::routes::RouteBuildError),
    #[error("Failed to bind TCP listener: {0}")]
    TcpBind(#[source] std::io::Error),
    #[error("Server error: {0}")]
    Server(#[source] std::io::Error),
    #[error("Failed to sync super admin configuration: {0}")]
    SuperAdminSync(#[source] sqlx::Error),
}

async fn run() -> Result<(), StartupError> {
    init_tracing()?;

    // Install Prometheus metrics recorder (must happen before any metrics are recorded)
    let prometheus_builder = metrics_exporter_prometheus::PrometheusBuilder::new();
    let prometheus_handle = prometheus_builder
        .install_recorder()
        .map_err(|error| StartupError::MetricsRecorder(error.to_string()))?;
    vrc_backend::METRICS_HANDLE
        .set(prometheus_handle)
        .map_err(|_| StartupError::MetricsHandleAlreadyInitialised)?;

    let config = AppConfig::from_env()?;
    tracing::info!(
        bind_addr = %config.bind_address,
        "Starting VRC Backend v{}",
        env!("CARGO_PKG_VERSION")
    );

    let db_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .min_connections(2)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .idle_timeout(std::time::Duration::from_secs(600))
        .max_lifetime(std::time::Duration::from_secs(1800))
        .connect(config.database_url.expose_secret())
        .await
        .map_err(StartupError::DatabaseConnect)?;

    tracing::info!("Running database migrations...");
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .map_err(StartupError::DatabaseMigration)?;

    let super_admin_ids = config.super_admin_discord_ids();
    if !super_admin_ids.is_empty() {
        tracing::info!(count = super_admin_ids.len(), "Loaded super admin login allowlist");
    }

    // Remove any legacy synthetic bootstrap user rows. Super-admin elevation
    // now happens on real Discord login in the OAuth callback.
    for discord_id in super_admin_ids {
        cleanup_legacy_bootstrap_super_admin(&db_pool, discord_id).await?;
    }

    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(StartupError::HttpClient)?;

    let webhook = config.discord_webhook_url.as_ref().map(|url| {
        tracing::info!("Discord webhook notifications enabled");
        DiscordWebhookSender::new(http_client.clone(), url.clone())
    });

    let state = std::sync::Arc::new(AppState {
        db_pool: db_pool.clone(),
        http_client,
        config,
        start_time: Instant::now(),
        webhook,
    });

    // Start background tasks
    scheduler::spawn(
        db_pool.clone(),
        state.config.session_cleanup_interval_secs,
        state.config.event_archival_interval_secs,
    );

    let app = routes::build_router(state.clone())?;

    let listener = TcpListener::bind(&state.config.bind_address)
        .await
        .map_err(StartupError::TcpBind)?;

    tracing::info!("Listening on {}", state.config.bind_address);

    // NFR-AVAIL-005: Graceful shutdown — drain in-flight requests within 30 seconds
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(StartupError::Server)?;

    tracing::info!("Server shut down gracefully");
    Ok(())
}

fn init_tracing() -> Result<(), StartupError> {
    let filter = match EnvFilter::try_from_default_env() {
        Ok(filter) => filter,
        Err(_) => "vrc_backend=info,tower_http=info,sqlx=warn"
            .parse::<EnvFilter>()
            .map_err(|error| StartupError::InvalidDefaultLogFilter(error.to_string()))?,
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().json())
        .init();

    Ok(())
}

/// Wait for SIGTERM or SIGINT (Ctrl-C), then allow a 30-second drain window
/// for in-flight requests to complete before the process exits.
async fn shutdown_signal() {
    let ctrl_c = async {
        match signal::ctrl_c().await {
            Ok(()) => Some("SIGINT"),
            Err(error) => {
                tracing::error!(error = %error, "Failed to install Ctrl-C handler");
                std::future::pending::<Option<&'static str>>().await
            }
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
                Some("SIGTERM")
            }
            Err(error) => {
                tracing::error!(error = %error, "Failed to install SIGTERM handler");
                std::future::pending::<Option<&'static str>>().await
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<Option<&'static str>>();

    tokio::select! {
        Some(signal_name) = ctrl_c => {
            tracing::info!(signal = signal_name, "Received shutdown signal, starting graceful shutdown");
        }
        Some(signal_name) = terminate => {
            tracing::info!(signal = signal_name, "Received shutdown signal, starting graceful shutdown");
        }
    }

    // Give in-flight requests up to 30 seconds to drain
    tokio::time::sleep(Duration::from_secs(30)).await;
}

async fn cleanup_legacy_bootstrap_super_admin(
    db_pool: &sqlx::PgPool,
    discord_id: &str,
) -> Result<(), StartupError> {
    let mut transaction = db_pool
        .begin()
        .await
        .map_err(StartupError::SuperAdminSync)?;

    let user_id = sqlx::query_scalar::<_, uuid::Uuid>(
        r"
        SELECT u.id
        FROM users u
        JOIN profiles p ON p.user_id = u.id
        WHERE u.discord_id = $1
          AND u.role = 'super_admin'
          AND u.status = 'active'
          AND u.discord_username = 'SuperAdmin'
          AND u.discord_display_name = 'SuperAdmin'
          AND u.discord_avatar_hash IS NULL
          AND u.avatar_url IS NULL
          AND p.nickname = 'SuperAdmin'
          AND p.bio_markdown = ''
          AND p.bio_html = ''
          AND p.avatar_url IS NULL
          AND p.is_public = false
          AND NOT EXISTS (SELECT 1 FROM sessions s WHERE s.user_id = u.id)
          AND NOT EXISTS (SELECT 1 FROM reports r WHERE r.reporter_user_id = u.id)
          AND NOT EXISTS (SELECT 1 FROM clubs c WHERE c.owner_user_id = u.id)
          AND NOT EXISTS (SELECT 1 FROM club_members cm WHERE cm.user_id = u.id)
          AND NOT EXISTS (SELECT 1 FROM gallery_images g WHERE g.uploaded_by_user_id = u.id)
          AND NOT EXISTS (SELECT 1 FROM events e WHERE e.host_user_id = u.id)
        FOR UPDATE
        ",
    )
    .bind(discord_id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(StartupError::SuperAdminSync)?;

    if let Some(user_id) = user_id {
        sqlx::query(
            r"
            DELETE FROM users
            WHERE id = $1
            ",
        )
        .bind(user_id)
        .execute(&mut *transaction)
        .await
        .map_err(StartupError::SuperAdminSync)?;

        tracing::info!(
            discord_id = discord_id,
            user_id = %user_id,
            "Removed legacy synthetic super admin placeholder"
        );
    }

    transaction
        .commit()
        .await
        .map_err(StartupError::SuperAdminSync)?;

    Ok(())
}
