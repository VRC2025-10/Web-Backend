# Unit Testing

## Principles

- Unit tests live next to the code they test (`mod tests` in the same file)
- No database, no network, no filesystem — pure functions only
- Use `#[cfg(test)]` module for test-only utilities
- Test both happy paths and error paths
- Use descriptive test names: `test_<function>_<scenario>_<expected_behavior>`

## Domain Layer Test Examples

### Profile Validation

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_vrc_id_valid() {
        let id = "usr_12345678-1234-1234-1234-123456789abc";
        assert!(validate_vrc_id(id).is_ok());
    }

    #[test]
    fn test_validate_vrc_id_missing_prefix() {
        let id = "12345678-1234-1234-1234-123456789abc";
        assert!(matches!(
            validate_vrc_id(id),
            Err(DomainError::ProfileValidation(_))
        ));
    }

    #[test]
    fn test_validate_vrc_id_uppercase_rejected() {
        let id = "usr_12345678-1234-1234-1234-123456789ABC";
        assert!(validate_vrc_id(id).is_err());
    }

    #[test]
    fn test_validate_x_id_valid() {
        assert!(validate_x_id("aqua_vrc").is_ok());
        assert!(validate_x_id("A").is_ok());
        assert!(validate_x_id("123456789012345").is_ok()); // 15 chars max
    }

    #[test]
    fn test_validate_x_id_too_long() {
        let id = "1234567890123456"; // 16 chars
        assert!(validate_x_id(id).is_err());
    }

    #[test]
    fn test_validate_x_id_special_chars_rejected() {
        assert!(validate_x_id("aqua@vrc").is_err());
        assert!(validate_x_id("aqua vrc").is_err());
        assert!(validate_x_id("aqua-vrc").is_err());
    }

    #[test]
    fn test_validate_bio_length_at_limit() {
        let bio = "a".repeat(2000);
        assert!(validate_bio(&bio).is_ok());
    }

    #[test]
    fn test_validate_bio_over_limit() {
        let bio = "a".repeat(2001);
        assert!(validate_bio(&bio).is_err());
    }
}
```

### Role Authorization

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn user_with_role(role: UserRole) -> User {
        User {
            id: 1,
            discord_id: "123".into(),
            role,
            status: UserStatus::Active,
            ..Default::default()
        }
    }

    #[test]
    fn test_admin_can_set_member_to_staff() {
        let actor = user_with_role(UserRole::Admin);
        let target = user_with_role(UserRole::Member);
        assert!(validate_role_change(&actor, &target, UserRole::Staff).is_ok());
    }

    #[test]
    fn test_admin_cannot_grant_admin() {
        let actor = user_with_role(UserRole::Admin);
        let target = user_with_role(UserRole::Member);
        assert!(matches!(
            validate_role_change(&actor, &target, UserRole::Admin),
            Err(DomainError::AdminRoleEscalation)
        ));
    }

    #[test]
    fn test_admin_cannot_grant_super_admin() {
        let actor = user_with_role(UserRole::Admin);
        let target = user_with_role(UserRole::Member);
        assert!(matches!(
            validate_role_change(&actor, &target, UserRole::SuperAdmin),
            Err(DomainError::SuperAdminRoleEscalation)
        ));
    }

    #[test]
    fn test_admin_cannot_modify_super_admin() {
        let actor = user_with_role(UserRole::Admin);
        let target = user_with_role(UserRole::SuperAdmin);
        assert!(matches!(
            validate_role_change(&actor, &target, UserRole::Member),
            Err(DomainError::SuperAdminProtected)
        ));
    }

    #[test]
    fn test_super_admin_can_grant_anything() {
        let actor = user_with_role(UserRole::SuperAdmin);
        let target = user_with_role(UserRole::Member);
        for role in [UserRole::Staff, UserRole::Admin, UserRole::SuperAdmin] {
            assert!(validate_role_change(&actor, &target, role).is_ok());
        }
    }

    #[test]
    fn test_member_cannot_change_roles() {
        let actor = user_with_role(UserRole::Member);
        let target = user_with_role(UserRole::Member);
        assert!(matches!(
            validate_role_change(&actor, &target, UserRole::Staff),
            Err(DomainError::RoleLevelInsufficient)
        ));
    }

    #[test]
    fn test_staff_cannot_change_roles() {
        let actor = user_with_role(UserRole::Staff);
        let target = user_with_role(UserRole::Member);
        assert!(matches!(
            validate_role_change(&actor, &target, UserRole::Staff),
            Err(DomainError::RoleLevelInsufficient)
        ));
    }
}
```

### Markdown Rendering

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_basic_markdown() {
        let html = render_markdown("**bold** and *italic*").unwrap();
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
    }

    #[test]
    fn test_render_strips_script_tags() {
        let html = render_markdown("<script>alert('xss')</script>").unwrap();
        assert!(!html.contains("<script"));
        assert!(!html.contains("alert"));
    }

    #[test]
    fn test_render_strips_event_handlers() {
        let html = render_markdown("<img src=x onerror=alert(1)>").unwrap();
        assert!(!html.contains("onerror"));
    }

    #[test]
    fn test_render_allows_safe_links() {
        let html = render_markdown("[link](https://example.com)").unwrap();
        assert!(html.contains("href=\"https://example.com\""));
        assert!(html.contains("rel=\"noopener noreferrer\""));
    }

    #[test]
    fn test_render_strips_javascript_links() {
        let result = render_markdown("[click](javascript:alert(1))");
        // Either the link is stripped or the entire render is rejected
        match result {
            Ok(html) => assert!(!html.contains("javascript:")),
            Err(DomainError::BioDangerous) => {} // Also acceptable
        }
    }

    #[test]
    fn test_render_empty_input() {
        let html = render_markdown("").unwrap();
        assert!(html.is_empty() || html.trim().is_empty());
    }
}
```

### Redirect Validation

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_redirect_normal_path() {
        assert_eq!(validate_redirect("/dashboard"), "/dashboard");
        assert_eq!(validate_redirect("/"), "/");
        assert_eq!(validate_redirect("/profile/edit"), "/profile/edit");
    }

    #[test]
    fn test_validate_redirect_rejects_protocol_relative() {
        assert_eq!(validate_redirect("//evil.com"), "/");
    }

    #[test]
    fn test_validate_redirect_rejects_absolute_url() {
        assert_eq!(validate_redirect("https://evil.com"), "/");
    }

    #[test]
    fn test_validate_redirect_rejects_backslash() {
        assert_eq!(validate_redirect("/foo\\bar"), "/");
    }

    #[test]
    fn test_validate_redirect_rejects_control_chars() {
        assert_eq!(validate_redirect("/foo\nbar"), "/");
    }
}
```

## Running Unit Tests

```bash
# All unit tests (fast, no DB needed)
cargo test --lib

# With output
cargo test --lib -- --nocapture

# Specific module
cargo test --lib domain::use_cases::tests
```
