# Out of Scope

The following features and capabilities are explicitly **not** part of this backend system:

| Item | Reason |
|------|--------|
| Frontend application (React/Next.js/etc.) | Separate repository and deployment |
| Discord bot implementation | Separate process; communicates via System API |
| Google Apps Script (GAS) implementation | Separate codebase; calls System API |
| Image upload processing (resize, compress, convert) | May be added later; initially stored as-is on local filesystem |
| Email notifications | Community uses Discord for all notifications |
| Real-time features (WebSocket, SSE) | Not required for current feature set |
| Full-text search | Member and event lists are small enough for DB queries with `LIKE` |
| Internationalization (i18n) of backend error messages | Error codes are machine-readable; human messages are Japanese by default |
| Mobile-specific API (e.g., push notification registration) | Web-only for now |
| Payment processing | No monetization |
| User-to-user messaging | Discord handles all messaging |
| Event RSVP / attendance tracking | May be added in a future phase |
| Multi-tenancy | Single community instance |
| Horizontal scaling / load balancing | Single Docker host (see A-004) |
| Blue-green or canary deployments | Simple restart deployment (see A-012) |
