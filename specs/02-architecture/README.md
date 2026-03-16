# Architecture Overview

## Architecture Style: Hexagonal Architecture (Ports & Adapters)

This backend uses **Hexagonal Architecture** — domain logic is at the center, fully isolated from HTTP, database, and external service concerns through trait-based ports.

### Why Hexagonal?

| Criterion | Hexagonal | Layered (traditional) | Decision |
|-----------|-----------|----------------------|----------|
| Testability | Domain logic testable with mock ports | DB/HTTP coupling makes unit tests hard | Hexagonal wins |
| Rust trait system fit | Ports = traits, Adapters = impl | Modules with direct imports | Hexagonal is idiomatic Rust |
| Compile-time guarantees | Trait bounds enforce correct wiring | Runtime errors from missing deps | Hexagonal wins |
| Complexity | Higher initial boilerplate | Simpler to start | Acceptable — "maximum difficulty" requirement |
| Educational value | Forces deep understanding of DI in Rust | Nothing new learned | Hexagonal wins |

### Key Decisions

- See [ADR-001](./architecture-decision-records/ADR-001-hexagonal-architecture.md) for full rationale
- See [ADR-002](./architecture-decision-records/ADR-002-type-state-authorization.md) for type-state auth
- See [ADR-003](./architecture-decision-records/ADR-003-error-algebra.md) for error type design
- See [ADR-004](./architecture-decision-records/ADR-004-custom-proc-macros.md) for procedural macros
- See [ADR-005](./architecture-decision-records/ADR-005-tower-middleware-composition.md) for middleware design
