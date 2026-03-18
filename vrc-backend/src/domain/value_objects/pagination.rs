use serde::{Deserialize, Serialize};

fn default_page() -> u32 {
    1
}
fn default_per_page() -> u32 {
    20
}

#[derive(Debug, Clone, Deserialize)]
pub struct PageRequest {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_per_page")]
    pub per_page: u32,
}

impl PageRequest {
    /// Validate and clamp pagination parameters.
    pub fn validate(&mut self) {
        if self.page == 0 {
            self.page = 1;
        }
        self.per_page = self.per_page.clamp(1, 100);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_clamps_page_zero_to_one() {
        let mut req = PageRequest {
            page: 0,
            per_page: 20,
        };
        req.validate();
        assert_eq!(req.page, 1);
    }

    #[test]
    fn test_validate_clamps_per_page_above_100() {
        let mut req = PageRequest {
            page: 1,
            per_page: 200,
        };
        req.validate();
        assert_eq!(req.per_page, 100);
    }

    #[test]
    fn test_validate_clamps_per_page_zero_to_one() {
        let mut req = PageRequest {
            page: 1,
            per_page: 0,
        };
        req.validate();
        assert_eq!(req.per_page, 1);
    }

    #[test]
    fn test_offset_first_page() {
        let req = PageRequest {
            page: 1,
            per_page: 20,
        };
        assert_eq!(req.offset(), 0);
    }

    #[test]
    fn test_offset_second_page() {
        let req = PageRequest {
            page: 2,
            per_page: 20,
        };
        assert_eq!(req.offset(), 20);
    }

    #[test]
    fn test_offset_large_page() {
        let req = PageRequest {
            page: 100,
            per_page: 50,
        };
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
}
