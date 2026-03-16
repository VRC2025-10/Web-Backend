# Assumptions

All design decisions rest on these assumptions. Each must be validated before implementation.

| ID | Assumption | Impact if Wrong | Validation Method | Status |
|----|-----------|----------------|-------------------|--------|
| A-001 | Community size will remain under 500 active members for the foreseeable future | Single-node PostgreSQL is sufficient; no sharding needed | Monitor `users` table row count | Unvalidated |
| A-002 | Peak concurrent users will not exceed 100 | Single Axum instance (multi-threaded Tokio) handles all traffic | Load test with k6 before launch | Unvalidated |
| A-003 | All members are also members of the designated Discord guild | Guild membership check during OAuth2 login is a hard requirement | Confirm with community organizers | Unvalidated |
| A-004 | The backend runs on a Proxmox VE VM with Docker, not a Kubernetes cluster or cloud VPS | No need for service discovery, distributed tracing, or multi-node orchestration; resources allocated from existing Proxmox pool | Confirm deployment target | Validated |
| A-005 | SSL termination is handled by a reverse proxy (Caddy/Nginx) in front of the Axum server | Backend listens on plain HTTP; no TLS configuration in Rust code | Confirm infrastructure setup | Unvalidated |
| A-006 | Gallery images are stored on local filesystem (Docker volume on the Proxmox VM); the backend stores file paths and serves images via Caddy | Backend needs multipart upload handling or file write; no external cloud storage dependency | Confirm storage approach | Validated |
| A-007 | Event data originates from Google Sheets and is pushed to the backend via System API (GAS trigger) | Backend does not poll external event sources; it receives push updates | Confirm GAS integration | Unvalidated |
| A-008 | The Discord bot calling System API runs as a separate process (not embedded in this backend) | System API needs only Bearer token auth, not Discord gateway integration | Confirm bot architecture | Unvalidated |
| A-009 | The frontend is a separate SPA (e.g., Next.js) served from a different origin | CORS configuration is required; session cookies need `SameSite=Lax` | Confirm frontend stack | Unvalidated |
| A-010 | Only one `super_admin` exists at any time, bootstrapped via environment variable | No UI for super_admin creation; role escalation is an env-var-only operation | Confirm governance model | Unvalidated |
| A-011 | The development team is a solo developer who values learning over velocity | Architecture complexity is justified by educational value, not team throughput | N/A (stated requirement) | Validated |
| A-012 | Downtime during deployment is acceptable (not zero-downtime) | Simple `docker compose down && up` deployment; no blue-green needed | Confirm with stakeholders | Unvalidated |
