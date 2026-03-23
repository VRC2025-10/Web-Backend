# セットアップガイド

## 前提条件

- Rust toolchain (stable)
- PostgreSQL 15+
- Discord Application (OAuth2 + Bot)

## 環境変数

`.env.example` をコピーして `.env` を作成する。

```bash
cp .env.example .env
```

### 必須

| 変数名 | 説明 | バリデーション |
|---|---|---|
| `DATABASE_URL` | PostgreSQL 接続 URL | `postgres://user:pass@host:port/db` |
| `DISCORD_CLIENT_ID` | Discord OAuth2 クライアント ID | — |
| `DISCORD_CLIENT_SECRET` | Discord OAuth2 クライアントシークレット | — |
| `DISCORD_REDIRECT_URI` | OAuth2 コールバック URL | 例: `https://vrcapi.arivell-vm.com/api/v1/auth/discord/callback` |
| `DISCORD_BOT_TOKEN` | Discord Bot トークン | — |
| `DISCORD_GUILD_ID` | 対象ギルド ID | — |
| `DISCORD_WEBHOOK_URL` | 通知用 Webhook URL | — |
| `SYSTEM_API_TOKEN` | System API 認証トークン | **最低64文字** |
| `SESSION_SECRET` | セッション署名シークレット | **最低32文字** |
| `FRONTEND_ORIGIN` | フロントエンド URL | 例: `https://vrc10.arivell-vm.com` |

### オプション

| 変数名 | デフォルト | 説明 |
|---|---|---|
| `BACKEND_PORT` | `8080` | サーバーポート |
| `SESSION_MAX_AGE_SECS` | `604800` (7日) | セッション有効期間（秒） |
| `ALLOWED_ORIGINS` | `""` | 追加 CORS オリジン（カンマ区切り） |
| `TRUST_X_FORWARDED_FOR` | `false` | リバースプロキシ背後なら `true` |
| `SESSION_CLEANUP_INTERVAL_SECS` | `3600` | 期限切れセッション掃除間隔（秒） |
| `MEMBER_SYNC_INTERVAL_SECS` | `3600` | メンバー同期間隔（秒） |
| `EVENT_ARCHIVE_INTERVAL_SECS` | `86400` | イベントアーカイブ間隔（秒） |
| `SUPER_ADMIN_DISCORD_ID` | *(なし)* | ログイン時に `super_admin` を付与する Discord ID。複数指定時はカンマ区切り |

## トークンの生成方法

### SYSTEM_API_TOKEN

System API の認証に使う静的トークン。GAS や Bot から呼ぶときに `Authorization: Bearer <token>` ヘッダに設定する。

```bash
# 64文字以上のランダム文字列を生成
openssl rand -base64 48
```

生成した値を `.env` の `SYSTEM_API_TOKEN` に設定し、呼び出し側（GAS の Script Properties 等）にも同じ値を設定する。

**セキュリティ仕様:**
- サーバー側では SHA-256 ハッシュ後に定数時間比較（`subtle::ConstantTimeEq`）で検証
- タイミング攻撃やブルートフォースに対して安全
- ログには成功/失敗のみ記録し、トークン値は出力しない

### SESSION_SECRET

```bash
openssl rand -base64 24
```

## データベースのセットアップ

```bash
# マイグレーション実行
sqlx migrate run

# マイグレーション一覧
# 01: ENUMs (user_role, user_status, event_status, report_status, report_target_type)
# 02: users
# 03: profiles
# 04: sessions
# 05: event_tags
# 06: events
# 07: event_tag_mappings
# 08: reports
# 09: super_admin ロール追加 (user_role ENUM 拡張)
# 10: clubs テーブル
# 11: club_members テーブル
# 12: gallery_images テーブル + gallery_image_status ENUM
```

### SuperAdmin の初回昇格

環境変数 `SUPER_ADMIN_DISCORD_ID` が設定されている場合、その Discord ID で実際にログインしたユーザーが `super_admin` に昇格します。複数人を指定する場合はカンマ区切りで設定できます。

- サーバー起動時にダミーの `users` / `profiles` レコードは作成しません
- 指定された Discord ID のユーザーが OAuth コールバックでログインしたタイミングで、`super_admin` ロールが付与されます
- 既にユーザーが存在する場合は、その既存レコードが `super_admin` に昇格します
- 旧バージョンが作った未使用のダミー SuperAdmin レコードは、起動時に安全条件を満たす場合のみ自動削除されます

```bash
# .env に追加
SUPER_ADMIN_DISCORD_ID=123456789012345678,987654321098765432
```

## 起動

```bash
# 開発
cargo run

# リリースビルド
cargo build --release
./target/release/vrc-backend
```

## テスト

```bash
# 全テスト実行
cargo test

# 特定テストスイートのみ
cargo test --test api_public
cargo test --test api_internal
cargo test --test api_system
cargo test --test middleware_tests
```

## Docker

```bash
docker build -t vrc-backend .
docker run --env-file .env -p 8080:8080 vrc-backend
```

## 認証の仕組み

### ユーザー認証 (Discord OAuth2)

```
ブラウザ → GET /api/v1/auth/discord/login
        ← 302 → Discord 認可画面
        → ユーザーが承認
        ← GET /api/v1/auth/discord/callback?code=...&state=...
           1. CSRF state 検証
           2. code → アクセストークン交換
           3. Discord ユーザー情報取得
           4. ギルドメンバーシップ確認
           5. ユーザー UPSERT + セッション生成
        ← 302 → フロントエンド (session_id Cookie 付き)
```

**セッション:**
- Cookie 名: `session_id`
- `HttpOnly`, `SameSite=Lax`, `Path=/`
- `Secure` は `FRONTEND_ORIGIN` が `https` の場合のみ
- 有効期間の半分が過ぎるとスライディングウィンドウで自動延長

### System API 認証 (Bearer Token)

```
GAS/Bot → POST /api/v1/system/events
          Authorization: Bearer <SYSTEM_API_TOKEN>
       ← 200 OK / 401 Unauthorized
```

固定トークンによる M2M (Machine-to-Machine) 認証。トークンのローテーションは手動で `.env` を更新してサーバーを再起動する。

## セキュリティ

### レート制限

| レイヤー | レート | キー |
|---|---|---|
| Public | 60 req/min | IP アドレス |
| Internal | 120 req/min | user_id |
| System | 30 req/min | 固定キー |
| Auth | 10 req/min | IP アドレス |

超過時は `429 Too Many Requests` + `Retry-After: 60` ヘッダ。

### CORS

- 許可オリジン: `FRONTEND_ORIGIN` + `ALLOWED_ORIGINS`
- メソッド: `GET`, `POST`, `PUT`, `DELETE`, `OPTIONS`
- クレデンシャル: 許可 (`credentials: include` が必要)
- プリフライトキャッシュ: 1時間

### セキュリティヘッダ (全レスポンス)

| ヘッダ | 値 |
|---|---|
| `X-Content-Type-Options` | `nosniff` |
| `X-Frame-Options` | `DENY` |
| `Strict-Transport-Security` | `max-age=63072000; includeSubDomains` |
| `Content-Security-Policy` | `default-src 'none'; frame-ancestors 'none'` |

### CSRF 対策

Internal API の状態変更リクエスト（POST/PUT/DELETE）は `Origin` ヘッダを `FRONTEND_ORIGIN` と照合して検証する。
