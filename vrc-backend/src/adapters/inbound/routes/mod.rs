pub mod admin;
pub mod auth;
pub mod health;
pub mod internal;
pub mod public;
pub mod system;

use std::sync::Arc;

use axum::Router;
use tower_http::trace::TraceLayer;

use crate::AppState;

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .merge(health::routes())
        .nest("/api/v1/auth/discord", auth::routes())
        .nest("/api/v1/internal", internal::routes())
        .nest("/api/v1/internal/admin", admin::routes())
        .nest("/api/v1/public", public::routes())
        .nest("/api/v1/system", system::routes())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
