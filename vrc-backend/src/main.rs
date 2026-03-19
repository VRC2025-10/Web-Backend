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
    #[error("Failed to bootstrap super admin: {0}")]
    SuperAdminBootstrap(#[source] sqlx::Error),
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

    // Bootstrap super admin if configured
    if let Some(ref discord_id) = config.super_admin_discord_id {
        bootstrap_super_admin(&db_pool, discord_id).await?;
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

async fn bootstrap_super_admin(
    db_pool: &sqlx::PgPool,
    discord_id: &str,
) -> Result<(), StartupError> {
    let mut transaction = db_pool
        .begin()
        .await
        .map_err(StartupError::SuperAdminBootstrap)?;

    // FR-AUTH-008: Upsert user with super_admin role
    let user_id = sqlx::query_scalar::<_, uuid::Uuid>(
        r"
        INSERT INTO users (discord_id, discord_username, discord_display_name, role, status)
        VALUES ($1, 'SuperAdmin', 'SuperAdmin', 'super_admin', 'active')
        ON CONFLICT (discord_id) DO UPDATE SET
            role = 'super_admin',
            status = 'active',
            updated_at = NOW()
        RETURNING id
        ",
    )
    .bind(discord_id)
    .fetch_one(&mut *transaction)
    .await
    .map_err(StartupError::SuperAdminBootstrap)?;

    // FR-AUTH-008: Seed a private bootstrap profile if none exists.
    sqlx::query(
        r"
        INSERT INTO profiles (user_id, nickname, bio_markdown, bio_html, is_public, updated_at)
        VALUES ($1, 'SuperAdmin', '', '', false, NOW())
        ON CONFLICT (user_id) DO NOTHING
        ",
    )
    .bind(user_id)
    .execute(&mut *transaction)
    .await
    .map_err(StartupError::SuperAdminBootstrap)?;

    transaction
        .commit()
        .await
        .map_err(StartupError::SuperAdminBootstrap)?;

    tracing::info!(discord_id = discord_id, user_id = %user_id, "Super admin bootstrapped");
    Ok(())
}
