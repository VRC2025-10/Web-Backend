use std::num::NonZeroU32;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{HeaderValue, Request, Response, StatusCode};
use governor::clock::DefaultClock;
use governor::state::keyed::DashMapStateStore;
use governor::{Quota, RateLimiter};
use serde_json::json;
use sha2::{Digest, Sha256};
use tower::{Layer, Service};

/// Key extraction strategy for rate limiting.
#[derive(Debug, Clone)]
pub enum KeyExtractor {
    /// Extract IP from X-Forwarded-For (rightmost) or peer address.
    PerIp,
    /// Extract session `user_id` if available, otherwise fall back to `PerIp`.
    PerUserOrIp,
    /// Single bucket for all requests.
    Global,
}

pub type SharedRateLimiter = Arc<RateLimiter<String, DashMapStateStore<String>, DefaultClock>>;

/// Configuration for one rate-limiting tier.
#[derive(Clone)]
pub struct RateLimitConfig {
    pub layer: &'static str,
    pub requests_per_minute: u32,
    pub burst: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum RateLimitConfigError {
    #[error("requests_per_minute must be greater than zero")]
    ZeroRequestsPerMinute,
    #[error("burst must be greater than zero")]
    ZeroBurst,
}

/// Build a [`SharedRateLimiter`] from a [`RateLimitConfig`].
pub fn build_limiter(cfg: &RateLimitConfig) -> Result<SharedRateLimiter, RateLimitConfigError> {
    let requests_per_minute = NonZeroU32::new(cfg.requests_per_minute)
        .ok_or(RateLimitConfigError::ZeroRequestsPerMinute)?;
    let burst = NonZeroU32::new(cfg.burst).ok_or(RateLimitConfigError::ZeroBurst)?;
    let quota = Quota::per_minute(requests_per_minute).allow_burst(burst);

    Ok(Arc::new(RateLimiter::dashmap(quota)))
}

/// Predefined tier configurations per the spec.
pub fn public_tier() -> RateLimitConfig {
    RateLimitConfig {
        layer: "public",
        requests_per_minute: 60,
        burst: 10,
    }
}

pub fn internal_tier() -> RateLimitConfig {
    RateLimitConfig {
        layer: "internal",
        requests_per_minute: 120,
        burst: 20,
    }
}

pub fn system_tier() -> RateLimitConfig {
    RateLimitConfig {
        layer: "system",
        requests_per_minute: 30,
        burst: 5,
    }
}

pub fn auth_tier() -> RateLimitConfig {
    RateLimitConfig {
        layer: "auth",
        requests_per_minute: 10,
        burst: 3,
    }
}

// ---------- Layer ----------

#[derive(Clone)]
pub struct RateLimitLayer {
    limiter: SharedRateLimiter,
    key_extractor: KeyExtractor,
    layer_name: &'static str,
    trust_xff: bool,
}

impl RateLimitLayer {
    pub fn new(
        limiter: SharedRateLimiter,
        key_extractor: KeyExtractor,
        layer_name: &'static str,
        trust_xff: bool,
    ) -> Self {
        Self {
            limiter,
            key_extractor,
            layer_name,
            trust_xff,
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
            layer_name: self.layer_name,
            trust_xff: self.trust_xff,
        }
    }
}

// ---------- Middleware service ----------

#[derive(Clone)]
pub struct RateLimitMiddleware<S> {
    inner: S,
    limiter: SharedRateLimiter,
    key_extractor: KeyExtractor,
    layer_name: &'static str,
    trust_xff: bool,
}

/// Extract the client IP from request headers or peer address.
/// When behind a trusted reverse proxy (`trust_xff` is true), use the
/// **rightmost** X-Forwarded-For entry (the one added by the trusted proxy
/// closest to us). Otherwise, always use the TCP peer address.
fn extract_ip(req: &Request<Body>, trust_xff: bool) -> String {
    // Only trust X-Forwarded-For when explicitly configured
    if trust_xff
        && let Some(xff) = req.headers().get("x-forwarded-for")
        && let Ok(value) = xff.to_str()
        && let Some(last) = value.rsplit(',').next()
    {
        let trimmed = last.trim();
        if let Ok(ip) = trimmed.parse::<std::net::IpAddr>() {
            return ip.to_string();
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
fn extract_key(req: &Request<Body>, strategy: &KeyExtractor, trust_xff: bool) -> String {
    match strategy {
        KeyExtractor::PerIp => extract_ip(req, trust_xff),
        KeyExtractor::PerUserOrIp => {
            // Use a deterministic hash-derived key so rate-limit logging never exposes
            // any raw session token material.
            if let Some(cookie_header) = req.headers().get("cookie")
                && let Ok(value) = cookie_header.to_str()
            {
                for pair in value.split(';') {
                    let trimmed = pair.trim();
                    if let Some(token) = trimmed.strip_prefix("session_id=")
                        && !token.is_empty()
                    {
                        let digest = Sha256::digest(token.as_bytes());
                        let key_prefix = hex::encode(&digest[..8]);
                        return format!("user:{key_prefix}");
                    }
                }
            }
            // Fall back to IP-based keying for unauthenticated requests
            extract_ip(req, trust_xff)
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
        let key = extract_key(&req, &self.key_extractor, self.trust_xff);
        let limiter = self.limiter.clone();
        let mut inner = self.inner.clone();
        let layer_name = self.layer_name;

        Box::pin(async move {
            match limiter.check_key(&key) {
                Ok(()) => inner.call(req).await,
                Err(not_until) => {
                    let wait_secs = not_until
                        .wait_time_from(governor::clock::DefaultClock::default().now())
                        .as_secs()
                        + 1;

                    metrics::counter!("rate_limit_rejections_total", "layer" => layer_name)
                        .increment(1);

                    tracing::warn!(
                        layer = layer_name,
                        key = %key,
                        retry_after_secs = wait_secs,
                        "Rate limit exceeded"
                    );

                    let body = json!({
                        "error": "ERR-RATELIMIT-001",
                        "message": "Rate limit exceeded. Please retry after the indicated time.",
                        "details": null,
                    });

                    let response_body = match serde_json::to_vec(&body) {
                        Ok(bytes) => Body::from(bytes),
                        Err(error) => {
                            tracing::error!(
                                error = %error,
                                layer = layer_name,
                                "Failed to serialize rate-limit rejection body"
                            );
                            Body::from(
                                r#"{"error":"ERR-INTERNAL","message":"Internal server error","details":null}"#,
                            )
                        }
                    };

                    let mut response = Response::new(response_body);
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

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    use super::*;
    use axum::body::to_bytes;
    use sha2::{Digest, Sha256};
    use tower::ServiceExt;
    use tower::service_fn;

    // Spec refs: rate-limiting.md "Per-Layer Configuration" and "IP Extraction Security".
    // Coverage: limiter construction, IP extraction, key extraction, and 429 response contract.

    fn request_with_peer() -> Request<Body> {
        let mut request = Request::builder()
            .method("GET")
            .uri("/api/v1/public/events")
            .body(Body::empty())
            .expect("request must build");
        request.extensions_mut().insert(ConnectInfo(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 9)),
            4242,
        )));
        request
    }

    fn expected_session_bucket(token: &str) -> String {
        let digest = Sha256::digest(token.as_bytes());
        format!("user:{}", hex::encode(&digest[..8]))
    }

    #[test]
    fn test_build_limiter_rejects_zero_requests_per_minute() {
        let error = build_limiter(&RateLimitConfig {
            layer: "test",
            requests_per_minute: 0,
            burst: 1,
        })
        .expect_err("zero requests_per_minute must fail");

        assert!(matches!(error, RateLimitConfigError::ZeroRequestsPerMinute));
    }

    #[test]
    fn test_build_limiter_rejects_zero_burst() {
        let error = build_limiter(&RateLimitConfig {
            layer: "test",
            requests_per_minute: 1,
            burst: 0,
        })
        .expect_err("zero burst must fail");

        assert!(matches!(error, RateLimitConfigError::ZeroBurst));
    }

    #[test]
    fn test_extract_ip_uses_peer_address_when_xff_is_not_trusted() {
        let mut request = request_with_peer();
        request
            .headers_mut()
            .insert("x-forwarded-for", HeaderValue::from_static("10.0.0.1, 198.51.100.5"));

        let ip = extract_ip(&request, false);

        assert_eq!(ip, "127.0.0.9");
    }

    #[test]
    fn test_extract_ip_uses_rightmost_xff_entry_when_trusted() {
        let mut request = request_with_peer();
        request
            .headers_mut()
            .insert("x-forwarded-for", HeaderValue::from_static("10.0.0.1, 198.51.100.5"));

        let ip = extract_ip(&request, true);

        assert_eq!(ip, "198.51.100.5");
    }

    #[test]
    fn test_extract_ip_falls_back_to_peer_when_trusted_header_is_empty() {
        let mut request = request_with_peer();
        request
            .headers_mut()
            .insert("x-forwarded-for", HeaderValue::from_static("10.0.0.1,   "));

        let ip = extract_ip(&request, true);

        assert_eq!(ip, "127.0.0.9");
    }

    #[test]
    fn test_extract_ip_falls_back_to_peer_when_trusted_header_is_invalid() {
        let mut request = request_with_peer();
        request
            .headers_mut()
            .insert("x-forwarded-for", HeaderValue::from_static("10.0.0.1, definitely-not-an-ip"));

        let ip = extract_ip(&request, true);

        assert_eq!(ip, "127.0.0.9");
    }

    #[test]
    fn test_extract_key_hashes_session_token_for_internal_requests() {
        let mut request = request_with_peer();
        request.headers_mut().insert(
            "cookie",
            HeaderValue::from_static("theme=dark; session_id=abcdefghijklmnopqrstuv; other=1"),
        );

        let key = extract_key(&request, &KeyExtractor::PerUserOrIp, false);

        assert_eq!(key, expected_session_bucket("abcdefghijklmnopqrstuv"));
    }

    #[test]
    fn test_extract_key_hashes_short_session_token() {
        let mut request = request_with_peer();
        request
            .headers_mut()
            .insert("cookie", HeaderValue::from_static("session_id=shorttoken"));

        let key = extract_key(&request, &KeyExtractor::PerUserOrIp, false);

        assert_eq!(key, expected_session_bucket("shorttoken"));
    }

    #[test]
    fn test_extract_key_falls_back_to_ip_when_session_cookie_is_missing() {
        let request = request_with_peer();

        let key = extract_key(&request, &KeyExtractor::PerUserOrIp, false);

        assert_eq!(key, "127.0.0.9");
    }

    #[test]
    fn test_extract_key_returns_global_bucket_for_global_strategy() {
        let request = request_with_peer();

        let key = extract_key(&request, &KeyExtractor::Global, false);

        assert_eq!(key, "global");
    }

    #[tokio::test]
    async fn test_rate_limit_middleware_allows_first_request() {
        let limiter = build_limiter(&RateLimitConfig {
            layer: "test",
            requests_per_minute: 1,
            burst: 1,
        })
        .expect("limiter config must be valid");
        let service = RateLimitLayer::new(limiter, KeyExtractor::PerIp, "test", false).layer(
            service_fn(|_: Request<Body>| async move {
                Ok::<_, Infallible>(Response::builder()
                    .status(StatusCode::NO_CONTENT)
                    .body(Body::empty())
                    .expect("response must build"))
            }),
        );

        let response = service
            .oneshot(request_with_peer())
            .await
            .expect("service must not fail");

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_rate_limit_middleware_rejects_second_request_with_contract_response() {
        let limiter = build_limiter(&RateLimitConfig {
            layer: "test",
            requests_per_minute: 1,
            burst: 1,
        })
        .expect("limiter config must be valid");
        let service = RateLimitLayer::new(limiter, KeyExtractor::PerIp, "test", false).layer(
            service_fn(|_: Request<Body>| async move {
                Ok::<_, Infallible>(Response::builder()
                    .status(StatusCode::NO_CONTENT)
                    .body(Body::empty())
                    .expect("response must build"))
            }),
        );

        let _ = service
            .clone()
            .oneshot(request_with_peer())
            .await
            .expect("first request must pass");
        let response = service
            .oneshot(request_with_peer())
            .await
            .expect("service must not fail");

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(
            response
                .headers()
                .get("retry-after")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok())
                .is_some_and(|seconds| seconds >= 1)
        );

        let payload = serde_json::from_slice::<serde_json::Value>(
            &to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("response body must be readable"),
        )
        .expect("response body must be valid json");
        assert_eq!(payload["error"], "ERR-RATELIMIT-001");
        assert_eq!(payload["details"], serde_json::Value::Null);
    }
}
