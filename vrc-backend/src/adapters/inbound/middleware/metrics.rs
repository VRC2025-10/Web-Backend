use std::time::Instant;

use axum::body::Body;
use axum::http::{Request, Response};
use tower::{Layer, Service};

/// Middleware that records HTTP request metrics using the `metrics` crate.
///
/// Recorded metrics:
/// - `http_requests_total` (counter):  method, path, status
/// - `http_request_duration_seconds` (histogram): method, path
/// - `http_requests_in_flight` (gauge): incremented on entry, decremented on exit
#[derive(Clone)]
pub struct MetricsLayer;

impl<S> Layer<S> for MetricsLayer {
    type Service = MetricsMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MetricsMiddleware { inner }
    }
}

#[derive(Clone)]
pub struct MetricsMiddleware<S> {
    inner: S,
}

/// Normalize path to avoid high-cardinality label explosion.
/// Replaces UUID-like path segments with `:id`.
fn normalize_path(path: &str) -> String {
    path.split('/')
        .map(|segment| {
            // Detect UUID format (with or without hyphens)
            if segment.len() >= 32
                && segment
                    .chars()
                    .all(|c| c.is_ascii_hexdigit() || c == '-')
            {
                ":id"
            } else if segment.parse::<i64>().is_ok() && !segment.is_empty() {
                ":id"
            } else {
                segment
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

impl<S> Service<Request<Body>> for MetricsMiddleware<S>
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
        let method = req.method().to_string();
        let path = normalize_path(req.uri().path());
        let start = Instant::now();

        metrics::gauge!("http_requests_in_flight").increment(1.0);

        let mut inner = self.inner.clone();

        Box::pin(async move {
            let result = inner.call(req).await;

            let duration = start.elapsed().as_secs_f64();
            metrics::gauge!("http_requests_in_flight").decrement(1.0);

            match &result {
                Ok(response) => {
                    let status = response.status().as_u16().to_string();
                    metrics::counter!(
                        "http_requests_total",
                        "method" => method.clone(),
                        "path" => path.clone(),
                        "status" => status,
                    )
                    .increment(1);
                    metrics::histogram!(
                        "http_request_duration_seconds",
                        "method" => method,
                        "path" => path,
                    )
                    .record(duration);
                }
                Err(_) => {
                    metrics::counter!(
                        "http_requests_total",
                        "method" => method.clone(),
                        "path" => path.clone(),
                        "status" => "500",
                    )
                    .increment(1);
                }
            }

            result
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path_replaces_uuids() {
        let path = "/api/v1/internal/admin/users/550e8400-e29b-41d4-a716-446655440000/role";
        assert_eq!(
            normalize_path(path),
            "/api/v1/internal/admin/users/:id/role"
        );
    }

    #[test]
    fn test_normalize_path_replaces_numeric_ids() {
        let path = "/api/v1/internal/admin/users/42/role";
        assert_eq!(
            normalize_path(path),
            "/api/v1/internal/admin/users/:id/role"
        );
    }

    #[test]
    fn test_normalize_path_preserves_static_segments() {
        let path = "/api/v1/internal/auth/me";
        assert_eq!(normalize_path(path), "/api/v1/internal/auth/me");
    }
}
