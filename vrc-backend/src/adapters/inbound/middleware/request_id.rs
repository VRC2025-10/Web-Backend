use axum::body::Body;
use axum::http::{HeaderValue, Request, Response};
use tower::{Layer, Service};
use ulid::Ulid;

/// Middleware that generates a ULID request ID for every request.
///
/// The request ID is:
/// 1. Injected into a tracing span (for structured logging)
/// 2. Added as `x-request-id` response header (for client correlation)
#[derive(Clone)]
pub struct RequestIdLayer;

impl<S> Layer<S> for RequestIdLayer {
    type Service = RequestIdMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequestIdMiddleware { inner }
    }
}

#[derive(Clone)]
pub struct RequestIdMiddleware<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for RequestIdMiddleware<S>
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
        let request_id = Ulid::new().to_string();
        let method = req.method().clone();
        let path = req.uri().path().to_owned();
        let mut inner = self.inner.clone();
        let id_for_header = request_id.clone();

        Box::pin(async move {
            let span = tracing::info_span!(
                "http_request",
                request_id = %request_id,
                method = %method,
                path = %path,
            );
            let _enter = span.enter();

            let result = inner.call(req).await;

            match result {
                Ok(mut response) => {
                    if let Ok(val) = HeaderValue::from_str(&id_for_header) {
                        response
                            .headers_mut()
                            .insert("x-request-id", val);
                    }
                    Ok(response)
                }
                Err(e) => Err(e),
            }
        })
    }
}
