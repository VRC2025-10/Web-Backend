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
