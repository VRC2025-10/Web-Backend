use axum::http::HeaderValue;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Deserializer, Serialize};

/// Raw wire format used only during deserialization.
#[derive(Deserialize)]
struct RawPageRequest {
    #[serde(default = "default_page")]
    page: u32,
    #[serde(default = "default_per_page")]
    per_page: u32,
}

fn default_page() -> u32 {
    1
}
fn default_per_page() -> u32 {
    20
}

/// Pagination parameters that are guaranteed valid after deserialization.
/// `page >= 1` and `1 <= per_page <= 100` — enforced at deserialization time.
/// Invalid values return `400 ERR-VALIDATION` per spec.
#[derive(Debug, Clone)]
pub struct PageRequest {
    page: u32,
    per_page: u32,
}

impl<'de> Deserialize<'de> for PageRequest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = RawPageRequest::deserialize(deserializer)?;
        if raw.page == 0 {
            return Err(serde::de::Error::custom("page must be >= 1"));
        }
        if raw.per_page == 0 || raw.per_page > 100 {
            return Err(serde::de::Error::custom(
                "per_page must be between 1 and 100",
            ));
        }
        Ok(Self {
            page: raw.page,
            per_page: raw.per_page,
        })
    }
}

impl PageRequest {
    /// Construct a validated PageRequest from raw values.
    /// Returns `None` if constraints are violated.
    pub fn new(page: u32, per_page: u32) -> Option<Self> {
        if page == 0 || per_page == 0 || per_page > 100 {
            return None;
        }
        Some(Self { page, per_page })
    }

    pub fn page(&self) -> u32 {
        self.page
    }

    pub fn per_page(&self) -> u32 {
        self.per_page
    }

    pub fn offset(&self) -> i64 {
        i64::from((self.page - 1) * self.per_page)
    }

    pub fn limit(&self) -> i64 {
        i64::from(self.per_page)
    }
}

impl Default for PageRequest {
    fn default() -> Self {
        Self {
            page: 1,
            per_page: 20,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PageResponse<T: Serialize> {
    pub items: Vec<T>,
    pub total_count: i64,
    pub total_pages: i64,
}

impl<T: Serialize> PageResponse<T> {
    pub fn new(items: Vec<T>, total_count: i64, per_page: u32) -> Self {
        let total_pages = if per_page == 0 {
            0
        } else {
            (total_count + i64::from(per_page) - 1) / i64::from(per_page)
        };
        Self {
            items,
            total_count,
            total_pages,
        }
    }
}

/// Implement `IntoResponse` for `PageResponse` to include pagination headers.
/// Returns `X-Total-Count` and `X-Total-Pages` headers alongside the JSON body.
impl<T: Serialize> IntoResponse for PageResponse<T> {
    fn into_response(self) -> Response {
        let total_count = self.total_count;
        let total_pages = self.total_pages;
        let mut response = axum::Json(self).into_response();

        let headers = response.headers_mut();
        let mut buf = itoa::Buffer::new();
        if let Ok(v) = HeaderValue::from_str(buf.format(total_count)) {
            headers.insert("x-total-count", v);
        }
        let mut buf2 = itoa::Buffer::new();
        if let Ok(v) = HeaderValue::from_str(buf2.format(total_pages)) {
            headers.insert("x-total-pages", v);
        }

        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Spec refs: pagination.md request parameter constraints and response headers.
    // Coverage: deserialization boundaries, page math, and response header contract.

    #[test]
    fn test_deserialize_rejects_page_zero() {
        let result: Result<PageRequest, _> =
            serde_urlencoded::from_str("page=0&per_page=20");
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_rejects_per_page_zero() {
        let result: Result<PageRequest, _> =
            serde_urlencoded::from_str("page=1&per_page=0");
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_rejects_per_page_above_100() {
        let result: Result<PageRequest, _> =
            serde_urlencoded::from_str("page=1&per_page=200");
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_accepts_valid_defaults() {
        let result: PageRequest = serde_urlencoded::from_str("").unwrap();
        assert_eq!(result.page(), 1);
        assert_eq!(result.per_page(), 20);
    }

    #[test]
    fn test_offset_first_page() {
        let req: PageRequest = serde_urlencoded::from_str("page=1&per_page=20").unwrap();
        assert_eq!(req.offset(), 0);
    }

    #[test]
    fn test_offset_second_page() {
        let req: PageRequest = serde_urlencoded::from_str("page=2&per_page=20").unwrap();
        assert_eq!(req.offset(), 20);
    }

    #[test]
    fn test_offset_large_page() {
        let req: PageRequest = serde_urlencoded::from_str("page=100&per_page=50").unwrap();
        assert_eq!(req.offset(), 4950);
    }

    #[test]
    fn test_total_pages_exact_division() {
        let resp: PageResponse<String> = PageResponse::new(vec![], 100, 20);
        assert_eq!(resp.total_pages, 5);
    }

    #[test]
    fn test_total_pages_with_remainder() {
        let resp: PageResponse<String> = PageResponse::new(vec![], 101, 20);
        assert_eq!(resp.total_pages, 6);
    }

    #[test]
    fn test_total_pages_zero_items() {
        let resp: PageResponse<String> = PageResponse::new(vec![], 0, 20);
        assert_eq!(resp.total_pages, 0);
    }

    #[test]
    fn test_total_pages_per_page_zero_returns_zero() {
        let resp: PageResponse<String> = PageResponse::new(vec![], 100, 0);
        assert_eq!(resp.total_pages, 0);
    }

    #[test]
    fn test_total_pages_single_item() {
        let resp: PageResponse<String> = PageResponse::new(vec![], 1, 20);
        assert_eq!(resp.total_pages, 1);
    }

    #[tokio::test]
    async fn test_page_response_into_response_sets_pagination_headers() {
        let response = PageResponse::new(vec!["item".to_owned()], 21, 20).into_response();

        assert_eq!(
            response
                .headers()
                .get("x-total-count")
                .and_then(|value| value.to_str().ok()),
            Some("21")
        );
        assert_eq!(
            response
                .headers()
                .get("x-total-pages")
                .and_then(|value| value.to_str().ok()),
            Some("2")
        );

        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body must be readable");
        let json: serde_json::Value =
            serde_json::from_slice(&bytes).expect("response body must be valid json");
        assert_eq!(json["total_count"], 21);
        assert_eq!(json["total_pages"], 2);
        assert_eq!(json["items"][0], "item");
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// P5: Pagination offset is always non-negative.
        #[test]
        fn pagination_offset_never_negative(
            page in 1u32..=10000,
            per_page in 1u32..=100,
        ) {
            let req = PageRequest { page, per_page };
            prop_assert!(req.offset() >= 0);
        }

        /// P5b: total_pages * per_page >= total_count.
        #[test]
        fn pagination_total_pages_correct(
            total_count in 0i64..=100_000,
            per_page in 1u32..=100,
        ) {
            let resp: PageResponse<String> = PageResponse::new(vec![], total_count, per_page);
            prop_assert!(resp.total_pages >= 0);
            if total_count > 0 {
                prop_assert!(resp.total_pages >= 1);
            }
            prop_assert!(resp.total_pages * i64::from(per_page) >= total_count);
        }
    }
}

// Kani formal verification harnesses for pagination arithmetic.
// Run with: cargo kani --harness proof_pagination_offset_no_overflow
#[cfg(kani)]
mod kani_proofs {
    use super::*;

    /// P2: Pagination offset never overflows for valid inputs.
    #[kani::proof]
    fn proof_pagination_offset_no_overflow() {
        let page: u32 = kani::any();
        let per_page: u32 = kani::any();

        kani::assume(page >= 1 && page <= 10_000);
        kani::assume(per_page >= 1 && per_page <= 100);

        let req = PageRequest { page, per_page };
        let offset = req.offset();
        assert!(offset >= 0);
        assert!(offset == i64::from(page - 1) * i64::from(per_page));
    }

    /// P2b: Total pages calculation never overflows.
    #[kani::proof]
    fn proof_total_pages_no_overflow() {
        let total_count: i64 = kani::any();
        let per_page: u32 = kani::any();

        kani::assume(total_count >= 0 && total_count <= 1_000_000);
        kani::assume(per_page >= 1 && per_page <= 100);

        let resp: PageResponse<u8> = PageResponse::new(vec![], total_count, per_page);
        assert!(resp.total_pages >= 0);
        assert!(resp.total_pages * i64::from(per_page) >= total_count);
    }
}
