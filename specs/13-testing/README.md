# Testing Strategy Overview

## Test Pyramid

```
                    ╱╲
                   ╱  ╲         E2E / Integration (API-level)
                  ╱    ╲        ~20 tests, run against real PostgreSQL
                 ╱──────╲
                ╱        ╲      Property-Based (proptest)
               ╱          ╲     ~15 properties, generative
              ╱────────────╲
             ╱              ╲   Unit Tests (domain + validation)
            ╱                ╲  ~80 tests, no I/O, fast
           ╱──────────────────╲
          ╱                    ╲ Compile-Time (type system + SQLx)
         ╱                      ╲ Always on, zero runtime cost
        ╱────────────────────────╲
```

## Coverage Targets

| Layer | Target | Rationale |
|-------|--------|-----------|
| Domain logic (use cases, validation) | 90%+ | Core business rules must be thoroughly tested |
| Adapters (routes, DB queries) | 70%+ | Integration tests cover the critical paths |
| Infrastructure (config, middleware) | 50%+ | Mostly tested indirectly via integration tests |
| Proc macros | 80%+ | Macro bugs are hard to debug; test generated code |

## Documents

| Document | Contents |
|----------|----------|
| [unit-testing.md](./unit-testing.md) | Unit test patterns and examples |
| [integration-testing.md](./integration-testing.md) | API-level tests with real DB |
| [property-testing.md](./property-testing.md) | Property-based testing with proptest |
