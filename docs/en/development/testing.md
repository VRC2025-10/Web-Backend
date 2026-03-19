# Testing Guide

> **Navigation**: [Docs Home](../README.md) > [Development](README.md) > Testing

This document covers the testing strategy, tools, and patterns used in VRC Web-Backend.

## Testing Pyramid

```
         ┌──────────┐
         │  Formal   │  ← Kani proofs (critical invariants)
         │  Verify   │
        ┌┴──────────┴┐
        │  Property   │  ← proptest (input space exploration)
        │  Tests      │
       ┌┴────────────┴┐
       │  Integration  │  ← vrc-backend/tests/ (API + DB)
       │  Tests        │
      ┌┴──────────────┴┐
      │   Unit Tests    │  ← #[cfg(test)] in each module
      └────────────────┘
```

## Quick Commands

| Command | What it runs |
|---|---|
| `make test` | All unit + integration tests |
| `make test-verbose` | Same, with stdout/stderr output |
| `cargo test -- --nocapture` | Tests with unfiltered output |
| `cargo test <name>` | Run tests matching `<name>` |
| `cargo test --lib` | Unit tests only |
| `cargo test --test '*'` | Integration tests only |
| `cargo kani` | Formal verification proofs |

## Unit Tests

Unit tests live alongside the code they test, inside `#[cfg(test)]` modules:

```rust
// src/domain/entities/user.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        // ...
    }
}
```

### Conventions

- Every public function should have at least one test.
- Test names describe the scenario: `test_<function>_<scenario>_<expected>`.
- Use `assert_eq!`, `assert_ne!`, `assert!` macros.
- Keep tests focused — one assertion per logical behavior.

### Running Unit Tests

```bash
cargo test --lib             # Unit tests in lib crate
cargo test --lib -p vrc-backend   # Only vrc-backend unit tests
cargo test --lib -p vrc-macros    # Only vrc-macros unit tests
```

## Property-Based Tests

Property-based tests use [proptest](https://docs.rs/proptest/) to explore large input spaces automatically. Instead of hand-picked examples, proptest generates thousands of randomized inputs and verifies that properties hold for all of them.

### What We Test with proptest

| Property | Description |
|---|---|
| VRC ID format | Valid VRC IDs match `usr_[a-zA-Z0-9]+` pattern and invalid ones are rejected |
| X (Twitter) ID format | Valid screen names match expected patterns |
| Role ordering | Role hierarchy is consistent (`Admin > Moderator > Member > Guest`) |
| Markdown sanitization | Rendered Markdown never contains `<script>`, `onclick`, or other XSS vectors |
| Pagination | `PageRequest` always produces valid SQL `LIMIT`/`OFFSET` values |

### Example

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn vrc_id_regex_matches_valid_ids(
        suffix in "[a-zA-Z0-9]{10,20}"
    ) {
        let id = format!("usr_{suffix}");
        assert!(VRC_ID_REGEX.is_match(&id));
    }

    #[test]
    fn markdown_never_produces_xss(
        input in ".*"
    ) {
        let rendered = render_markdown(&input);
        assert!(!rendered.contains("<script"));
        assert!(!rendered.contains("onclick"));
    }
}
```

### Running Property Tests

```bash
# Default (256 cases per test)
cargo test -- proptest

# Extended (10,000 cases per test)
PROPTEST_CASES=10000 cargo test -- proptest
```

When a proptest failure is found, it writes a regression file to `proptest-regressions/`. These files are committed to the repo so the failing case is always re-tested.

## Integration Tests

Integration tests live in `vrc-backend/tests/` and test the full HTTP handler → database round trip.

### Structure

```
vrc-backend/tests/
├── common/          # Shared test helpers
│   └── mod.rs       # Test app setup, auth helpers
├── health_test.rs   # Health endpoint tests
├── auth_test.rs     # Authentication flow tests
└── ...
```

### Running Integration Tests

Integration tests require a running PostgreSQL instance:

```bash
# Start the database
make db-up

# Run integration tests
cargo test --test '*'

# Run a specific integration test file
cargo test --test health_test
```

### Writing Integration Tests

```rust
use axum::http::StatusCode;
use axum_test::TestServer;

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let app = setup_test_app().await;
    let server = TestServer::new(app).unwrap();

    let response = server.get("/health").await;

    assert_eq!(response.status_code(), StatusCode::OK);
}
```

## Formal Verification (Kani)

[Kani](https://model-checking.github.io/kani/) is a formal verification tool for Rust that uses bounded model checking to prove correctness properties.

### What We Verify with Kani

| Proof | Property |
|---|---|
| Role authorization | Authorized roles are always a superset of required roles; no privilege escalation possible |
| Redirect URL validation | Only same-origin or allowlisted URLs pass validation; no open redirect |
| Member leave state machine | State transitions are valid; cannot reach an invalid state from any starting state |

### Running Kani Proofs

```bash
# Run all proofs
cargo kani

# Run a specific proof
cargo kani --harness proof_role_authorization
```

Kani is also run in the nightly CI pipeline to catch regressions.

### Example Kani Proof

```rust
#[cfg(kani)]
mod verification {
    use super::*;

    #[kani::proof]
    fn proof_role_authorization() {
        let required: Role = kani::any();
        let actual: Role = kani::any();

        if is_authorized(actual, required) {
            assert!(actual.level() >= required.level());
        }
    }
}
```

## Coverage Strategy

### Expectations

| Layer | Coverage Target | How |
|---|---|---|
| Domain logic | **≥ 90%** | Unit tests + property tests |
| Adapters (HTTP handlers) | **≥ 80%** | Integration tests |
| Error paths | **≥ 70%** | Unit tests with crafted error inputs |
| Infrastructure glue | **Best effort** | Integration tests |

### Generating Coverage Reports

```bash
# Install coverage tool
cargo install cargo-tarpaulin

# Generate HTML report
cargo tarpaulin --out html --output-dir coverage/
```

## Writing Tests — Patterns

### Arrange-Act-Assert

```rust
#[test]
fn test_page_request_clamps_to_max() {
    // Arrange
    let raw = PageRequest { page: 1, per_page: 999 };

    // Act
    let clamped = raw.clamp();

    // Assert
    assert_eq!(clamped.per_page, MAX_PER_PAGE);
}
```

### Testing Error Cases

```rust
#[test]
fn test_invalid_vrc_id_returns_error() {
    let result = validate_vrc_id("not-a-valid-id");
    assert!(result.is_err());
}
```

### Testing with Database (Integration)

Integration tests use a dedicated test database to avoid polluting development data. Each test function should set up its own data and clean up after itself (or use transactions that are rolled back).

## Related Documents

- [Setup Guide](setup.md) — installing test tools
- [Build System](build.md) — building before testing
- [CI/CD](ci-cd.md) — automated test execution
- [Project Structure](project-structure.md) — where tests live in the codebase
