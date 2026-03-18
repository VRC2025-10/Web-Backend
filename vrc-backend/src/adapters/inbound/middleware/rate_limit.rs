use std::num::NonZeroU32;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{HeaderValue, Request, Response, StatusCode};
use governor::clock::DefaultClock;
use governor::state::keyed::DashMapStateStore;
use governor::{Quota, RateLimiter};
use serde_json::json;
use tower::{Layer, Service};

/// Key extraction strategy for rate limiting.
#[derive(Debug, Clone)]
pub enum KeyExtractor {
    /// Extract IP from X-Forwarded-For (rightmost) or peer address.
    PerIp,
    /// Extract session user_id if available, otherwise fall back to `PerIp`.
    PerUserOrIp,
    /// Single bucket for all requests.
    Global,
}

pub type SharedRateLimiter = Arc<RateLimiter<String, DashMapStateStore<String>, DefaultClock>>;

/// Configuration for one rate-limiting tier.
#[derive(Clone)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub burst: u32,
}

/// Build a [`SharedRateLimiter`] from a [`RateLimitConfig`].
pub fn build_limiter(cfg: &RateLimitConfig) -> SharedRateLimiter {
    let quota = Quota::per_minute(NonZeroU32::new(cfg.requests_per_minute).expect("non-zero"))
        .allow_burst(NonZeroU32::new(cfg.burst).expect("non-zero"));

    Arc::new(RateLimiter::dashmap(quota))
}

/// Predefined tier configurations per the spec.
pub fn public_tier() -> RateLimitConfig {
    RateLimitConfig {
        requests_per_minute: 60,
        burst: 10,
    }
}

pub fn internal_tier() -> RateLimitConfig {
    RateLimitConfig {
        requests_per_minute: 120,
        burst: 20,
    }
}

pub fn system_tier() -> RateLimitConfig {
    RateLimitConfig {
        requests_per_minute: 30,
        burst: 5,
    }
}

pub fn auth_tier() -> RateLimitConfig {
    RateLimitConfig {
        requests_per_minute: 10,
        burst: 3,
    }
}

// ---------- Layer ----------

#[derive(Clone)]
pub struct RateLimitLayer {
    limiter: SharedRateLimiter,
    key_extractor: KeyExtractor,
}

impl RateLimitLayer {
    pub fn new(limiter: SharedRateLimiter, key_extractor: KeyExtractor) -> Self {
        Self {
            limiter,
            key_extractor,
        }
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitMiddleware {
            inner,
            limiter: self.limiter.clone(),
            key_extractor: self.key_extractor.clone(),
        }
    }
}

// ---------- Middleware service ----------

#[derive(Clone)]
pub struct RateLimitMiddleware<S> {
    inner: S,
    limiter: SharedRateLimiter,
    key_extractor: KeyExtractor,
}

/// Extract the client IP from request headers or peer address.
/// When behind a reverse proxy, use the **rightmost** X-Forwarded-For entry
/// (the one added by the trusted proxy closest to us).
fn extract_ip(req: &Request<Body>) -> String {
    // Try X-Forwarded-For (rightmost entry)
    if let Some(xff) = req.headers().get("x-forwarded-for") {
        if let Ok(value) = xff.to_str() {
            if let Some(last) = value.rsplit(',').next() {
                let trimmed = last.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_owned();
                }
            }
        }
    }

    // Fallback: extension set by hyper ConnectInfo
    if let Some(connect_info) = req.extensions().get::<ConnectInfo<std::net::SocketAddr>>() {
        return connect_info.0.ip().to_string();
    }

    // Last resort
    "unknown".to_owned()
}

/// Extract the rate limit key depending on the strategy.
fn extract_key(req: &Request<Body>, strategy: &KeyExtractor) -> String {
    match strategy {
        KeyExtractor::PerIp => extract_ip(req),
        KeyExtractor::PerUserOrIp => {
            // Try to extract user_id from session cookie via SHA-256 hash fingerprint.
            // This provides per-user keying for authenticated requests without
            // performing a full DB lookup — we use the cookie value hash as a stable key.
            if let Some(cookie_header) = req.headers().get("cookie") {
                if let Ok(value) = cookie_header.to_str() {
                    for pair in value.split(';') {
                        let trimmed = pair.trim();
                        if let Some(token) = trimmed.strip_prefix("session_id=") {
                            if !token.is_empty() {
                                use sha2::{Digest, Sha256};
                                let mut hasher = Sha256::new();
                                hasher.update(token.as_bytes());
                                let hash = hasher.finalize();
                                return format!("user:{}", hex::encode(&hash[..8]));
                            }
                        }
                    }
                }
            }
            // Fall back to IP-based keying for unauthenticated requests
            extract_ip(req)
        }
        KeyExtractor::Global => "global".to_owned(),
    }
}

impl<S> Service<Request<Body>> for RateLimitMiddleware<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let key = extract_key(&req, &self.key_extractor);
        let limiter = self.limiter.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            match limiter.check_key(&key) {
                Ok(_) => inner.call(req).await,
                Err(not_until) => {
                    let wait_secs = not_until
                        .wait_time_from(governor::clock::DefaultClock::default().now())
                        .as_secs()
                        + 1;

                    metrics::counter!("rate_limit_rejections_total", "layer" => "unknown")
                        .increment(1);

                    tracing::warn!(
                        key = %key,
                        retry_after_secs = wait_secs,
                        "Rate limit exceeded"
                    );

                    let body = json!({
                        "error": "ERR-RATELIMIT-001",
                        "message": "Rate limit exceeded. Please retry after the indicated time.",
                        "details": null,
                    });

                    let mut response =
                        Response::new(Body::from(serde_json::to_string(&body).unwrap_or_default()));
                    *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
                    response.headers_mut().insert(
                        "retry-after",
                        HeaderValue::from_str(&wait_secs.to_string())
                            .unwrap_or_else(|_| HeaderValue::from_static("5")),
                    );
                    response
                        .headers_mut()
                        .insert("content-type", HeaderValue::from_static("application/json"));

                    Ok(response)
                }
            }
        })
    }
}

use governor::clock::Clock;
