# Glossary

| Term | Definition |
|------|-----------|
| **Class Reunion (同期会)** | A VRChat community event where members who joined around October gather regularly |
| **Member** | A registered user of the platform who is also a member of the designated Discord guild |
| **Profile** | A member's public-facing information card (VRC ID, X/Twitter handle, bio in Markdown) |
| **Club (部活動)** | A sub-group within the community organized around a shared interest or activity |
| **Gallery** | A collection of images associated with a club, subject to approval workflow (`pending` → `approved` / `rejected`) |
| **Event** | A scheduled VRChat gathering with title, description, host, time, location, and tags |
| **Event Tag** | A colored label (e.g., "Social", "Beginner") that categorizes events |
| **System API** | Machine-to-machine API layer for GAS and Discord Bot integration |
| **Internal API** | Session-authenticated BFF (Backend for Frontend) API layer |
| **Public API** | Unauthenticated, cacheable API layer for public data consumption |
| **Auth API** | Discord OAuth2 login flow endpoints |
| **GAS** | Google Apps Script — used to push event data from Google Sheets to the System API |
| **Guild** | A Discord server; membership in the designated guild is required for login |
| **Session** | A server-side session identified by a UUID stored in an HttpOnly cookie |
| **OAuth State** | A CSRF-protection nonce generated during Discord OAuth2 login flow |
| **Type-State Pattern** | A Rust design pattern where state transitions are encoded as type transformations, making invalid states unrepresentable at compile time |
| **Tower Layer** | A composable middleware abstraction from the `tower` crate that wraps a `Service` |
| **Hexagonal Architecture** | An architecture where domain logic is at the center, surrounded by ports (traits) and adapters (implementations), isolating business rules from infrastructure |
| **Port** | A Rust trait defining an interface between the domain and the outside world (e.g., `UserRepository`, `DiscordClient`) |
| **Adapter** | A concrete implementation of a port (e.g., `PostgresUserRepository`, `ReqwestDiscordClient`) |
| **Compile-Time Verification** | SQLx's ability to check SQL queries against a live database schema at `cargo build` time |
| **Error Algebra** | A system where each API layer has a dedicated error enum, and conversions between error types are exhaustive (`From` impls with no catch-all) |
| **RBAC** | Role-Based Access Control — authorization model used in this system (4 roles: `member`, `staff`, `admin`, `super_admin`) |
