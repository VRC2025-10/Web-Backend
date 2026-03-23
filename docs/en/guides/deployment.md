# Deployment Guide

> **Audience**: Operators
>
> **Navigation**: [Docs Home](../README.md) > [Guides](README.md) > Deployment

## Overview

The VRC Web-Backend production setup runs as a Docker Compose stack on a single Proxmox VM. The stack contains four services:

- Rust backend
- Next.js frontend
- PostgreSQL
- Caddy

Caddy sits behind Cloudflare proxying and terminates origin TLS with a Cloudflare Origin CA certificate.

## Prerequisites

- Proxmox VM or another Linux server
- Docker Engine 24 or newer
- Docker Compose 2.20 or newer
- `vrcapi.arivell-vm.com` and `vrc10.arivell-vm.com` pointing to the server IP
- Cloudflare proxy enabled for both DNS records
- Cloudflare SSL/TLS mode set to `Full (strict)`
- Discord application configured

## Secrets Management

Store runtime secrets under `secrets/`:

```bash
mkdir -p secrets

openssl rand -base64 32 > secrets/db_password.txt
openssl rand -hex 64 > secrets/session_secret.txt
openssl rand -hex 64 > secrets/system_api_token.txt

# Save the values from Cloudflare Dashboard > SSL/TLS > Origin Server
cat > secrets/cloudflare-origin.crt << 'EOF'
-----BEGIN CERTIFICATE-----
...
-----END CERTIFICATE-----
EOF

cat > secrets/cloudflare-origin.key << 'EOF'
-----BEGIN PRIVATE KEY-----
...
-----END PRIVATE KEY-----
EOF

chmod 600 secrets/*
```

> **Important**: never commit `secrets/` to version control.

## Initial Deployment

```bash
git clone <repo-url> /opt/vrc-backend
cd /opt/vrc-backend

mkdir -p secrets
openssl rand -base64 32 > secrets/db_password.txt
openssl rand -hex 64 > secrets/session_secret.txt
openssl rand -hex 64 > secrets/system_api_token.txt
chmod 600 secrets/*

cat > .env << 'EOF'
DISCORD_CLIENT_ID=your_client_id
DISCORD_CLIENT_SECRET=your_client_secret
DISCORD_GUILD_ID=your_guild_id
BACKEND_BASE_URL=https://vrcapi.arivell-vm.com
DISCORD_REDIRECT_URI=https://vrcapi.arivell-vm.com/api/v1/auth/discord/callback
FRONTEND_ORIGIN=https://vrc10.arivell-vm.com
COOKIE_SECURE=true
TRUST_X_FORWARDED_FOR=true
EOF

docker compose -f docker-compose.prod.yml up -d
curl -s https://vrcapi.arivell-vm.com/health | jq .
```

Notes:

- `app` and `frontend` are built automatically during `up -d`
- the backend runs database migrations automatically on startup
- do not challenge the Discord OAuth routes with Cloudflare WAF or Bot Fight Mode

## Updating

```bash
cd /opt/vrc-backend
git pull origin main
docker compose -f docker-compose.prod.yml up -d
curl -s https://vrcapi.arivell-vm.com/health | jq .
```

## Caddy Configuration

The current Caddyfile is wired for the Cloudflare Origin CA certificate:

```caddyfile
(cloudflare_origin_tls) {
    tls /etc/caddy/certs/cloudflare-origin.crt /etc/caddy/certs/cloudflare-origin.key
}

vrcapi.arivell-vm.com {
    import cloudflare_origin_tls
    reverse_proxy app:3000
}

vrc10.arivell-vm.com {
    import cloudflare_origin_tls
    reverse_proxy frontend:3000
}
```

Caddy handles:

- origin TLS termination with the Cloudflare Origin CA certificate
- HTTP to HTTPS redirects
- HTTP/3 enablement
- reverse proxying to `app` and `frontend`

## Rollback

```bash
docker compose -f docker-compose.prod.yml logs app --tail=100
git checkout <previous-tag-or-commit>
docker compose -f docker-compose.prod.yml up -d
curl -s https://vrcapi.arivell-vm.com/health | jq .
```

## Health Check

```bash
curl -s https://vrcapi.arivell-vm.com/health
```

## Related Documents

- [Configuration Guide](configuration.md) — environment variables and secrets
- [Security Guide](security.md) — security hardening
- [Troubleshooting](troubleshooting.md) — deployment issues
- [Architecture Overview](../architecture/README.md) — system structure
