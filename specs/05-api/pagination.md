# Pagination

## Contract

All list endpoints use offset-based pagination with consistent query parameters and response structure.

### Request Parameters

| Parameter | Type | Default | Constraints | Description |
|-----------|------|---------|-------------|-------------|
| `page` | integer | 1 | ≥ 1 | Page number (1-indexed) |
| `per_page` | integer | 20 | 1–100 | Items per page |

Invalid values (non-integer, out of range) return `400 ERR-VALIDATION`.

### Response Structure

```json
{
  "items": [ ... ],
  "total_count": 42,
  "total_pages": 3
}
```

### Response Headers

| Header | Example | Description |
|--------|---------|-------------|
| `X-Total-Count` | `42` | Total items matching filter |
| `X-Total-Pages` | `3` | `ceil(total_count / per_page)` |

### Rust Types

```rust
#[derive(Debug, Deserialize)]
pub struct PageRequest {
    #[serde(default = "default_page")]
    pub page: u32,      // validated: >= 1
    #[serde(default = "default_per_page")]
    pub per_page: u32,  // validated: 1..=100
}

impl PageRequest {
    pub fn offset(&self) -> i64 {
        ((self.page - 1) * self.per_page) as i64
    }

    pub fn limit(&self) -> i64 {
        self.per_page as i64
    }
}

#[derive(Debug, Serialize)]
pub struct PageResponse<T: Serialize> {
    pub items: Vec<T>,
    pub total_count: i64,
    pub total_pages: i64,
}

impl<T: Serialize> PageResponse<T> {
    pub fn new(items: Vec<T>, total_count: i64, per_page: u32) -> Self {
        let total_pages = (total_count as f64 / per_page as f64).ceil() as i64;
        Self { items, total_count, total_pages }
    }
}
```

### SQL Pattern

```sql
-- Item query
SELECT ... FROM <table>
WHERE <filters>
ORDER BY <sort>
LIMIT $limit OFFSET $offset;

-- Count query (always executed alongside item query)
SELECT COUNT(*) FROM <table>
WHERE <filters>;
```

Both queries use the same `WHERE` clause. The count query does not need `ORDER BY` or `LIMIT`.

### Why Not Cursor-Based?

Cursor-based pagination is superior for large datasets and real-time feeds. However:
- Our largest table (users) caps at ~1,000 rows
- The frontend expects `total_pages` for a classic page navigator UI
- Offset pagination is simpler to implement and understand
- Performance impact of `OFFSET` is negligible at our scale

If data volume exceeds 10,000 rows in any table, migrate to cursor-based pagination (keyset pagination using `WHERE id > $cursor`).
