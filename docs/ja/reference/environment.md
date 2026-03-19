# 環境変数クイックリファレンス

> **ナビゲーション**: [ドキュメントホーム](../README.md) > [リファレンス](README.md) > 環境変数

環境変数をカテゴリ別にまとめたクイックリファレンスです。各変数の詳細は[設定リファレンス](configuration.md)を参照してください。

## サーバー設定

| 変数名 | 必須 | デフォルト | 説明 |
|--------|------|---------|------|
| `BIND_ADDRESS` | いいえ | `0.0.0.0:3000` | サーバーがリッスンするアドレスとポート |

## データベース

| 変数名 | 必須 | デフォルト | 説明 |
|--------|------|---------|------|
| `DATABASE_URL` | **はい** | — | PostgreSQL 接続 URL |
| `DATABASE_MAX_CONNECTIONS` | いいえ | `10` | 接続プールの最大接続数 |

## Discord

| 変数名 | 必須 | デフォルト | 説明 |
|--------|------|---------|------|
| `DISCORD_CLIENT_ID` | **はい** | — | OAuth2 クライアント ID |
| `DISCORD_CLIENT_SECRET` | **はい** | — | OAuth2 クライアントシークレット |
| `DISCORD_GUILD_ID` | **はい** | — | 対象サーバーのギルド ID |
| `DISCORD_WEBHOOK_URL` | いいえ | なし | 通知送信用 Webhook URL |

## セキュリティ

| 変数名 | 必須 | デフォルト | 説明 |
|--------|------|---------|------|
| `SESSION_SECRET` | **はい** | — | セッション署名用シークレット（32文字以上） |
| `SYSTEM_API_TOKEN` | **はい** | — | System API 認証トークン（64文字以上） |
| `COOKIE_SECURE` | いいえ | `false` | Cookie の Secure フラグ（本番では `true`） |
| `TRUST_X_FORWARDED_FOR` | いいえ | `false` | リバースプロキシの X-Forwarded-For を信頼 |

## セッション

| 変数名 | 必須 | デフォルト | 説明 |
|--------|------|---------|------|
| `SESSION_MAX_AGE_SECS` | いいえ | `604800`（7日） | セッションの有効期間（秒） |
| `SESSION_CLEANUP_INTERVAL_SECS` | いいえ | `3600`（1時間） | 期限切れセッションの自動削除間隔（秒） |

## バックグラウンドタスク

| 変数名 | 必須 | デフォルト | 説明 |
|--------|------|---------|------|
| `EVENT_ARCHIVAL_INTERVAL_SECS` | いいえ | `86400`（1日） | 過去イベントの自動アーカイブ間隔（秒） |

## 認証

| 変数名 | 必須 | デフォルト | 説明 |
|--------|------|---------|------|
| `BACKEND_BASE_URL` | **はい** | — | OAuth2 コールバック URL の生成に使用 |
| `FRONTEND_ORIGIN` | **はい** | — | CORS 許可オリジン・認証後リダイレクト先 |

## 運用

| 変数名 | 必須 | デフォルト | 説明 |
|--------|------|---------|------|
| `SUPER_ADMIN_DISCORD_ID` | いいえ | なし | 起動時に super_admin ロールを付与する Discord ユーザー ID |
| `RUST_LOG` | いいえ | `vrc_backend=info,tower_http=info,sqlx=warn` | ログレベル設定（[EnvFilter 構文](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html)） |

## 最小構成例

開発環境で最低限必要な `.env` ファイルの例です:

```env
DATABASE_URL=postgres://vrc:password@localhost:5432/vrc_dev
DISCORD_CLIENT_ID=123456789012345678
DISCORD_CLIENT_SECRET=your-client-secret
DISCORD_GUILD_ID=987654321098765432
BACKEND_BASE_URL=http://localhost:3000
FRONTEND_ORIGIN=http://localhost:5173
SESSION_SECRET=ここに32文字以上のランダム文字列を設定
SYSTEM_API_TOKEN=ここに64文字以上のランダム文字列を設定
```

## 関連ドキュメント

- [設定リファレンス（詳細）](configuration.md)
- [エラーカタログ](errors.md)
- [用語集](glossary.md)
