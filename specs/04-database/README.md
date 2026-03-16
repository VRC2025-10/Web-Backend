# Database Design Overview

## Documents

| Document | Contents |
|----------|----------|
| [conceptual-model.md](./conceptual-model.md) | ER diagram and entity descriptions |
| [logical-model.md](./logical-model.md) | Complete schema: tables, columns, types, constraints, indexes |
| [query-patterns.md](./query-patterns.md) | Critical query patterns with index usage analysis |
| [migration-strategy.md](./migration-strategy.md) | SQLx migration approach |
| [backup-recovery.md](./backup-recovery.md) | Backup strategy and recovery procedures |

## Key Design Principles

1. **UUIDs as primary keys** — No sequential IDs exposed (prevents enumeration attacks)
2. **PostgreSQL ENUMs** — `user_role`, `user_status`, `event_status`, `report_status`, `report_target_type`, `gallery_image_status` are native ENUMs
3. **Timestamps in UTC** — All `TIMESTAMPTZ` columns store UTC; formatting is a frontend concern
4. **Soft state, hard boundaries** — Profiles are soft-visibility (`is_public`); users are soft-suspend (`status`); no hard deletes for audit trail
5. **Foreign keys everywhere** — Referential integrity is the database's job, not the application's
