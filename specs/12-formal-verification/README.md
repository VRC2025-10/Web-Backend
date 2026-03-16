# Formal Verification Overview

This project employs **Kani** (AWS's bounded model checker for Rust) and compile-time correctness patterns to formally verify critical invariants **directly on the implementation code** — eliminating the specification-implementation gap entirely.

## Why Not TLA+?

TLA+ is excellent for protocol design, but it creates a fundamental problem: **the model and the code are separate artifacts**. When the implementation drifts from the TLA+ specification, the proofs become meaningless. For a project maintained by a solo developer, keeping a separate formal model in sync with the codebase is an ongoing burden with no enforcement mechanism.

**Kani solves this** by running proofs on the actual Rust source code. If the implementation changes, the proof harnesses either still pass (correct refactoring) or fail (invariant violation detected). Zero gap.

## Verification Scope

| Component | Method | What We Verify |
|-----------|--------|---------------|
| Role authorization logic | Kani proof harness | No privilege escalation; role hierarchy monotonicity; super_admin protection |
| Session token hashing | Kani proof harness | Hash uniqueness (collision freedom within bounded domain); no raw token storage path |
| Member leave atomicity | Kani proof harness | State machine transitions; no partial-leave observable state |
| Pagination arithmetic | Kani proof harness | No integer overflow in offset/limit calculation |
| Redirect validation | Kani proof harness | No open redirect; output always starts with `/` |
| All domain types | `#[kani::proof]` | Absence of panics, overflow, out-of-bounds in all domain logic |

## Why Kani?

- **Verifies actual Rust code**: Proof harnesses are `#[cfg(kani)]` functions in the same crate — no separate language
- **CBMC-backed**: Uses the C Bounded Model Checker under the hood; exhaustive within bounds
- **Catches real bugs**: Panics, integer overflow, assertion violations, unreachable code
- **No runtime cost**: All verification happens at build/CI time
- **Rust-native**: Understands ownership, borrowing, lifetimes — no abstraction mismatch
- **Romance**: Bounded model checking on a community website backend is absurdly over-engineered, and that's the point

## Documents

| Document | Contents |
|----------|----------|
| [kani-proofs.md](./kani-proofs.md) | Proof harness specifications and implementation guide |
| [correctness-patterns.md](./correctness-patterns.md) | Compile-time correctness patterns in Rust |
