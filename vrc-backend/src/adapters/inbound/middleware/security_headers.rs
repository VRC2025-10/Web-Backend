use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use axum::http::{HeaderValue, Response, header};
use tower::{Layer, Service};

/// Single middleware layer that sets all security headers in one pass.
///
/// More efficient than 6 separate `SetResponseHeaderLayer::overriding` layers
/// because it avoids 6 separate `Service` wrappings and 6 separate poll cycles.
/// In production, Caddy also sets these — this is defense-in-depth.
#[derive(Clone)]
pub struct SecurityHeadersLayer;

impl<S> Layer<S> for SecurityHeadersLayer {
    type Service = SecurityHeadersService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        SecurityHeadersService { inner }
    }
}

#[derive(Clone)]
pub struct SecurityHeadersService<S> {
    inner: S,
}

impl<S, ReqBody, ResBody> Service<axum::http::Request<ReqBody>> for SecurityHeadersService<S>
where
    S: Service<axum::http::Request<ReqBody>, Response = Response<ResBody>>,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: axum::http::Request<ReqBody>) -> Self::Future {
        let future = self.inner.call(req);
        Box::pin(async move {
            let mut response = future.await?;
            let headers = response.headers_mut();
            headers.insert(
                header::HeaderName::from_static("x-content-type-options"),
                HeaderValue::from_static("nosniff"),
            );
            headers.insert(
                header::HeaderName::from_static("x-frame-options"),
                HeaderValue::from_static("DENY"),
            );
            headers.insert(
                header::HeaderName::from_static("referrer-policy"),
                HeaderValue::from_static("strict-origin-when-cross-origin"),
            );
            headers.insert(
                header::HeaderName::from_static("permissions-policy"),
                HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
            );
            headers.insert(
                header::HeaderName::from_static("content-security-policy"),
                HeaderValue::from_static("default-src 'none'; frame-ancestors 'none'"),
            );
            headers.insert(
                header::HeaderName::from_static("strict-transport-security"),
                HeaderValue::from_static("max-age=63072000; includeSubDomains; preload"),
            );
            Ok(response)
        })
    }
}
