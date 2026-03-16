# Risk Register

## Risk Matrix

| Likelihood → | Low | Medium | High |
|---|---|---|---|
| **High Impact** | 🟡 Medium Risk | 🟠 High Risk | 🔴 Critical Risk |
| **Medium Impact** | 🟢 Low Risk | 🟡 Medium Risk | 🟠 High Risk |
| **Low Impact** | 🟢 Low Risk | 🟢 Low Risk | 🟡 Medium Risk |

## Identified Risks

### R-001: Compile Time Degrades Developer Experience

| Field | Value |
|-------|-------|
| Category | Technical |
| Likelihood | High |
| Impact | Medium |
| Risk Level | 🟠 High |
| Description | Rust compile times can become painful as codebase grows. Proc macro crate adds compilation overhead. Type-state generics may slow type checking. |
| Mitigation | (1) cargo-chef in Docker reduces CI rebuild time. (2) `cargo check` for fast iteration. (3) Split into workspace crates (vrc-backend, vrc-macros) so macro changes don't recompile everything. (4) Use `mold` linker for faster linking. (5) Target < 30s incremental compile per NFR-BUILD-003. |
| Trigger | Incremental compile exceeds 30 seconds consistently. |
| Contingency | Profile compile with `cargo build --timings`, identify and extract slow modules. |

### R-002: Type-State Pattern Too Complex for Some Handlers

| Field | Value |
|-------|-------|
| Category | Technical |
| Likelihood | Medium |
| Impact | Medium |
| Risk Level | 🟡 Medium |
| Description | Some endpoints may need conditional authorization that doesn't map cleanly to the `AuthenticatedUser<R: Role>` pattern (e.g., "owner OR admin" checks). |
| Mitigation | The type-state pattern handles the minimum role requirement. Additional checks (e.g., "is this MY profile?") are done in the handler body as runtime checks. This is documented in ADR-002. |
| Trigger | A handler needs role logic that can't be expressed as a single minimum role level. |
| Contingency | Fall back to runtime authorization for that specific endpoint; document the exception. |

### R-003: Discord API Changes or Rate Limits

| Field | Value |
|-------|-------|
| Category | External Dependency |
| Likelihood | Low |
| Impact | High |
| Risk Level | 🟡 Medium |
| Description | Discord may change OAuth2 endpoints, deprecate scopes, or impose stricter rate limits that affect login flow. |
| Mitigation | (1) Discord OAuth2 is a stable, well-documented API. (2) Abstract Discord interaction behind a trait (Hexagonal Architecture) so the implementation can be swapped. (3) Cache Discord user info to minimize API calls. |
| Trigger | Discord login stops working or returns unexpected errors. |
| Contingency | Adapter can be updated without touching domain logic. |

### R-004: SQLx Compile-Time Checking Friction

| Field | Value |
|-------|-------|
| Category | Technical |
| Likelihood | Medium |
| Impact | Low |
| Risk Level | 🟢 Low |
| Description | SQLx's compile-time query verification requires a running database or offline query data. This can cause friction in CI and when switching branches with schema changes. |
| Mitigation | (1) Use `cargo sqlx prepare` to generate offline query data (`sqlx-data.json`). (2) CI uses `--check` mode against offline data. (3) Document the workflow in the dev setup guide. |
| Trigger | CI fails because sqlx-data.json is out of date. |
| Contingency | Add a CI step that detects stale sqlx data and provides a clear error message. |

### R-005: Kani Verification Complexity

| Field | Value |
|-------|-------|
| Category | Technical |
| Likelihood | Medium |
| Impact | Low |
| Risk Level | 🟢 Low |
| Description | Kani bounded model checking may hit state space explosion or long verification times for complex functions. Some functions (e.g., those calling async I/O) cannot be verified by Kani. |
| Mitigation | (1) Kani proofs target pure domain functions only (no I/O, no async). (2) Bound input domains with `kani::assume` to keep state space tractable. (3) Kani verification is a separate CI job that doesn't block the main build. |
| Trigger | `cargo kani` takes > 5 minutes or reports "verification inconclusive". |
| Contingency | Reduce bounds (smaller input domain) or convert the harness to a proptest property. Kani and proptest are complementary — move properties between them as needed. |

### R-006: Proxmox VM Resource Constraints

| Field | Value |
|-------|-------|
| Category | Infrastructure |
| Likelihood | Low |
| Impact | Medium |
| Risk Level | 🟢 Low |
| Description | Running PostgreSQL + Rust app + Caddy on a Proxmox VM (1-2 GB RAM) may be tight under load. |
| Mitigation | (1) jemalloc reduces memory fragmentation. (2) PostgreSQL configured with conservative `shared_buffers` (128MB). (3) Connection pooling limited to 20 connections. (4) Caddy is lightweight (~30MB). (5) Proxmox allows live CPU/RAM increase without full VM migration. |
| Trigger | OOM killer terminates a process. |
| Contingency | Increase VM RAM allocation in Proxmox UI (instant, no cost). Tune PostgreSQL memory settings down further. |

### R-007: Ammonia / Pulldown-cmark Sanitization Bypass

| Field | Value |
|-------|-------|
| Category | Security |
| Likelihood | Low |
| Impact | High |
| Risk Level | 🟡 Medium |
| Description | A vulnerability in the Markdown → HTML sanitization pipeline could allow XSS. |
| Mitigation | (1) Three-layer defense: pulldown-cmark rendering, ammonia sanitization, post-sanitization regex check. (2) Property-based testing with proptest generates random inputs and verifies no dangerous HTML in output. (3) `cargo audit` in CI detects known CVEs. (4) Ammonia is a well-established, audited crate. |
| Trigger | A proptest case finds a bypass, or `cargo audit` reports a CVE. |
| Contingency | Disable Markdown rendering entirely (store plain text only) until the vulnerability is patched. |

### R-008: October Event Deadline Pressure

| Field | Value |
|-------|-------|
| Category | Business |
| Likelihood | Medium |
| Impact | High |
| Risk Level | 🟠 High |
| Description | The Class Reunion is an annual October event. If the backend isn't ready, it misses the window. |
| Mitigation | (1) Phased delivery: Phase 0-2 provides a working MVP (login + profiles + events). (2) "Romance" features (Kani, proc macros, type-state) can be added after MVP if time is constrained. (3) Hardening (Phase 4) and verification (Phase 5) can be deferred post-launch. |
| Trigger | August arrives and Phase 2 is not complete. |
| Contingency | Ship MVP (Phase 0-2) with basic security hardening. Add "romance" features after the event. |
