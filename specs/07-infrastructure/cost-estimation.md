# Cost Estimation

## Production Environment (Proxmox VM — Self-Hosted)

The backend runs on a VM inside an existing Proxmox VE server. No cloud VPS rental required.

| Resource | Provider | Spec | Monthly Cost |
|----------|----------|------|-------------|
| Proxmox VM | Self-hosted | 2 vCPU, 2 GB RAM, 40 GB SSD (from existing Proxmox pool) | $0 |
| Domain | Registrar | .com / .dev domain | ~$1/month (amortized) |
| Electricity (marginal) | — | ~5W incremental for one VM | ~$0.50/month |
| **Total** | | | **~$1.50/month** |

Caddy handles TLS (free via Let's Encrypt). No CDN, no R2/S3, no external cloud services.

## Development Environment

Free — Docker Compose runs locally on developer machine.

## CI/CD

| Resource | Provider | Monthly Cost |
|----------|----------|-------------|
| CI Minutes | GitHub Actions (Free tier) | $0 (2,000 min/month) |
| Container Registry | GHCR (Free for public repos) | $0 |
| **Total** | | **$0** |

## Cost Growth Projection

| Scale | Users | Monthly Requests | Infra Cost |
|-------|-------|-----------------|------------|
| Launch | ~100 | ~50K | ~$1.50 (domain only) |
| 6 months | ~300 | ~150K | ~$1.50 (same) |
| 1 year | ~500 | ~300K | ~$1.50 (same — increase VM allocation if needed) |
| Peak (around event) | ~500 concurrent | ~50K/day spike | ~$1.50 (same, handled by caching) |

The system is designed to handle this scale on a single Proxmox VM without any architectural changes. If the VM needs more resources, simply increase CPU/RAM allocation in the Proxmox UI.

## Cost Optimization Decisions

1. **Self-hosted Proxmox over cloud VPS**: Zero recurring compute cost. VM resources allocated from existing hardware pool.
2. **Local filesystem over R2/S3**: Gallery images stored on a Docker volume mounted from the VM disk. No egress fees, no API call costs, no external dependency.
3. **Caddy over Nginx**: Free auto-TLS, no separate certbot setup.
4. **No CDN needed**: API-only backend with local image serving via Caddy. Community size (~500) does not justify CDN.
5. **No managed monitoring**: Prometheus metrics exposed at `/metrics`; can be scraped by any Prometheus instance on the same Proxmox network.
5. **GitHub Actions free tier**: 2,000 minutes/month is more than enough for a project this size (typical build: ~10 min).
