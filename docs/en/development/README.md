# Development

> **Navigation**: [Docs Home](../README.md) > Development

This section covers everything you need to contribute to the VRC Web-Backend project — from setting up your local environment to understanding the CI/CD pipeline.

## Quick Links

| I want to... | Go to |
|---|---|
| Set up my local environment | [Setup Guide](setup.md) |
| Build the project | [Build System](build.md) |
| Run tests | [Testing Guide](testing.md) |
| Understand the CI/CD pipeline | [CI/CD](ci-cd.md) |
| Navigate the codebase | [Project Structure](project-structure.md) |

## Development Workflow Overview

```
┌─────────────┐    ┌──────────┐    ┌───────────┐    ┌──────────┐
│  Local Dev   │───▶│  Commit  │───▶│  PR / CI  │───▶│  Merge   │
│  & Testing   │    │  & Push  │    │  Pipeline  │    │  & CD    │
└─────────────┘    └──────────┘    └───────────┘    └──────────┘
```

1. **Develop locally** — write code, run `make watch` for auto-reload, run `make test` frequently.
2. **Pre-commit check** — run `make check` (lint + test + build) before pushing.
3. **Open a Pull Request** — CI runs automatically: formatting, clippy, tests, build, security audit.
4. **Review & Merge** — once CI passes and review is approved, merge to `main`.
5. **Continuous Deployment** — merges to `main` trigger Docker image build and deployment.

## Key Commands

```bash
make help          # Show all available commands
make setup         # One-time setup (install tools, create .env, start DB, build)
make run           # Start the development server
make watch         # Auto-reload on file changes (requires cargo-watch)
make check         # Full pre-commit check (lint + test + build)
make test          # Run all tests
make lint          # clippy + fmt --check
make db-reset      # Reset the database
```

## Prerequisites

- **Rust 1.85+** (Edition 2024)
- **Docker & Docker Compose** (for PostgreSQL)
- **Git**

See the [Setup Guide](setup.md) for complete installation instructions.

## Related Documents

- [Architecture Overview](../architecture/README.md)
- [API Reference](../reference/api/README.md)
- [Configuration Guide](../guides/configuration.md)
- [Contributing Guidelines](../../../CONTRIBUTING.md)
