# Architecture Decision Records (ADRs)

> **Audience**: All
>
> **Navigation**: [Docs Home](../../README.md) > [Design](../README.md) > ADRs

## What Are ADRs?

Architecture Decision Records capture important architectural decisions along with their context and consequences. Each ADR describes a single decision, why it was made, and what trade-offs were accepted.

We follow the format described in [ADR Template](template.md).

## ADR Index

| ADR | Title | Status | Date |
|-----|-------|--------|------|
| [ADR-0001](0001-hexagonal-architecture.md) | Hexagonal Architecture | ✅ Accepted | 2025-01-15 |
| [ADR-0002](0002-type-state-authorization.md) | Type-State Authorization | ✅ Accepted | 2025-01-15 |
| [ADR-0003](0003-compile-time-sql.md) | Compile-Time SQL Verification | ✅ Accepted | 2025-01-20 |
| [ADR-0004](0004-algebraic-error-types.md) | Algebraic Error Types | ✅ Accepted | 2025-01-20 |
| [ADR-0005](0005-formal-verification.md) | Formal Verification with Kani | ✅ Accepted | 2025-02-01 |
| [ADR-0006](0006-discord-only-authentication.md) | Discord-Only Authentication | ✅ Accepted | 2025-01-10 |
| [ADR-0007](0007-server-side-sessions.md) | Server-Side Sessions over JWT | ✅ Accepted | 2025-01-10 |
| [ADR-0008](0008-japanese-error-messages.md) | Japanese Error Messages | ✅ Accepted | 2025-01-10 |

## Status Definitions

| Status | Meaning |
|--------|---------|
| ✅ Accepted | Decision is in effect and implemented |
| 🔄 Proposed | Decision is under discussion |
| ⛔ Deprecated | Decision has been superseded by a newer ADR |
| ❌ Rejected | Decision was considered but not adopted |

## Creating a New ADR

1. Copy the [template](template.md)
2. Number it sequentially (e.g., `0009-my-decision.md`)
3. Fill in all sections — Context is the most important
4. Set status to **Proposed**
5. Open a PR for review
6. Update status to **Accepted** when merged

## Related Documents

- [Design Principles](../principles.md)
- [Design Patterns](../patterns.md)
- [Trade-offs](../trade-offs.md)
