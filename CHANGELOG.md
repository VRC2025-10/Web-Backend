# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Hexagonal architecture with domain-driven design (entities, ports, adapters)
- Discord OAuth2 authentication with guild membership verification
- Role-based access control: `super_admin`, `admin`, `staff`, `member`
- Public API for member profiles, events, clubs, and gallery browsing
- Internal API for profile management and administration
- System API for machine-to-machine integration (GAS, Discord bots)
- Server-side session management with automatic cleanup
- Markdown rendering with XSS-safe HTML sanitization (pulldown-cmark + ammonia)
- Per-IP rate limiting with lock-free token bucket (governor)
- Discord webhook notifications for events and moderation reports
- Background task scheduler (session cleanup, event archival)
- Prometheus metrics endpoint for observability
- Structured JSON logging via tracing
- Compile-time SQL verification with SQLx offline mode
- Custom `#[handler]` procedural macro for route generation
- Property-based testing with proptest
- Formal verification harnesses with Kani
- Multi-stage Docker build with Caddy reverse proxy
- Comprehensive CI/CD pipeline (22-job CI, CD with rollback, release automation)
- Security scanning pipeline (cargo-audit, cargo-deny, container scanning)
- Nightly extended testing (Miri, Kani, property-based fuzzing)

[Unreleased]: https://github.com/VRC2025-10/Web-Backend/commits/main
