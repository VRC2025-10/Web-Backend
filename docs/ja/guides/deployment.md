# デプロイメントガイド

> **対象読者**: オペレーター
>
> **ナビゲーション**: [ドキュメントホーム](../README.md) > [ガイド](README.md) > デプロイメント

## 概要

VRC Web-Backend は単一の Proxmox VM 上で Docker Compose スタックとして実行されます。本番スタックは 4 つのサービスで構成されます。

- Rust バックエンド
- Next.js フロントエンド
- PostgreSQL
- Caddy

Caddy は Cloudflare のプロキシ配下で動作し、Cloudflare Origin CA 証明書でオリジン TLS を終端します。

## 前提条件

- Proxmox VM または任意の Linux サーバー
- Docker Engine 24 以上
- Docker Compose 2.20 以上
- `vrcapi.arivell-vm.com` と `vrc10.arivell-vm.com` がサーバー IP を向いていること
- Cloudflare 側で両方の DNS レコードをプロキシ有効にしていること
- Cloudflare の SSL/TLS モードが `Full (strict)` であること
- Discord アプリケーションが設定済みであること

## シークレット管理

本番では `secrets/` 配下に以下を配置します。

```bash
mkdir -p secrets

openssl rand -base64 32 > secrets/db_password.txt
openssl rand -hex 64 > secrets/session_secret.txt
openssl rand -hex 64 > secrets/system_api_token.txt

# Cloudflare Dashboard > SSL/TLS > Origin Server の値を保存
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

> **重要**: `secrets/` はコミットしません。`.gitignore` で除外してください。

## 初回デプロイ

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

補足:

- `app` と `frontend` は `up -d` 時に自動ビルドされます
- バックエンドは起動時に自動で DB マイグレーションを実行します
- Cloudflare の WAF や Bot Fight Mode で Discord の OAuth コールバックを遮断しないでください

## アップデート

```bash
cd /opt/vrc-backend
git pull origin main
docker compose -f docker-compose.prod.yml up -d
curl -s https://vrcapi.arivell-vm.com/health | jq .
```

## Caddy 設定

現在の Caddyfile は Cloudflare Origin CA を使う前提です。

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

Caddy の役割:

- Cloudflare Origin CA 証明書でオリジン TLS を終端
- HTTP から HTTPS へのリダイレクト
- HTTP/3 の有効化
- `app` と `frontend` へのリバースプロキシ

## 関連ドキュメント

- [設定ガイド](configuration.md) — 環境変数とシークレット
- [セキュリティガイド](security.md) — セキュリティ強化
- [CI/CD](../development/ci-cd.md) — 自動化パイプライン
- [トラブルシューティング](troubleshooting.md) — デプロイ問題
