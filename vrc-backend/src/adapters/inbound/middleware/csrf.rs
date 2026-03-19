use axum::body::Body;
use axum::http::{HeaderValue, Method, Request, Response, StatusCode, header};
use serde_json::json;
use tower::{Layer, Service};

/// CSRF protection layer using Origin header validation.
///
/// Only applied to the Internal API layer (cookie-authenticated endpoints).
/// System API endpoints use Bearer tokens and are exempt.
///
/// Safe methods (GET, HEAD, OPTIONS) are always allowed through.
/// State-changing methods (POST, PUT, PATCH, DELETE) must include an `Origin`
/// header matching the configured frontend origin.
#[derive(Clone)]
pub struct CsrfLayer {
    allowed_origin: String,
}

impl CsrfLayer {
    pub fn new(allowed_origin: &str) -> Self {
        Self {
            allowed_origin: allowed_origin.to_owned(),
        }
    }
}

impl<S> Layer<S> for CsrfLayer {
    type Service = CsrfMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CsrfMiddleware {
            inner,
            allowed_origin: self.allowed_origin.clone(),
        }
    }
}

#[derive(Clone)]
pub struct CsrfMiddleware<S> {
    inner: S,
    allowed_origin: String,
}

impl<S> Service<Request<Body>> for CsrfMiddleware<S>
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
        let method = req.method().clone();
        let mut inner = self.inner.clone();

        // Safe methods bypass CSRF check
        if method == Method::GET || method == Method::HEAD || method == Method::OPTIONS {
            return Box::pin(async move { inner.call(req).await });
        }

        // State-changing methods require Origin or Referer header matching frontend.
        // Origin is preferred (explicitly set by browsers for cross-origin requests).
        // Referer is used as fallback because some privacy extensions strip Origin.
        let origin = req.headers().get(header::ORIGIN).cloned();
        let referer = req.headers().get(header::REFERER).cloned();
        let allowed = self.allowed_origin.clone();

        Box::pin(async move {
            let origin_matches = origin
                .as_ref()
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| value == allowed);

            // Fall back to Referer only when Origin is absent. If Origin is present but
            // mismatched, reject the request per the security contract.
            let referer_matches = if origin.is_none() {
                referer
                    .as_ref()
                    .and_then(|r| r.to_str().ok())
                    .is_some_and(|r| extract_origin_from_url(r) == allowed)
            } else {
                false
            };

            if origin_matches || referer_matches {
                return inner.call(req).await;
            }

            tracing::warn!(
                origin = ?origin,
                referer = ?referer,
                expected = ?allowed,
                "CSRF check failed: neither Origin nor Referer matched"
            );

            let body = json!({
                "error": "ERR-CSRF-001",
                "message": "CSRF verification failed.",
                "details": null,
            });

            let response_body = match serde_json::to_vec(&body) {
                Ok(bytes) => Body::from(bytes),
                Err(error) => {
                    tracing::error!(error = %error, "Failed to serialize CSRF rejection body");
                    Body::from(
                        r#"{"error":"ERR-INTERNAL","message":"Internal server error","details":null}"#,
                    )
                }
            };

            let mut response = Response::new(response_body);
            *response.status_mut() = StatusCode::FORBIDDEN;
            response
                .headers_mut()
                .insert("content-type", HeaderValue::from_static("application/json"));
            Ok(response)
        })
    }
}

/// Extract the origin (scheme + host + optional port) from a full URL.
///
/// Returns a `&str` slice of the input to avoid allocation.
/// For example, `"https://example.com/path?q=1"` → `"https://example.com"`.
/// Returns the input unchanged if parsing fails, ensuring the comparison
/// will safely reject rather than accidentally match.
#[inline]
fn extract_origin_from_url(url: &str) -> &str {
    // Find the third slash which separates origin from path: "https://host/..."
    let after_scheme = url.find("://").map_or(0, |i| i + 3);
    let path_start = url[after_scheme..].find('/').map(|i| i + after_scheme);
    match path_start {
        Some(i) => &url[..i],
        None => url,
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use super::*;
    use tower::ServiceExt;
    use tower::service_fn;

    // Spec refs: application-security.md "CSRF Protection" and internal-api.md.
    // Coverage: safe-method bypass, Origin match, Referer fallback, and rejection body contract.

    fn build_service() -> impl Service<
        Request<Body>,
        Response = Response<Body>,
        Error = Infallible,
        Future = impl Send,
    > + Clone {
        CsrfLayer::new("https://frontend.example")
            .layer(service_fn(|request: Request<Body>| async move {
                let mut response = Response::new(Body::from(request.method().to_string()));
                *response.status_mut() = StatusCode::NO_CONTENT;
                Ok::<_, Infallible>(response)
            }))
    }

    async fn rejection_json(response: Response<Body>) -> serde_json::Value {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body must be readable");
        serde_json::from_slice(&bytes).expect("response body must be valid json")
    }

    #[test]
    fn test_extract_origin_from_url_strips_path_query_and_fragment() {
        let origin = extract_origin_from_url("https://frontend.example/path?q=1#frag");

        assert_eq!(origin, "https://frontend.example");
    }

    #[test]
    fn test_extract_origin_from_url_returns_origin_without_path() {
        let origin = extract_origin_from_url("https://frontend.example:8443");

        assert_eq!(origin, "https://frontend.example:8443");
    }

    #[test]
    fn test_extract_origin_from_url_returns_input_when_parse_fails() {
        let origin = extract_origin_from_url("not-a-valid-url");

        assert_eq!(origin, "not-a-valid-url");
    }

    #[tokio::test]
    async fn test_safe_get_method_bypasses_csrf_checks() {
        let response = build_service()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/v1/internal/auth/me")
                    .body(Body::empty())
                    .expect("request must build"),
            )
            .await
            .expect("service must not fail");

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_post_allows_matching_origin() {
        let response = build_service()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/internal/reports")
                    .header(header::ORIGIN, "https://frontend.example")
                    .body(Body::empty())
                    .expect("request must build"),
            )
            .await
            .expect("service must not fail");

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_post_allows_matching_referer_when_origin_is_absent() {
        let response = build_service()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/internal/reports")
                    .header(header::REFERER, "https://frontend.example/settings/profile")
                    .body(Body::empty())
                    .expect("request must build"),
            )
            .await
            .expect("service must not fail");

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_post_rejects_missing_origin_and_referer() {
        let response = build_service()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/internal/reports")
                    .body(Body::empty())
                    .expect("request must build"),
            )
            .await
            .expect("service must not fail");

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("application/json")
        );

        let body = rejection_json(response).await;
        assert_eq!(body["error"], "ERR-CSRF-001");
        assert_eq!(body["details"], serde_json::Value::Null);
    }

    #[tokio::test]
    async fn test_post_rejects_mismatched_origin_even_when_referer_matches() {
        let response = build_service()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/internal/reports")
                    .header(header::ORIGIN, "https://evil.example")
                    .header(header::REFERER, "https://frontend.example/settings/profile")
                    .body(Body::empty())
                    .expect("request must build"),
            )
            .await
            .expect("service must not fail");

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_post_rejects_mismatched_referer() {
        let response = build_service()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/internal/reports")
                    .header(header::REFERER, "https://evil.example/settings/profile")
                    .body(Body::empty())
                    .expect("request must build"),
            )
            .await
            .expect("service must not fail");

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
