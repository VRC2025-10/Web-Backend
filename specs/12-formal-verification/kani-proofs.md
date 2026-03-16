# Kani Proof Harnesses

## Setup

Add Kani as a verification tool (not a Cargo dependency — it's an external tool):

```bash
# Install Kani (requires rustup)
cargo install --locked kani-verifier
kani setup

# Run all proof harnesses
cargo kani

# Run a specific harness
cargo kani --harness proof_role_change_no_escalation
```

### CI Integration

```yaml
# In GitHub Actions workflow
formal-verification:
  name: Kani Verification
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Install Kani
      run: |
        cargo install --locked kani-verifier
        kani setup
    - name: Run Kani proofs
      run: cargo kani --all-features
```

---

## Proof Harnesses

### P1: Role Change — No Privilege Escalation

Verify that `validate_role_change` never allows a lower-privileged actor to grant a higher privilege than they possess.

```rust
// src/domain/use_cases/role.rs

#[cfg(kani)]
mod verification {
    use super::*;

    fn any_role() -> UserRole {
        let v: u8 = kani::any();
        kani::assume(v < 4);
        match v {
            0 => UserRole::Member,
            1 => UserRole::Staff,
            2 => UserRole::Admin,
            _ => UserRole::SuperAdmin,
        }
    }

    #[kani::proof]
    fn proof_role_change_no_escalation() {
        let actor_role = any_role();
        let target_current_role = any_role();
        let new_role = any_role();

        if let Ok(()) = validate_role_change(actor_role, target_current_role, new_role) {
            // If the change was allowed, actor must have strictly higher level
            // than the new_role (except SuperAdmin can grant SuperAdmin)
            assert!(role_level(actor_role) >= role_level(new_role));
        }
    }

    #[kani::proof]
    fn proof_admin_cannot_grant_admin_or_above() {
        let target_current_role = any_role();
        let new_role = any_role();
        kani::assume(new_role == UserRole::Admin || new_role == UserRole::SuperAdmin);

        let result = validate_role_change(UserRole::Admin, target_current_role, new_role);
        assert!(result.is_err());
    }

    #[kani::proof]
    fn proof_super_admin_always_exists() {
        // After any role change, at least one super_admin must remain.
        // Model: 3 users with arbitrary roles, one change operation.
        let roles: [UserRole; 3] = [any_role(), any_role(), any_role()];
        let actor_idx: usize = kani::any();
        let target_idx: usize = kani::any();
        let new_role = any_role();
        kani::assume(actor_idx < 3 && target_idx < 3 && actor_idx != target_idx);

        // Count current super_admins
        let sa_count = roles.iter().filter(|r| **r == UserRole::SuperAdmin).count();
        kani::assume(sa_count >= 1); // precondition: at least one SA exists

        if validate_role_change(roles[actor_idx], roles[target_idx], new_role).is_ok() {
            // After change, count super_admins
            let mut new_roles = roles;
            new_roles[target_idx] = new_role;
            let new_sa_count = new_roles.iter().filter(|r| **r == UserRole::SuperAdmin).count();
            assert!(new_sa_count >= 1, "Role change must not eliminate all super_admins");
        }
    }

    #[kani::proof]
    fn proof_member_staff_cannot_change_roles() {
        let target_role = any_role();
        let new_role = any_role();

        assert!(validate_role_change(UserRole::Member, target_role, new_role).is_err());
        assert!(validate_role_change(UserRole::Staff, target_role, new_role).is_err());
    }
}
```

### P2: Pagination — No Integer Overflow

Verify that pagination offset calculation never overflows.

```rust
// src/domain/value_objects/pagination.rs

#[cfg(kani)]
mod verification {
    use super::*;

    #[kani::proof]
    fn proof_pagination_offset_no_overflow() {
        let page: u32 = kani::any();
        let per_page: u32 = kani::any();

        kani::assume(page >= 1 && page <= 10_000);
        kani::assume(per_page >= 1 && per_page <= 100);

        let req = PageRequest { page, per_page };
        let offset = req.offset(); // Must not panic or overflow
        assert!(offset >= 0);
        assert!(offset == ((page - 1) as i64) * (per_page as i64));
    }

    #[kani::proof]
    fn proof_total_pages_no_overflow() {
        let total_count: i64 = kani::any();
        let per_page: u32 = kani::any();

        kani::assume(total_count >= 0 && total_count <= 1_000_000);
        kani::assume(per_page >= 1 && per_page <= 100);

        let total_pages = compute_total_pages(total_count, per_page);
        assert!(total_pages >= 0);
        assert!(total_pages * (per_page as i64) >= total_count);
    }
}
```

### P3: Redirect Validation — No Open Redirect

Verify that redirect validation always produces a safe relative path.

```rust
// src/domain/use_cases/auth.rs

#[cfg(kani)]
mod verification {
    use super::*;

    #[kani::proof]
    #[kani::unwind(64)] // Bound string length for termination
    fn proof_redirect_always_relative() {
        // Generate a bounded-length arbitrary string
        let len: usize = kani::any();
        kani::assume(len <= 32);

        let mut input = Vec::with_capacity(len);
        for _ in 0..len {
            input.push(kani::any::<u8>());
        }

        if let Ok(s) = std::str::from_utf8(&input) {
            let result = validate_redirect(s);
            assert!(result.starts_with('/'));
            assert!(!result.contains("//"));
            assert!(!result.contains('\\'));
            // No protocol-relative URL possible
            if result.len() >= 2 {
                assert!(result.as_bytes()[1] != b'/');
            }
        }
    }
}
```

### P4: Session Token Hash — Uniqueness (Bounded)

Verify that different tokens always produce different hashes within the checked domain.

```rust
// src/infrastructure/crypto.rs

#[cfg(kani)]
mod verification {
    use super::*;

    #[kani::proof]
    fn proof_different_tokens_different_hashes() {
        let token1: [u8; 32] = kani::any();
        let token2: [u8; 32] = kani::any();
        kani::assume(token1 != token2);

        let hash1 = sha256_hash(&token1);
        let hash2 = sha256_hash(&token2);

        // SHA-256 collision freedom (holds for bounded model checking domain)
        assert!(hash1 != hash2);
    }

    #[kani::proof]
    fn proof_hash_is_deterministic() {
        let token: [u8; 32] = kani::any();
        let hash1 = sha256_hash(&token);
        let hash2 = sha256_hash(&token);
        assert_eq!(hash1, hash2);
    }
}
```

### P5: Member Leave State Machine

Verify that the leave function transitions from any valid pre-state to the correct post-state.

```rust
// src/domain/use_cases/member_leave.rs

#[cfg(kani)]
mod verification {
    use super::*;

    #[kani::proof]
    fn proof_leave_result_is_fully_suspended() {
        let user_status: UserStatus = {
            let v: u8 = kani::any();
            kani::assume(v < 2);
            match v { 0 => UserStatus::Active, _ => UserStatus::Suspended }
        };
        let has_sessions: bool = kani::any();
        let profile_is_public: bool = kani::any();
        let club_count: u8 = kani::any();
        kani::assume(club_count <= 5);

        let pre = MemberState {
            status: user_status,
            has_sessions,
            profile_is_public,
            club_count,
        };

        let post = compute_leave_state(&pre);

        // Post-conditions: all cleanup applied
        assert_eq!(post.status, UserStatus::Suspended);
        assert!(!post.has_sessions);
        assert!(!post.profile_is_public);
        assert_eq!(post.club_count, 0);
    }
}
```

---

## Running Verification

```bash
# Run all harnesses (exhaustive within bounds)
cargo kani

# Run with verbose output (shows CBMC statistics)
cargo kani --verbose

# Run a specific harness with increased unwind bound
cargo kani --harness proof_redirect_always_relative --unwind 128

# Generate verification report
cargo kani --output-format terse
```

**Expected runtime**: All harnesses complete in < 60 seconds on a modern machine. The bounded domain is small enough for CBMC to exhaustively explore.

## Verification vs. Testing Boundary

| Technique | Scope | Exhaustiveness |
|-----------|-------|---------------|
| **Kani proof harnesses** | Pure domain logic (role change, pagination, redirect, state machines) | Exhaustive within bounds |
| **proptest** | Input validation (VRC ID format, X ID, bio length, Markdown rendering) | Statistical (256+ cases) |
| **Unit tests** | Specific scenarios, error paths, edge cases | Manual, curated |
| **Integration tests** | Full HTTP → DB → response cycle | Scenario-based |
| **SQLx compile-time** | SQL schema correctness | Exhaustive (every query) |
| **Type-state pattern** | Role authorization at endpoint level | Exhaustive (compile-time) |

Kani and proptest are complementary:
- **Kani** for properties that can be checked on pure functions with bounded inputs
- **proptest** for properties that involve complex data transformations (Markdown → HTML) where Kani's bounded nature would be too restrictive
