use std::time::{Duration, Instant};

use tokio::net::TcpListener;
use tokio::signal;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use vrc_backend::AppState;
use vrc_backend::adapters::inbound::routes;
use vrc_backend::adapters::outbound::discord::webhook::DiscordWebhookSender;
use vrc_backend::background::scheduler;
use vrc_backend::config::AppConfig;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            "vrc_backend=info,tower_http=info,sqlx=warn"
                .parse()
                .expect("valid filter")
        }))
        .with(fmt::layer().json())
        .init();

    // Install Prometheus metrics recorder (must happen before any metrics are recorded)
    let prometheus_builder = metrics_exporter_prometheus::PrometheusBuilder::new();
    let prometheus_handle = prometheus_builder
        .install_recorder()
        .expect("Failed to install Prometheus recorder");
    vrc_backend::METRICS_HANDLE
        .set(prometheus_handle)
        .expect("Metrics handle already initialised");

    let config = AppConfig::from_env().expect("Failed to load configuration");
    tracing::info!(
        bind_addr = %config.bind_address,
        "Starting VRC Backend v{}",
        env!("CARGO_PKG_VERSION")
    );

    let db_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to database");

    tracing::info!("Running database migrations...");
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to run database migrations");

    // Bootstrap super admin if configured
    if let Some(ref discord_id) = config.super_admin_discord_id {
        bootstrap_super_admin(&db_pool, discord_id).await;
    }

    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("Failed to create HTTP client");

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

    let app = routes::build_router(state.clone());

    let listener = TcpListener::bind(&state.config.bind_address)
        .await
        .expect("Failed to bind TCP listener");

    tracing::info!("Listening on {}", state.config.bind_address);

    // NFR-AVAIL-005: Graceful shutdown — drain in-flight requests within 30 seconds
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("Server error");

    tracing::info!("Server shut down gracefully");
}

/// Wait for SIGTERM or SIGINT (Ctrl-C), then allow a 30-second drain window
/// for in-flight requests to complete before the process exits.
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => tracing::info!("Received SIGINT, starting graceful shutdown"),
        () = terminate => tracing::info!("Received SIGTERM, starting graceful shutdown"),
    }

    // Give in-flight requests up to 30 seconds to drain
    tokio::time::sleep(Duration::from_secs(30)).await;
}

async fn bootstrap_super_admin(db_pool: &sqlx::PgPool, discord_id: &str) {
    // FR-AUTH-008: Upsert user with super_admin role
    let result = sqlx::query_scalar::<_, uuid::Uuid>(
        r"
        INSERT INTO users (discord_id, discord_username, discord_display_name, role, status)
        VALUES ($1, 'SuperAdmin', 'SuperAdmin', 'super_admin', 'active')
        ON CONFLICT (discord_id) DO UPDATE SET role = 'super_admin'
        RETURNING id
        ",
    )
    .bind(discord_id)
    .fetch_one(db_pool)
    .await;

    match result {
        Ok(user_id) => {
            tracing::info!(discord_id = discord_id, user_id = %user_id, "Super admin bootstrapped");

            // FR-AUTH-008: Create dummy profile if none exists
            if let Err(e) = sqlx::query(
                r"
                INSERT INTO profiles (user_id, nickname, is_public, updated_at)
                VALUES ($1, 'SuperAdmin', false, NOW())
                ON CONFLICT (user_id) DO NOTHING
                ",
            )
            .bind(user_id)
            .execute(db_pool)
            .await
            {
                tracing::warn!(error = %e, "Failed to bootstrap super admin profile");
            }
        }
        Err(e) => tracing::warn!(error = %e, "Failed to bootstrap super admin"),
    }
}
