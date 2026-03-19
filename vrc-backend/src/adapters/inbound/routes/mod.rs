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
use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderName, Method, header, HeaderValue};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;

use crate::AppState;
use crate::adapters::inbound::middleware::csrf::CsrfLayer;
use crate::adapters::inbound::middleware::metrics::MetricsLayer;
use crate::adapters::inbound::middleware::rate_limit::{
    KeyExtractor, RateLimitConfigError, RateLimitLayer, auth_tier, build_limiter, internal_tier,
    public_tier, system_tier,
};
use crate::adapters::inbound::middleware::request_id::RequestIdLayer;
use crate::adapters::inbound::middleware::security_headers::SecurityHeadersLayer;

#[derive(Debug, thiserror::Error)]
pub enum RouteBuildError {
    #[error("Failed to build {layer} rate limiter: {source}")]
    RateLimiter {
        layer: &'static str,
        #[source]
        source: RateLimitConfigError,
    },
}

pub fn build_router(state: Arc<AppState>) -> Result<Router, RouteBuildError> {
    const X_TOTAL_COUNT: HeaderName = HeaderName::from_static("x-total-count");
    const X_TOTAL_PAGES: HeaderName = HeaderName::from_static("x-total-pages");

    let frontend_origin = state.config.frontend_origin.clone();

    // CORS — single origin, credentials allowed (cookie-based auth)
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::exact(
            state.config.frontend_origin_header.clone(),
        ))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT, header::ORIGIN])
        .expose_headers([X_TOTAL_COUNT, X_TOTAL_PAGES])
        .allow_credentials(true)
        .max_age(Duration::from_secs(3600));

    // Rate limiters per tier
    let public_cfg = public_tier();
    let internal_cfg = internal_tier();
    let system_cfg = system_tier();
    let auth_cfg = auth_tier();

    let public_limiter =
        build_limiter(&public_cfg).map_err(|source| RouteBuildError::RateLimiter {
            layer: public_cfg.layer,
            source,
        })?;
    let internal_limiter =
        build_limiter(&internal_cfg).map_err(|source| RouteBuildError::RateLimiter {
            layer: internal_cfg.layer,
            source,
        })?;
    let system_limiter =
        build_limiter(&system_cfg).map_err(|source| RouteBuildError::RateLimiter {
            layer: system_cfg.layer,
            source,
        })?;
    let auth_limiter = build_limiter(&auth_cfg).map_err(|source| RouteBuildError::RateLimiter {
        layer: auth_cfg.layer,
        source,
    })?;

    // CSRF layer — only for internal (cookie-authenticated) routes
    let csrf = CsrfLayer::new(&frontend_origin);

    let trust_xff = state.config.trust_x_forwarded_for;

    // Per-tier routers with rate limiting applied
    let internal_routes = Router::new()
        .merge(internal::routes())
        .nest("/admin", admin::routes())
        .layer(csrf)
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("private, no-store"),
        ))
        .layer(RateLimitLayer::new(
            internal_limiter,
            KeyExtractor::PerUserOrIp,
            internal_cfg.layer,
            trust_xff,
        ));

    let public_routes = Router::new()
        .merge(public::routes())
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=30, stale-while-revalidate=60"),
        ))
        .layer(RateLimitLayer::new(
            public_limiter,
            KeyExtractor::PerIp,
            public_cfg.layer,
            trust_xff,
        ));

    let system_routes = Router::new()
        .merge(system::routes())
        .layer(RateLimitLayer::new(
            system_limiter,
            KeyExtractor::Global,
            system_cfg.layer,
            trust_xff,
        ));

    let auth_routes = Router::new()
        .merge(auth::routes())
        .layer(RateLimitLayer::new(
            auth_limiter,
            KeyExtractor::PerIp,
            auth_cfg.layer,
            trust_xff,
        ));

    let router = Router::new()
        .merge(health::routes())
        .merge(metrics_endpoint::routes())
        .nest("/api/v1/auth/discord", auth_routes)
        .nest("/api/v1/internal", internal_routes)
        .nest("/api/v1/public", public_routes)
        .nest("/api/v1/system", system_routes)
        // Global layers applied to all routes (outermost first in execution)
        .layer(DefaultBodyLimit::max(1_048_576)) // 1 MB request body limit
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .layer(MetricsLayer)
        .layer(RequestIdLayer)
        // Security headers: single layer instead of 6 separate SetResponseHeaderLayer wrappers
        .layer(SecurityHeadersLayer)
        .with_state(state);

    Ok(router)
}
