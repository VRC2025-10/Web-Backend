# Migration Strategy

## Tooling

SQLx migrations (`sqlx migrate run`). Migrations are plain SQL files in `migrations/` directory, ordered by numeric prefix.

## Migration Files

```
migrations/
├── 001_create_enums.sql
├── 002_create_users.sql
├── 003_create_profiles.sql
├── 004_create_sessions.sql
├── 005_create_event_tags.sql
├── 006_create_events.sql
├── 007_create_event_tag_mappings.sql
├── 008_create_reports.sql
├── 009_create_clubs.sql
├── 010_create_club_members.sql
└── 011_create_gallery_images.sql
```

## Migration Execution

### Local Development

```bash
# Apply all pending migrations
sqlx migrate run

# Check migration status
sqlx migrate info

# Revert last migration (if reversible file exists)
sqlx migrate revert
```

### CI/CD

Migrations run automatically on application startup via `sqlx::migrate!()` macro, which embeds migration files at compile time:

```rust
sqlx::migrate!("./migrations")
    .run(&pool)
    .await
    .expect("Failed to run database migrations");
```

### Production Deployment

1. `docker compose up -d db` — Start/ensure PostgreSQL is running
2. `docker compose up -d app` — Application starts, runs `sqlx::migrate!()` on boot
3. If migration fails, application exits with non-zero code and `docker compose` will not restart it (no restart policy for init failures)

## Zero-Downtime Migration Patterns

For this project, brief downtime during migration is acceptable (see A-012). However, the following patterns are documented for future reference:

| Schema Change | Safe Pattern |
|--------------|-------------|
| Add nullable column | `ALTER TABLE ADD COLUMN ... NULL` — no lock, instant |
| Add NOT NULL column with default | `ALTER TABLE ADD COLUMN ... NOT NULL DEFAULT ...` — PG 11+ instant |
| Add index | `CREATE INDEX CONCURRENTLY` — does not block writes |
| Drop column | Deploy code that stops reading the column first, then `ALTER TABLE DROP COLUMN` |
| Rename column | Two-phase: add new column → dual-write → migrate reads → drop old column |
| Change column type | Create new column → backfill → swap reads → drop old column |
| Add ENUM value | `ALTER TYPE ... ADD VALUE` — always append (PG doesn't support remove) |

## Rollback Procedures

Each migration has a corresponding `down.sql` concept (not always created as files):

| Migration | Rollback SQL |
|-----------|-------------|
| Create table | `DROP TABLE IF EXISTS <table> CASCADE` |
| Create index | `DROP INDEX IF EXISTS <index>` |
| Add ENUM type | `DROP TYPE IF EXISTS <type> CASCADE` (only if no table uses it) |
| Add ENUM value | **Not supported by PostgreSQL** — create new type and swap |

## SQLx Offline Mode

For CI (where no live PostgreSQL is available during compilation):

```bash
# Generate offline query data (run locally with DB available)
cargo sqlx prepare

# This creates `.sqlx/` directory with query metadata
# Commit `.sqlx/` to version control

# CI builds with:
SQLX_OFFLINE=true cargo build
```
