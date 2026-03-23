# 設定リファレンス

> **ナビゲーション**: [ドキュメントホーム](../README.md) > [リファレンス](README.md) > 設定

全ての設定は環境変数で行います。`.env` ファイルまたはシステム環境変数で設定してください。

## 必須環境変数

| 変数名 | 型 | バリデーション | 説明 |
|--------|-----|-------------|------|
| `DATABASE_URL` | String | PostgreSQL URL形式 | PostgreSQL 接続 URL（例: `postgres://user:pass@localhost:5432/db`） |
| `DISCORD_CLIENT_ID` | String | 非空 | Discord OAuth2 クライアント ID |
| `DISCORD_CLIENT_SECRET` | Secret | 非空 | Discord OAuth2 クライアントシークレット |
| `DISCORD_GUILD_ID` | String | 非空 | 対象 Discord サーバーのギルド ID |
| `BACKEND_BASE_URL` | String | URL形式 | バックエンドのベース URL（例: `https://api.example.com`） |
| `FRONTEND_ORIGIN` | String | Origin形式 | フロントエンドのオリジン URL（CORS 用、例: `https://example.com`） |
| `SESSION_SECRET` | Secret | 32文字以上 | セッション署名シークレット |
| `SYSTEM_API_TOKEN` | Secret | 64文字以上 | System API 認証トークン |

## オプション環境変数

| 変数名 | 型 | デフォルト | 説明 |
|--------|-----|---------|------|
| `BIND_ADDRESS` | String | `0.0.0.0:3000` | サーバーバインドアドレス |
| `DATABASE_MAX_CONNECTIONS` | u32 | `10` | データベース接続プール最大数 |
| `SESSION_MAX_AGE_SECS` | i64 | `604800`（7日） | セッション有効期間（秒） |
| `SESSION_CLEANUP_INTERVAL_SECS` | u64 | `3600`（1時間） | 期限切れセッション削除間隔（秒） |
| `EVENT_ARCHIVAL_INTERVAL_SECS` | u64 | `86400`（1日） | イベント自動アーカイブ間隔（秒） |
| `SUPER_ADMIN_DISCORD_ID` | String | なし | ログイン時に `super_admin` を付与する対象の Discord ID |
| `DISCORD_WEBHOOK_URL` | String | なし | Discord Webhook URL（通知用） |
| `COOKIE_SECURE` | bool | `false` | Cookie に Secure フラグを付与（本番では `true`） |
| `TRUST_X_FORWARDED_FOR` | bool | `false` | X-Forwarded-For ヘッダーを信頼（リバースプロキシ経由時 `true`） |
| `RUST_LOG` | String | `vrc_backend=info,tower_http=info,sqlx=warn` | ログレベル設定 |

## シークレットの生成

```bash
# SESSION_SECRET（32バイト以上）
openssl rand -hex 32

# SYSTEM_API_TOKEN（64バイト以上）
openssl rand -hex 64
```

## 本番環境チェックリスト

- [ ] `COOKIE_SECURE=true`
- [ ] `TRUST_X_FORWARDED_FOR=true`（Caddy 経由の場合）
- [ ] `FRONTEND_ORIGIN` が正しいドメインに設定されている
- [ ] `SESSION_SECRET` と `SYSTEM_API_TOKEN` が十分な長さで安全に生成されている
- [ ] `DATABASE_URL` が本番データベースを指している
- [ ] `DISCORD_WEBHOOK_URL` が設定されている（通知が必要な場合）
- [ ] `SUPER_ADMIN_DISCORD_ID` が適切に設定されている
- [ ] `RUST_LOG` が適切なレベルに設定されている

## 設定バリデーション

起動時に以下のバリデーションが実行されます:

- 必須環境変数の存在確認
- `SESSION_SECRET` が32文字以上であること
- `SYSTEM_API_TOKEN` が64文字以上であること
- `FRONTEND_ORIGIN` が有効な Origin 形式であること
- 数値パラメータが正の値であること

バリデーション失敗時はエラーメッセージとともにプロセスが終了します。

## 関連ドキュメント

- [環境変数クイックリファレンス](environment.md)
- [設定ガイド](../guides/configuration.md)
- [デプロイガイド](../guides/deployment.md)
