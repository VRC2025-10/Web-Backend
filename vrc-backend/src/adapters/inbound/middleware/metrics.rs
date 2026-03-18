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
/// Replaces UUID path segments and numeric IDs with `:id`.
fn normalize_path(path: &str) -> String {
    path.split('/')
        .map(|segment| {
            // Replace UUID-like segments or pure-numeric IDs with :id
            // to avoid high-cardinality label explosion in Prometheus
            if is_uuid_like(segment)
                || (!segment.is_empty()
                    && segment.len() <= 20
                    && segment.chars().all(|c| c.is_ascii_digit()))
            {
                ":id"
            } else {
                segment
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Check whether a segment looks like a UUID (with or without hyphens).
fn is_uuid_like(s: &str) -> bool {
    match s.len() {
        // UUID with hyphens: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
        36 => {
            s.as_bytes().iter().enumerate().all(|(i, &b)| {
                if i == 8 || i == 13 || i == 18 || i == 23 {
                    b == b'-'
                } else {
                    b.is_ascii_hexdigit()
                }
            })
        }
        // UUID without hyphens: 32 hex digits
        32 => s.bytes().all(|b| b.is_ascii_hexdigit()),
        _ => false,
    }
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
