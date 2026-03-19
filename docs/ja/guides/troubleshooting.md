# トラブルシューティングガイド

> **対象読者**: 全員
>
> **ナビゲーション**: [ドキュメントホーム](../README.md) > [ガイド](README.md) > トラブルシューティング

## 概要

VRC Web-Backend の開発、ビルド、実行中に遭遇する一般的な問題の症状、原因、解決策を説明します。

---

## データベース問題

### データベース接続拒否

| | 詳細 |
|---|------|
| **症状** | `error connecting to database: Connection refused (os error 111)` |
| **原因** | PostgreSQL が起動していないか、期待されるホスト/ポートで接続を受け付けていない |
| **解決策** | |

```bash
# PostgreSQL の起動確認
docker compose ps postgres

# 起動していない場合
docker compose up -d postgres

# 接続確認
docker compose exec postgres pg_isready -U postgres

# .env の DATABASE_URL を確認
grep DATABASE_URL .env
```

---

### マイグレーション失敗

| | 詳細 |
|---|------|
| **症状** | `sqlx migrate run` が `connection refused` または `database does not exist` で失敗 |
| **原因** | データベースに到達できないか、対象データベースが未作成 |
| **解決策** | |

```bash
# データベースの作成
docker compose exec postgres createdb -U postgres vrc_dev

# マイグレーション実行
cargo sqlx migrate run
```

---

## 認証問題

### CSRF バリデーション失敗

| | 詳細 |
|---|------|
| **症状** | POST/PUT/DELETE リクエストで `403 Forbidden`（エラーコード `ERR-CSRF-001`） |
| **原因** | リクエストの `Origin` ヘッダーがバックエンド設定の `FRONTEND_ORIGIN` と一致しない |
| **解決策** | |

```bash
# FRONTEND_ORIGIN の確認
grep FRONTEND_ORIGIN .env

# よくある間違い:
#   - 末尾スラッシュ: https://example.com/（間違い）
#   - ポート不一致:    http://localhost:3000（間違い、フロントエンドポートにすべき）
#   - プロトコル不一致: http:// vs https://
```

---

### ログイン失敗 — ギルド未加入

| | 詳細 |
|---|------|
| **症状** | ログインがエラー付きでフロントエンドにリダイレクト、または `403 Forbidden`（`ERR-AUTH-003`） |
| **原因** | Discord ユーザーが設定されたギルド（`DISCORD_GUILD_ID`）のメンバーではない |
| **解決策** | |

1. ユーザーが Discord サーバーに参加していることを確認
2. `DISCORD_GUILD_ID` が正しいことを確認
3. Discord 開発者モードを有効化（ユーザー設定 → 詳細設定 → 開発者モード）
4. サーバー名を右クリック → サーバー ID をコピー
5. 設定された `DISCORD_GUILD_ID` と比較

---

### セッション無効

| | 詳細 |
|---|------|
| **症状** | 以前動作していた認証済みエンドポイントで `401 Unauthorized` |
| **原因** | セッションの期限切れ、手動失効、またはセッションテーブルのクリア |
| **解決策** | |

1. ログアウトして再ログインし新しいセッションを作成
2. セッション有効期限設定を確認:
   ```bash
   grep SESSION_MAX_AGE_HOURS .env
   # デフォルトは 168 時間（7日）
   ```

---

## システム API 問題

### システムトークン無効

| | 詳細 |
|---|------|
| **症状** | システム API エンドポイントで `401 Unauthorized`（エラーコード `ERR-SYNC-001`） |
| **原因** | `Authorization: Bearer <token>` ヘッダーが `SYSTEM_API_TOKEN` と一致しない |
| **解決策** | |

```bash
# トークンの一致確認
grep SYSTEM_API_TOKEN .env | wc -c

# curl でテスト
curl -v -H "Authorization: Bearer $(grep SYSTEM_API_TOKEN .env | cut -d= -f2)" \
  http://localhost:3000/api/v1/system/events
```

---

### レート制限超過

| | 詳細 |
|---|------|
| **症状** | `429 Too Many Requests` レスポンス |
| **原因** | エンドポイントティアのレート制限を超過 |
| **解決策** | |

レート制限ウィンドウのリセットを待ちます。ティアごとの制限:

| ティア | 制限 | リセットウィンドウ |
|-------|------|-----------------|
| Public | 60リクエスト | 1分あたり |
| Internal | 120リクエスト | 1分あたり |
| System | 30リクエスト | 1分あたり |
| Auth | 10リクエスト | 1分あたり |

---

## ビルド問題

### SQLx ビルドエラー

| | 詳細 |
|---|------|
| **症状** | `cargo build` 中に `error communicating with database` |
| **原因** | SQLx コンパイル時クエリに PostgreSQL の実行またはオフラインメタデータが必要 |
| **解決策** | |

**方法 A: データベースを起動**
```bash
docker compose up -d postgres
export DATABASE_URL=postgres://postgres:postgres@localhost:5432/vrc_dev
cargo build
```

**方法 B: オフラインモードを使用**
```bash
export SQLX_OFFLINE=true
cargo build
```

**方法 C: オフラインメタデータを再生成**
```bash
docker compose up -d postgres
cargo sqlx prepare
git add .sqlx/
git commit -m "Update sqlx offline metadata"
```

## 関連ドキュメント

- [設定ガイド](configuration.md) — 環境変数の設定
- [デプロイメントガイド](deployment.md) — 本番デプロイ
- [セキュリティガイド](security.md) — セキュリティ問題
- [エラーリファレンス](../reference/errors.md) — 全エラーコード一覧
