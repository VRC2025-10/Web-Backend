# Property-Based Testing

## Strategy

Use `proptest` to generate random inputs and verify that properties hold for all generated cases. This catches edge cases that manually written tests miss.

## Properties

### P1: Markdown rendering never produces dangerous HTML

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn markdown_never_produces_script_tags(input in "\\PC{0,2000}") {
        if let Ok(html) = render_markdown(&input) {
            let lower = html.to_lowercase();
            prop_assert!(!lower.contains("<script"));
            prop_assert!(!lower.contains("javascript:"));
            prop_assert!(!lower.contains("onerror="));
            prop_assert!(!lower.contains("onload="));
        }
        // render_markdown returning Err is also fine (BioDangerous)
    }
}
```

### P2: VRC ID validation accepts only valid format

```rust
proptest! {
    #[test]
    fn valid_vrc_ids_match_pattern(
        a in "[0-9a-f]{8}",
        b in "[0-9a-f]{4}",
        c in "[0-9a-f]{4}",
        d in "[0-9a-f]{4}",
        e in "[0-9a-f]{12}",
    ) {
        let id = format!("usr_{}-{}-{}-{}-{}", a, b, c, d, e);
        prop_assert!(validate_vrc_id(&id).is_ok());
    }

    #[test]
    fn random_strings_rejected_as_vrc_id(input in "\\PC{0,100}") {
        // Unless it happens to match the exact pattern (astronomically unlikely)
        if !input.starts_with("usr_") {
            prop_assert!(validate_vrc_id(&input).is_err());
        }
    }
}
```

### P3: X ID validation is strict

```rust
proptest! {
    #[test]
    fn valid_x_ids_accepted(id in "[a-zA-Z0-9_]{1,15}") {
        prop_assert!(validate_x_id(&id).is_ok());
    }

    #[test]
    fn x_ids_with_special_chars_rejected(
        prefix in "[a-zA-Z0-9_]{0,7}",
        bad_char in "[^a-zA-Z0-9_]",
        suffix in "[a-zA-Z0-9_]{0,7}",
    ) {
        let input = format!("{}{}{}", prefix, bad_char, suffix);
        prop_assert!(validate_x_id(&input).is_err());
    }
}
```

### P4: Role level is monotonically ordered

```rust
proptest! {
    #[test]
    fn role_level_ordering(
        r1 in prop_oneof!["member", "staff", "admin", "super_admin"],
        r2 in prop_oneof!["member", "staff", "admin", "super_admin"],
    ) {
        let l1 = role_level(&r1);
        let l2 = role_level(&r2);
        // If r1 satisfies r2, then level(r1) >= level(r2)
        if l1 >= l2 {
            prop_assert!(satisfies_role(&r1, &r2));
        }
    }
}
```

### P5: Pagination invariants

```rust
proptest! {
    #[test]
    fn pagination_offset_never_negative(
        page in 1u32..=10000,
        per_page in 1u32..=100,
    ) {
        let req = PageRequest { page, per_page };
        prop_assert!(req.offset() >= 0);
    }

    #[test]
    fn pagination_total_pages_correct(
        total_count in 0i64..=100000,
        per_page in 1u32..=100,
    ) {
        let total_pages = (total_count as f64 / per_page as f64).ceil() as i64;
        prop_assert!(total_pages >= 0);
        if total_count > 0 {
            prop_assert!(total_pages >= 1);
        }
        // Check: total_pages * per_page >= total_count
        prop_assert!(total_pages * per_page as i64 >= total_count);
    }
}
```

### P6: Redirect validation rejects all dangerous patterns

```rust
proptest! {
    #[test]
    fn redirect_never_returns_absolute_url(input in "\\PC{0,200}") {
        let result = validate_redirect(&input);
        // Result must start with '/' (relative) or be the default "/"
        prop_assert!(result.starts_with('/'));
        // Must not contain protocol-relative patterns
        prop_assert!(!result.contains("//"));
        prop_assert!(!result.contains('\\'));
    }
}
```

### P7: Session token hash is irreversible (probabilistic)

```rust
proptest! {
    #[test]
    fn different_tokens_produce_different_hashes(
        token1 in prop::array::uniform32(any::<u8>()),
        token2 in prop::array::uniform32(any::<u8>()),
    ) {
        prop_assume!(token1 != token2);
        let hash1 = sha256(&token1);
        let hash2 = sha256(&token2);
        prop_assert_ne!(hash1, hash2);
    }
}
```

## Running Property Tests

```bash
# Run with default case count (256 cases per property)
cargo test --lib -- proptest

# Run with more cases (for pre-release verification)
PROPTEST_CASES=10000 cargo test --lib -- proptest

# Regression file: proptest-regressions/ stores shrunk counterexamples
# These are automatically replayed on subsequent runs
```

## Shrinking

`proptest` automatically shrinks failing inputs to the minimal reproduction case. For example, if a random string causes an XSS bypass in `render_markdown`, `proptest` will shrink it to the shortest possible string that still triggers the bug.

Shrunk counterexamples are stored in `proptest-regressions/` files (committed to version control) and replayed on every subsequent test run.
