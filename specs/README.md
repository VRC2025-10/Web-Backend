# VRC Class Reunion — Backend System Specification

## Vision

A Rust-powered backend for the VRChat October Class Reunion community website that pushes the boundaries of type-safe web development. This system encodes authorization rules, query correctness, and protocol invariants at compile time — turning entire categories of runtime bugs into compilation errors.

## Philosophy: Romance Through Rigor

This project deliberately chooses the hardest viable path at every decision point:

- **Type-State Authorization**: Role permissions are phantom types. Handlers that require `Admin` literally cannot be called with a `Member` token — the code won't compile.
- **Compile-Time SQL**: Every query is verified against the live database schema at build time via SQLx. A column rename breaks the build, not production.
- **Algebraic Error Types**: Each API layer has its own error enum. Error conversion is total (no catch-all). Every error code in the spec maps to exactly one variant.
- **Zero-Copy Hot Paths**: Public API responses use `Bytes` and `Cow<'_, str>` to avoid allocation on cache hits.
- **Tower Middleware Calculus**: Authentication, rate limiting, CORS, and caching compose as typed Tower layers — the type signature of a router tells you its middleware stack.
- **Formal Verification**: Critical domain functions are verified with Kani bounded model checking — proofs run on the actual Rust code, eliminating the specification-implementation gap.
- **Custom Procedural Macros**: A `#[handler]` macro generates route registration, permission checks, and OpenAPI documentation from a single annotation.
- **Lock-Free Rate Limiter**: The per-IP rate limiter uses atomic operations and a sharded concurrent map — no mutexes on the hot path.
- **jemalloc**: Global allocator override for reduced fragmentation under sustained load.
- **Property-Based Testing**: All input validators are tested with `proptest` — if it can be generated, it's tested.

## Specification Structure

```
specs/
├── README.md                          ← You are here
├── ASSUMPTIONS.md                     ← Documented assumptions with validation status
├── GLOSSARY.md                        ← Domain terminology
├── 01-requirements/                   ← Functional & non-functional requirements
├── 02-architecture/                   ← Hexagonal architecture, component design, ADRs
├── 03-technology/                     ← Technology evaluation & selection
├── 04-database/                       ← Schema, migrations, query patterns
├── 05-api/                            ← REST API contracts (OpenAPI), error codes
├── 06-security/                       ← Threat model, auth design, data protection
├── 07-infrastructure/                 ← Docker, CI/CD, observability, DR
├── 12-formal-verification/            ← Kani proof harnesses for domain logic
├── 13-testing/                        ← Test strategy, property-based testing
└── 15-project-management/             ← Milestones, phases, risks
```

## Key Numbers

| Metric | Target |
|--------|--------|
| Community size | ~50–300 members |
| Concurrent users (peak) | ~50 |
| API latency P99 (public, cached) | < 5ms |
| API latency P99 (internal, DB hit) | < 50ms |
| Availability | 99.5% (allows ~44h downtime/year) |
| Cold start (Docker) | < 3 seconds |
| Binary size (release, stripped) | < 30 MB |
| Memory footprint (idle) | < 50 MB RSS |
| Compile time (incremental) | < 30 seconds |
