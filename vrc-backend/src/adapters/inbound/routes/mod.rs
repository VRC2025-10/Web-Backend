pub mod admin;
pub mod auth;
pub mod health;
pub mod internal;
pub mod metrics_endpoint;
pub mod public;
pub mod system;

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::http::{Method, header};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::AppState;
use crate::adapters::inbound::middleware::csrf::CsrfLayer;
use crate::adapters::inbound::middleware::metrics::MetricsLayer;
use crate::adapters::inbound::middleware::rate_limit::{
    KeyExtractor, RateLimitLayer, auth_tier, build_limiter, internal_tier, public_tier, system_tier,
};
use crate::adapters::inbound::middleware::request_id::RequestIdLayer;
use crate::adapters::inbound::middleware::security_headers::apply_security_headers;

pub fn build_router(state: Arc<AppState>) -> Router {
    let frontend_origin = state.config.frontend_origin.clone();

    // CORS — single origin, credentials allowed (cookie-based auth)
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::exact(
            frontend_origin
                .parse()
                .expect("FRONTEND_ORIGIN must be a valid header value"),
        ))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT, header::ORIGIN])
        .allow_credentials(true)
        .max_age(Duration::from_secs(3600));

    // Rate limiters per tier
    let public_limiter = build_limiter(&public_tier());
    let internal_limiter = build_limiter(&internal_tier());
    let system_limiter = build_limiter(&system_tier());
    let auth_limiter = build_limiter(&auth_tier());

    // CSRF layer — only for internal (cookie-authenticated) routes
    let csrf = CsrfLayer::new(&frontend_origin);

    // Per-tier routers with rate limiting applied
    let internal_routes = Router::new()
        .merge(internal::routes())
        .nest("/admin", admin::routes())
        .layer(csrf)
        .layer(RateLimitLayer::new(
            internal_limiter,
            KeyExtractor::PerUserOrIp,
        ));

    let public_routes = Router::new()
        .merge(public::routes())
        .layer(RateLimitLayer::new(public_limiter, KeyExtractor::PerIp));

    let system_routes = Router::new()
        .merge(system::routes())
        .layer(RateLimitLayer::new(system_limiter, KeyExtractor::Global));

    let auth_routes = Router::new()
        .merge(auth::routes())
        .layer(RateLimitLayer::new(auth_limiter, KeyExtractor::PerIp));

    let router = Router::new()
        .merge(health::routes())
        .merge(metrics_endpoint::routes())
        .nest("/api/v1/auth/discord", auth_routes)
        .nest("/api/v1/internal", internal_routes)
        .nest("/api/v1/public", public_routes)
        .nest("/api/v1/system", system_routes)
        // Global layers applied to all routes (outermost first in execution)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .layer(MetricsLayer)
        .layer(RequestIdLayer)
        .with_state(state);

    // Security headers applied last (wraps everything)
    apply_security_headers(router)
}
