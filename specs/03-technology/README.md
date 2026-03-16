# Technology Selection Overview

## Stack Summary

```
┌─────────────────────────────────────────────────────┐
│                    Frontend SPA                       │
│                  (Next.js — separate repo)            │
└────────────┬────────────────────────────────────────┘
             │ HTTPS
┌────────────▼────────────────────────────────────────┐
│              Caddy (Reverse Proxy + TLS)              │
└────────────┬────────────────────────────────────────┘
             │ HTTP :8080
┌────────────▼────────────────────────────────────────┐
│              Rust / Axum 0.8 / Tokio                  │
│                                                       │
│  tower-http    │  governor    │  tracing              │
│  (CORS, trace) │ (rate limit) │ (structured logging)  │
│  pulldown-cmark│  ammonia     │  metrics-prometheus    │
│  (markdown)    │ (sanitize)   │ (observability)       │
│  reqwest       │  subtle      │  jemalloc             │
│  (Discord API) │ (const-time) │ (global allocator)    │
│                                                       │
│  SQLx 0.8 (compile-time verified queries)             │
└────────────┬────────────────────────────────────────┘
             │ TCP :5432
┌────────────▼────────────────────────────────────────┐
│              PostgreSQL 16                             │
│  ENUMs • UUIDs • UPSERT • JSONB                      │
└─────────────────────────────────────────────────────┘
```

## Evaluation Documents

| Document | Contents |
|----------|----------|
| [tech-stack.md](./tech-stack.md) | Final selections with versions, Cargo.toml, and comparison matrices |
