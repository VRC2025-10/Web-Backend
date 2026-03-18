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

        // State-changing methods require Origin header matching frontend
        let origin = req.headers().get(header::ORIGIN).cloned();
        let allowed = self.allowed_origin.clone();

        Box::pin(async move {
            match origin {
                Some(ref o) if o == &allowed => inner.call(req).await,
                _ => {
                    tracing::warn!(
                        origin = ?origin,
                        expected = ?allowed,
                        "CSRF check failed: Origin header missing or mismatched"
                    );

                    let body = json!({
                        "error": "ERR-CSRF-001",
                        "message": "CSRF verification failed.",
                        "details": null,
                    });

                    let mut response = Response::new(Body::from(
                        serde_json::to_string(&body).unwrap_or_default(),
                    ));
                    *response.status_mut() = StatusCode::FORBIDDEN;
                    response.headers_mut().insert(
                        "content-type",
                        HeaderValue::from_static("application/json"),
                    );
                    Ok(response)
                }
            }
        })
    }
}
