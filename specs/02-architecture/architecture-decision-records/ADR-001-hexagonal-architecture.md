# ADR-001: Hexagonal Architecture (Ports & Adapters)

## Status
Accepted

## Context

We need an internal architecture for a Rust/Axum backend that:
1. Maximizes educational value ("maximum difficulty" requirement)
2. Enables unit testing of domain logic without database or network access
3. Leverages Rust's trait system for compile-time dependency injection
4. Cleanly separates HTTP concerns from business rules

## Decision

Adopt **Hexagonal Architecture** (Ports & Adapters) with:
- **Domain core** (`src/domain/`) has zero external dependencies (no Axum, no SQLx, no reqwest)
- **Ports** are Rust traits (e.g., `trait UserRepository`, `trait DiscordClient`)
- **Adapters** are concrete implementations wired at startup in `main.rs`
- Use cases accept ports as generic parameters (`impl UserRepository`) or trait objects (`Arc<dyn UserRepository>`)

### Dependency Injection Strategy

Use **static dispatch with generics** for performance-critical paths and **dynamic dispatch with `Arc<dyn Trait>`** for shared application state:

```rust
// Application state holds trait objects (one allocation, shared across all requests)
pub struct AppState {
    pub user_repo: Arc<dyn UserRepository>,
    pub profile_repo: Arc<dyn ProfileRepository>,
    pub session_repo: Arc<dyn SessionRepository>,
    pub discord_client: Arc<dyn DiscordClient>,
    pub webhook_sender: Arc<dyn WebhookSender>,
    pub markdown_renderer: Arc<dyn MarkdownRenderer>,
    pub clock: Arc<dyn Clock>,
    pub config: Arc<AppConfig>,
}
```

This trades ~1 vtable indirection per call for simplicity of wiring. At our scale (<100 concurrent users), this is unmeasurable.

## Consequences

### Positive
- Domain logic is testable with mock implementations of every port
- Swapping PostgreSQL for another database requires only new adapter implementations
- Forces clear thinking about interface boundaries
- `domain/` compiles without any I/O crate features — fast incremental builds

### Negative
- More boilerplate than a flat module structure (trait definitions + impl blocks)
- Developers must understand generics, trait objects, and `Send + Sync` bounds
- Over-engineering for a ~300-user community site (justified by educational goal)

### Risks
- Generic type signatures can become unwieldy → Mitigate with type aliases and `Arc<dyn Trait>`
- Compile times may increase with deep generic nesting → Mitigate with trait objects for DI

## Alternatives Considered

### Alternative 1: Flat Module Structure (Traditional Layered)
- **Description**: `routes/`, `services/`, `models/`, `db/` modules with direct imports
- **Pros**: Simpler, less boilerplate, faster to write
- **Cons**: Business logic coupled to DB types, hard to mock, less educational value
- **Rejection Reason**: Does not maximize learning; violates "maximum difficulty" requirement

### Alternative 2: Actor Model (Actix)
- **Description**: Each domain concern is an actor with message passing
- **Pros**: Natural concurrency model, fault isolation per actor
- **Cons**: Overhead for simple CRUD operations, complex message routing, Actix actor framework is less maintained
- **Rejection Reason**: Over-engineering communication for a request-response API; actor overhead unjustified
