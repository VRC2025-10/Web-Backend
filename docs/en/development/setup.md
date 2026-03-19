# Development Environment Setup

> **Navigation**: [Docs Home](../README.md) > [Development](README.md) > Setup

Complete guide for setting up a local development environment for VRC Web-Backend.

## Prerequisites

| Tool | Version | Purpose |
|------|---------|---------|
| Rust | 1.85+ | Compiler, cargo |
| Docker | 20.10+ | PostgreSQL container |
| Docker Compose | v2+ | Container orchestration |
| Git | 2.30+ | Version control |

## Step-by-Step Setup

### 1. Install Rust

Install Rust via [rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

Install required toolchain components:

```bash
rustup component add rustfmt clippy
```

Verify installation:

```bash
rustc --version   # Should be 1.85.0 or later
cargo --version
```

### 2. Install Docker & Docker Compose

Follow the official Docker installation guide for your OS:

- **Linux**: [docs.docker.com/engine/install](https://docs.docker.com/engine/install/)
- **macOS**: [Docker Desktop for Mac](https://docs.docker.com/desktop/install/mac-install/)
- **Windows (WSL2)**: [Docker Desktop for Windows](https://docs.docker.com/desktop/install/windows-install/)

Verify:

```bash
docker --version
docker compose version
```

### 3. Clone the Repository

```bash
git clone https://github.com/<org>/Web-Backend.git
cd Web-Backend
```

### 4. Configure Environment

Create your `.env` file from the template:

```bash
cp .env.example .env
```

Edit `.env` with your local settings. The defaults work with the provided `docker-compose.yml`:

```dotenv
DATABASE_URL=postgres://vrc:vrc@localhost:5432/vrc
```

### 5. Start PostgreSQL

```bash
docker compose up -d
```

This starts a PostgreSQL 16 instance. Database migrations run automatically on first server start.

Verify the database is running:

```bash
docker compose ps
```

### 6. Build the Project

```bash
cargo build
```

The first build downloads and compiles all dependencies — this takes a few minutes.

### 7. Run the Server

```bash
cargo run
```

The server starts on `http://localhost:3000`.

### 8. Verify

```bash
curl http://localhost:3000/health
```

You should receive a health status response indicating the server is running and connected to the database.

### Quick Setup (One Command)

If you prefer, `make setup` handles steps 4–6 automatically:

```bash
make setup
```

## Optional Tools

### cargo-watch (Auto-Reload)

Automatically recompiles and restarts the server on file changes:

```bash
cargo install cargo-watch
make watch
```

### cargo-audit (Security Scanning)

Checks dependencies for known security vulnerabilities:

```bash
cargo install cargo-audit
cargo audit
```

### cargo-deny (License & Advisory Checks)

Comprehensive dependency policy enforcement:

```bash
cargo install cargo-deny
cargo deny check
```

### sqlx-cli (Database Management)

Manage migrations and generate offline query metadata:

```bash
cargo install sqlx-cli --no-default-features --features postgres,rustls
```

### Kani Verifier (Formal Verification)

For running formal verification proofs:

```bash
cargo install --locked kani-verifier
cargo kani setup
```

See the [Testing Guide](testing.md) for details on Kani usage.

## IDE Setup

### VS Code (Recommended)

Install the following extensions:

| Extension | ID | Purpose |
|---|---|---|
| rust-analyzer | `rust-lang.rust-analyzer` | Rust language support |
| Even Better TOML | `tamasfe.even-better-toml` | Cargo.toml support |
| Error Lens | `usernamehw.errorlens` | Inline error display |
| crates | `serayuzgur.crates` | Dependency version hints |

Recommended `settings.json` for the workspace:

```jsonc
{
  "rust-analyzer.check.command": "clippy",
  "rust-analyzer.check.extraArgs": ["--", "-D", "warnings"],
  "rust-analyzer.cargo.features": "all",
  "[rust]": {
    "editor.formatOnSave": true,
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  }
}
```

### JetBrains (RustRover / IntelliJ + Rust Plugin)

RustRover provides built-in Rust support. Open the workspace root (`Web-Backend/`) as the project root so the workspace `Cargo.toml` is detected.

## Troubleshooting

### Database connection refused

Ensure PostgreSQL is running:

```bash
docker compose ps
docker compose logs postgres
```

If the container is not running, try:

```bash
docker compose down -v
docker compose up -d
```

### `sqlx` compile-time errors

If you see SQLx errors about missing database, you can build in offline mode:

```bash
SQLX_OFFLINE=true cargo build
```

The project includes pre-generated query metadata in `target/sqlx-prepare-check/`.

### Port already in use

If port 3000 is occupied, either stop the conflicting process or override the port via `.env`:

```dotenv
PORT=3001
```

## Related Documents

- [Build System](build.md) — debug, release, and Docker builds
- [Testing Guide](testing.md) — running tests and verification
- [CI/CD](ci-cd.md) — automated pipeline
- [Project Structure](project-structure.md) — codebase navigation
