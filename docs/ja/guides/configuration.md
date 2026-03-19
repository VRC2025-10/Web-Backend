# 設定ガイド

> **対象読者**: ユーザー、オペレーター
>
> **ナビゲーション**: [ドキュメントホーム](../README.md) > [ガイド](README.md) > 設定

## 概要

VRC Web-Backend は環境変数で設定され、通常はプロジェクトルートの `.env` ファイルで提供されます。設定は起動時にバリデーションされ、必須変数が欠落または無効な場合はアプリケーションの起動を拒否します。

## 設定の仕組み

```mermaid
graph TD
    ENV[".env ファイル"]
    SYS["システム環境変数"]
    APP["アプリケーション起動"]
    VAL["バリデーション"]

    ENV -->|"dotenvy で読み込み"| APP
    SYS -->|".env を上書き"| APP
    APP --> VAL

    VAL -->|"すべて有効"| RUN["アプリケーション起動 ✅"]
    VAL -->|"欠落/無効"| FAIL["起動失敗 ❌<br/>具体的エラーメッセージ"]
```

## 必須変数

| 変数 | 説明 | 例 | 制約 |
|-----|------|-----|------|
| `DATABASE_URL` | PostgreSQL 接続文字列 | `postgres://user:pass@localhost:5432/vrc` | 有効な PostgreSQL URL |
| `SESSION_SECRET` | セッション署名用 HMAC キー | (64文字以上のランダム16進文字列) | 最低64文字 |
| `DISCORD_CLIENT_ID` | Discord OAuth2 アプリケーション ID | `1234567890` | 空でないこと |
| `DISCORD_CLIENT_SECRET` | Discord OAuth2 アプリケーションシークレット | `AbCdEf...` | 空でないこと |
| `DISCORD_GUILD_ID` | 対象 Discord サーバー ID | `9876543210` | 空でないこと |
| `DISCORD_REDIRECT_URI` | OAuth2 コールバック URL | `https://api.example.com/api/v1/auth/callback` | 有効な URL |
| `FRONTEND_ORIGIN` | CORS/CSRF 用フロントエンド URL | `https://example.com` | 有効な URL、末尾スラッシュなし |
| `SYSTEM_API_TOKEN` | システム API 認証トークン | (64文字以上のランダム16進文字列) | 最低64文字 |

## オプション変数

| 変数 | 説明 | デフォルト | 値 |
|-----|------|---------|-----|
| `RUST_LOG` | ログレベルフィルタ | `info` | `trace`, `debug`, `info`, `warn`, `error` |
| `HOST` | バインドアドレス | `0.0.0.0` | 有効な IP アドレス |
| `PORT` | バインドポート | `3000` | 1-65535 |
| `COOKIE_SECURE` | Cookie に HTTPS を要求 | `true` | `true`, `false` |
| `TRUST_X_FORWARDED_FOR` | クライアント IP 用プロキシヘッダーを信頼 | `false` | `true`, `false` |
| `SESSION_MAX_AGE_HOURS` | セッション有効期間 | `168`（7日） | 正の整数 |

## シークレット生成

暗号学的に安全なシークレットを `openssl` で生成:

```bash
# SESSION_SECRET（64文字以上の16進文字列）
openssl rand -hex 64

# SYSTEM_API_TOKEN（64文字以上の16進文字列）
openssl rand -hex 64

# データベースパスワード
openssl rand -base64 32
```

> **警告**: 環境（開発、ステージング、本番）間でシークレットを再利用しないでください。各環境で新しいシークレットを生成してください。

## Discord アプリケーション設定

1. [Discord Developer Portal](https://discord.com/developers/applications) にアクセス
2. 新しいアプリケーションを作成（または既存を選択）
3. **OAuth2** 設定に移動
4. リダイレクト URI を追加: `https://your-domain.com/api/v1/auth/callback`
5. **Client ID** と **Client Secret** をコピー
6. 対象 Discord サーバーの **Guild ID** をメモ（サーバー名を右クリック → サーバー ID をコピー）

必要な OAuth2 スコープ: `identify`, `guilds`

## .env ファイルの例

```bash
# === 必須 ===
DATABASE_URL=postgres://postgres:postgres@localhost:5432/vrc_dev
SESSION_SECRET=<openssl rand -hex 64 の出力>
DISCORD_CLIENT_ID=your_client_id
DISCORD_CLIENT_SECRET=your_client_secret
DISCORD_GUILD_ID=your_guild_id
DISCORD_REDIRECT_URI=http://localhost:3000/api/v1/auth/callback
FRONTEND_ORIGIN=http://localhost:5173
SYSTEM_API_TOKEN=<openssl rand -hex 64 の出力>

# === オプション ===
RUST_LOG=debug
HOST=0.0.0.0
PORT=3000
COOKIE_SECURE=false
TRUST_X_FORWARDED_FOR=false
```

## 本番チェックリスト

| 設定 | 要件 | 理由 |
|-----|------|------|
| `COOKIE_SECURE` | `true` | Cookie は HTTPS のみで送信 |
| `TRUST_X_FORWARDED_FOR` | `true` | バックエンドは Caddy リバースプロキシの背後 |
| `FRONTEND_ORIGIN` | 実際の本番 URL | CORS と CSRF バリデーションが使用 |
| `DISCORD_REDIRECT_URI` | 本番コールバック URL | Discord Developer Portal と一致必須 |
| `RUST_LOG` | `info` または `warn` | 本番での冗長ログを回避 |
| `SESSION_SECRET` | 新しい64文字以上のシークレット | 本番環境に固有 |
| DB パスワード | 強力でユニーク | 開発デフォルトではないこと |

## 関連ドキュメント

- [デプロイメントガイド](deployment.md) — Docker による本番デプロイ
- [セキュリティガイド](security.md) — 設定のセキュリティ影響
- [環境変数リファレンス](../reference/environment.md) — 完全な変数一覧
- [トラブルシューティング](troubleshooting.md) — 設定関連の問題
