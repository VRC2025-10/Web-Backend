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
/// Writes directly into a pre-allocated String to minimize allocations.
fn normalize_path(path: &str) -> String {
    // Fast path: most API paths are short and have no IDs
    let mut result = String::with_capacity(path.len());
    let mut first = true;

    for segment in path.split('/') {
        if !first {
            result.push('/');
        }
        first = false;

        if is_uuid_like(segment)
            || (!segment.is_empty()
                && segment.len() <= 20
                && segment.as_bytes().iter().all(u8::is_ascii_digit))
        {
            result.push_str(":id");
        } else {
            result.push_str(segment);
        }
    }
    result
}

/// Check whether a segment looks like a UUID (with or without hyphens).
/// Uses byte-level checks throughout to avoid UTF-8 overhead.
#[inline]
fn is_uuid_like(s: &str) -> bool {
    let bytes = s.as_bytes();
    match bytes.len() {
        // UUID with hyphens: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
        36 => {
            bytes[8] == b'-'
                && bytes[13] == b'-'
                && bytes[18] == b'-'
                && bytes[23] == b'-'
                && bytes
                    .iter()
                    .enumerate()
                    .all(|(i, &b)| i == 8 || i == 13 || i == 18 || i == 23 || b.is_ascii_hexdigit())
        }
        // UUID without hyphens: 32 hex digits
        32 => bytes.iter().all(u8::is_ascii_hexdigit),
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
        // Use static str for method to avoid per-request allocation
        let method: &'static str = match *req.method() {
            axum::http::Method::GET => "GET",
            axum::http::Method::POST => "POST",
            axum::http::Method::PUT => "PUT",
            axum::http::Method::PATCH => "PATCH",
            axum::http::Method::DELETE => "DELETE",
            axum::http::Method::HEAD => "HEAD",
            axum::http::Method::OPTIONS => "OPTIONS",
            _ => "OTHER",
        };
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
                    // Use itoa-style formatting to avoid allocation for status code
                    let status_code = response.status().as_u16();
                    let mut status_buf = itoa::Buffer::new();
                    let status = status_buf.format(status_code);
                    metrics::counter!(
                        "http_requests_total",
                        "method" => method,
                        "path" => path.clone(),
                        "status" => status.to_owned(),
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
                        "method" => method,
                        "path" => path,
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
