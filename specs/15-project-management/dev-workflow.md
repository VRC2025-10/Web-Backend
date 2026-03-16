# Development Workflow

## Branch Strategy

```
main ─────────────────────────────────────────── (always deployable)
  └── feat/phase-1-auth ─────── PR ──→ main
  └── feat/phase-2-profile ──── PR ──→ main
  └── fix/session-expiry ────── PR ──→ main
```

- `main` is the production branch. Every commit on `main` should pass CI and be deployable.
- Feature branches follow the naming convention: `feat/<phase>-<description>` or `fix/<description>`.
- All changes go through a pull request, even for solo development (to maintain CI discipline).

## Commit Convention

Follow Conventional Commits:

```
<type>(<scope>): <description>

Types: feat, fix, refactor, test, docs, chore, ci
Scopes: auth, profile, event, club, gallery, admin, system, infra, tla
```

Examples:
```
feat(auth): implement Discord OAuth2 login redirect
fix(profile): reject VRC IDs with uppercase characters
test(auth): add proptest for session token hashing
refactor(domain): extract role validation into separate module
docs(api): update error code catalog
ci: add Kani proof harness CI step
chore: update dependencies
```

## Development Cycle

### Daily Workflow

```
1. Pull latest main
2. Create or continue a feature branch
3. Write failing test → Write code → Tests pass
4. cargo fmt && cargo clippy -- -D warnings
5. cargo sqlx prepare  (if SQL queries changed)
6. Commit with conventional commit message
7. Push and create/update PR
8. CI must pass before merge
```

### Code Quality Gates (enforced by CI)

| Gate | Tool | Requirement |
|------|------|-------------|
| Format | `cargo fmt --check` | No formatting changes needed |
| Lint | `cargo clippy -- -D warnings` | Zero warnings |
| Unit tests | `cargo test --lib` | All pass |
| Integration tests | `cargo test --test '*'` | All pass |
| SQL verification | `cargo sqlx prepare --check` | Offline data matches |
| Security audit | `cargo audit` | No known vulnerabilities |
| Build | `cargo build --release` | Compiles successfully |

### Pre-PR Checklist

Before creating a pull request:

- [ ] All new code has tests (unit or integration)
- [ ] `cargo fmt` applied
- [ ] `cargo clippy` clean
- [ ] `cargo sqlx prepare` updated (if SQL changed)
- [ ] Error codes follow the `ERR-XXX-NNN` convention
- [ ] New endpoints documented in the relevant spec file (optional but encouraged)

## Environment Setup

### Local Development

```bash
# 1. Start PostgreSQL
docker compose up -d db

# 2. Copy environment variables
cp .env.example .env
# Edit .env with your Discord app credentials

# 3. Run migrations
cargo sqlx migrate run

# 4. Run the server
cargo run

# 5. Run tests
cargo test
```

### Environment Variables

See [docs/setup.md](../../docs/setup.md) for the full list. Critical variables:

| Variable | Example | Required |
|----------|---------|----------|
| `DATABASE_URL` | `postgres://vrc_app:password@localhost:5432/vrc_class_reunion` | Yes |
| `DISCORD_CLIENT_ID` | `1234567890` | Yes |
| `DISCORD_CLIENT_SECRET` | `abcdef...` | Yes |
| `SESSION_SECRET` | (64+ hex chars) | Yes |
| `SYSTEM_API_TOKEN` | (64+ hex chars) | Yes |
| `BASE_URL` | `https://vrc-classreunion.example.com` | Yes |
| `RUST_LOG` | `vrc_backend=debug,tower_http=debug` | No |
