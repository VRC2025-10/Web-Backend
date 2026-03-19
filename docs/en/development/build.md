# Build System

> **Navigation**: [Docs Home](../README.md) > [Development](README.md) > Build

This document covers all build modes, optimizations, Docker image creation, and the Makefile task runner.

## Build Modes

### Debug Build

```bash
cargo build
# or
make build-debug
```

- Fast compilation, no optimizations
- Debug symbols included
- Incremental compilation enabled
- Use for development and debugging

### Release Build

```bash
cargo build --release
# or
make build
```

The release profile is configured in the workspace root `Cargo.toml` for maximum performance:

```toml
[profile.release]
opt-level = 3        # Maximum optimization
lto = "fat"          # Full link-time optimization across all crates
codegen-units = 1    # Single codegen unit for better optimization
strip = "symbols"    # Remove debug symbols from binary
panic = "abort"      # Abort on panic (smaller binary, no unwinding)
```

| Setting | Effect |
|---|---|
| `opt-level = 3` | Aggressive compiler optimizations, best runtime performance |
| `lto = "fat"` | Whole-program optimization across all crates and dependencies. Slower compile, faster binary |
| `codegen-units = 1` | Forces single-threaded codegen, enabling cross-unit optimizations. Slower compile, faster binary |
| `strip = "symbols"` | Removes symbol table from the binary, reducing size significantly |
| `panic = "abort"` | Eliminates unwinding tables, reducing binary size. Panics terminate immediately |

Release builds take significantly longer but produce a smaller, faster binary.

### Binary Size

The stripped release binary is typically **< 30 MB**. This is achieved through:

- Symbol stripping
- Panic abort (no unwinding tables)
- LTO (dead code elimination across crates)

## Docker Build

The project uses a multi-stage Docker build with **cargo-chef** for optimal layer caching.

### Build Command

```bash
docker build -t vrc-backend:latest .
# or
make docker-build
```

### Multi-Stage Build Explained

```
┌─────────────────────────────────────────────────────┐
│ Stage 1: chef                                       │
│   Base Rust image + cargo-chef installed             │
├─────────────────────────────────────────────────────┤
│ Stage 2: planner                                    │
│   COPY source → cargo chef prepare                  │
│   Output: recipe.json (dependency graph only)        │
├─────────────────────────────────────────────────────┤
│ Stage 3: builder                                    │
│   cargo chef cook (build deps from recipe.json)      │
│   COPY source → cargo build --release                │
│   strip binary                                      │
├─────────────────────────────────────────────────────┤
│ Stage 4: runtime                                    │
│   debian:bookworm-slim (minimal runtime)             │
│   COPY binary + migrations                          │
│   Non-root user, healthcheck configured              │
└─────────────────────────────────────────────────────┘
```

**Why cargo-chef?** Dependency compilation is the slowest part of a Rust Docker build. cargo-chef separates the dependency graph (`recipe.json`) from your source code. When only source code changes, the dependency layer is cached and Docker skips it entirely.

### Build Environment

The Docker build sets:

```dockerfile
ENV RUSTFLAGS="-C target-cpu=x86-64-v3"
ENV SQLX_OFFLINE=true
```

- **`target-cpu=x86-64-v3`** — enables AVX2 and other SIMD instructions available on modern x86-64 CPUs (Intel Haswell+ / AMD Excavator+). This is appropriate for modern cloud environments.
- **`SQLX_OFFLINE=true`** — uses pre-generated query metadata instead of requiring a live database connection at compile time.

### Runtime Image

The runtime stage uses `debian:bookworm-slim` and includes:

- `ca-certificates` — for TLS connections
- `curl` — for the Docker HEALTHCHECK
- Non-root `app` user (UID 1000)
- Port 3000 exposed
- Health check: `curl http://127.0.0.1:3000/health` every 30s

## SQLx Offline Mode

SQLx validates SQL queries at compile time against a real database. For CI and Docker builds where no database is available, SQLx supports **offline mode**.

### Generating Query Metadata

When you add or modify SQL queries, regenerate the metadata:

```bash
make sqlx-prepare
# or
cargo sqlx prepare --workspace
```

This writes JSON files to `target/sqlx-prepare-check/`, one per query.

### Using Offline Mode

Set the environment variable:

```bash
SQLX_OFFLINE=true cargo build
```

CI and Docker builds set this automatically.

> **Important**: Always run `make sqlx-prepare` after changing any SQL query and commit the updated metadata files. CI verifies that the metadata is up to date.

## Makefile Targets

All common tasks are available via `make`:

| Target | Description |
|---|---|
| `help` | Show all available commands |
| `setup` | Install dependencies and set up development environment |
| `run` | Run the development server |
| `run-release` | Run the server in release mode |
| `watch` | Run with auto-reload (requires `cargo-watch`) |
| `build` | Build in release mode |
| `build-debug` | Build in debug mode |
| `test` | Run all tests |
| `test-verbose` | Run all tests with verbose output |
| `lint` | Run clippy and format check |
| `fmt` | Auto-format code |
| `clippy` | Run clippy lints |
| `check` | Full pre-commit check (lint + test + build) |
| `db-up` | Start PostgreSQL via Docker Compose |
| `db-down` | Stop PostgreSQL |
| `db-reset` | Reset database (destroy volume, restart fresh) |
| `db-logs` | Show PostgreSQL logs |
| `docker-build` | Build the production Docker image |
| `docker-up` | Start all services (production compose) |
| `docker-down` | Stop all services (production compose) |
| `docker-logs` | Show production container logs |
| `sqlx-prepare` | Regenerate SQLx offline query cache |
| `clean` | Clean build artifacts |
| `update` | Update Cargo dependencies |
| `audit` | Run security audit on dependencies |
| `deny` | Run cargo-deny checks (licenses, advisories, bans) |

Run `make help` for the complete list with descriptions.

## Cross-Compilation Notes

The production build targets `x86-64-v3` microarchitecture via `RUSTFLAGS="-C target-cpu=x86-64-v3"`. This requires:

- **Minimum CPU**: Intel Haswell (2013) / AMD Excavator (2015) or newer
- **Features enabled**: AVX2, BMI1, BMI2, FMA, LZCNT, MOVBE, XSAVE
- **Cloud compatibility**: All major cloud providers (AWS, GCP, Azure) support x86-64-v3 on current instance types

If deploying to older hardware, remove or change the `target-cpu` flag in the `Dockerfile`.

## Related Documents

- [Setup Guide](setup.md) — local environment setup
- [Testing Guide](testing.md) — running tests
- [CI/CD](ci-cd.md) — automated builds in CI
- [Project Structure](project-structure.md) — codebase layout
