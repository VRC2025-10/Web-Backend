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
    allowed_origin: HeaderValue,
}

impl CsrfLayer {
    pub fn new(allowed_origin: &str) -> Self {
        Self {
            allowed_origin: HeaderValue::from_str(allowed_origin)
                .expect("FRONTEND_ORIGIN must be a valid header value"),
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
    allowed_origin: HeaderValue,
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
                .is_some_and(|o| o == &allowed);

            // Fall back to Referer: extract scheme+host origin prefix and compare
            let referer_matches = if !origin_matches {
                referer
                    .as_ref()
                    .and_then(|r| r.to_str().ok())
                    .is_some_and(|r| extract_origin_from_url(r) == allowed.to_str().unwrap_or(""))
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

            let mut response =
                Response::new(Body::from(serde_json::to_string(&body).unwrap_or_default()));
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
/// For example, `"https://example.com/path?q=1"` → `"https://example.com"`.
/// Returns the input unchanged if parsing fails, ensuring the comparison
/// will safely reject rather than accidentally match.
fn extract_origin_from_url(url: &str) -> String {
    // Find the third slash which separates origin from path: "https://host/..."
    let after_scheme = url.find("://").map(|i| i + 3).unwrap_or(0);
    let path_start = url[after_scheme..].find('/').map(|i| i + after_scheme);
    match path_start {
        Some(i) => url[..i].to_owned(),
        None => url.to_owned(),
    }
}
