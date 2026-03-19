# Contributing to VRC Web-Backend

Thank you for your interest in contributing to the VRC Web-Backend! This guide will help you get started.

## Code of Conduct

This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code. Please report unacceptable behavior via the methods described in the Code of Conduct.

## How Can I Contribute?

### Reporting Bugs

Before creating a bug report, please check [existing issues](https://github.com/VRC2025-10/Web-Backend/issues) to avoid duplicates.

When filing a bug report, use the [Bug Report template](https://github.com/VRC2025-10/Web-Backend/issues/new?template=bug_report.yml) and include:

- A clear, descriptive title
- Steps to reproduce the behavior
- Expected vs actual behavior
- Environment details (OS, Rust version, PostgreSQL version)
- Relevant logs or error messages

### Suggesting Features

Feature requests are tracked as GitHub Issues. Use the [Feature Request template](https://github.com/VRC2025-10/Web-Backend/issues/new?template=feature_request.yml) and describe:

- The problem you're trying to solve
- Your proposed solution
- Alternatives you've considered

### Your First Contribution

Look for issues labeled:

- [`good first issue`](https://github.com/VRC2025-10/Web-Backend/labels/good%20first%20issue) — well-scoped, starter-friendly tasks
- [`help wanted`](https://github.com/VRC2025-10/Web-Backend/labels/help%20wanted) — issues where help is appreciated

### Pull Request Process

1. **Fork** the repository and clone your fork
2. **Create a branch** from `main`:
   ```bash
   git checkout -b feat/my-feature
   ```
3. **Make your changes** — follow the style guide below
4. **Run checks** locally:
   ```bash
   make check   # or run each step individually:
   cargo fmt --check
   cargo clippy -- -D warnings
   cargo test
   ```
5. **Commit** using [Conventional Commits](https://www.conventionalcommits.org/):
   ```
   feat(api): add pagination to gallery endpoint
   fix(auth): handle expired Discord tokens gracefully
   docs(readme): update quick start instructions
   refactor(domain): extract event validation logic
   ```
6. **Push** to your fork and open a Pull Request against `main`
7. Fill in the PR template — link related issues, describe your changes
8. Wait for CI checks to pass and a maintainer review

## Development Setup

### Prerequisites

- **Rust** 1.85+ (`rustup install stable`)
- **PostgreSQL** 16+
- **Docker** & Docker Compose (for containerized PostgreSQL)

### Getting Started

```bash
# Clone your fork
git clone https://github.com/<your-username>/Web-Backend.git
cd Web-Backend

# Configure environment
cp .env.example .env
# Edit .env with your local settings (see docs/setup.md)

# Start PostgreSQL via Docker
docker compose up -d

# Build the project
cargo build

# Run database migrations
# Migrations run automatically on startup, or use:
# sqlx migrate run --source vrc-backend/migrations

# Run the server
cargo run

# Run tests
cargo test

# Run linting
cargo fmt --check
cargo clippy -- -D warnings
```

### Useful Commands

```bash
make build       # Build in release mode
make test        # Run all tests
make lint        # Run clippy + format check
make fmt         # Auto-format code
make check       # Run lint + test + build (full pre-commit check)
make run         # Run the development server
make docker-up   # Start PostgreSQL in Docker
make docker-down # Stop PostgreSQL
make clean       # Clean build artifacts
```

## Style Guide

### Code Style

- Follow standard Rust idioms and the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- `cargo fmt` is enforced — run it before committing
- `cargo clippy` with `pedantic` warnings is enabled — resolve all warnings
- Prefer explicit error types over `anyhow` / `Box<dyn Error>` — see `src/errors/`

### Architecture Rules

- **Domain layer** (`src/domain/`) must not depend on adapters or frameworks
- **Ports** define trait interfaces; **adapters** implement them
- New endpoints should follow the existing pattern in `src/adapters/inbound/routes/`
- SQL queries must use SQLx compile-time verification (offline mode via `.sqlx/` cache)

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/):

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

Types: `feat`, `fix`, `docs`, `refactor`, `test`, `ci`, `chore`, `perf`, `security`

Scopes: `api`, `auth`, `domain`, `db`, `config`, `docker`, `ci`, `deps`

### Branch Naming

```
feat/<short-description>     # New features
fix/<short-description>      # Bug fixes
docs/<short-description>     # Documentation
refactor/<short-description> # Refactoring
ci/<short-description>       # CI/CD changes
chore/<short-description>    # Maintenance
```

## Project Structure

```
Web-Backend/
├── Cargo.toml               # Workspace root
├── vrc-backend/              # Main application crate
│   ├── src/
│   │   ├── main.rs           # Entry point
│   │   ├── lib.rs            # AppState, module declarations
│   │   ├── domain/           # Business logic (entities, ports, value objects)
│   │   ├── adapters/         # HTTP routes (inbound), DB repos (outbound)
│   │   ├── auth/             # Session extraction, role checks
│   │   ├── background/       # Scheduled tasks
│   │   ├── config/           # Environment-based config
│   │   └── errors/           # Per-layer error types
│   ├── migrations/           # SQLx database migrations
│   └── tests/                # Integration & contract tests
├── vrc-macros/               # Custom procedural macros
├── docs/                     # User-facing documentation
├── specs/                    # System specification & design docs
├── .github/                  # CI/CD workflows, issue templates
└── docker-compose.yml        # Local development (PostgreSQL)
```

## Testing

- **Unit tests**: alongside source code (`#[cfg(test)]` modules)
- **Integration tests**: `vrc-backend/tests/` — API contract tests, router tests
- **Property-based tests**: using `proptest` for input validation coverage
- **Formal verification**: Kani proof harnesses for critical domain logic

Run the full test suite:
```bash
cargo test
```

Run a specific test:
```bash
cargo test test_name
```

## Questions?

- Open a [Discussion](https://github.com/VRC2025-10/Web-Backend/discussions) for questions or ideas
- Check the [documentation](docs/README.md) and [specs](specs/README.md) for design context

Thank you for contributing!
