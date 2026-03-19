# API 使用例

> **ナビゲーション**: [ドキュメントホーム](../README.md) > [はじめに](README.md) > 使用例

VRC Backend API を操作するための curl 使用例集です。すべての例はサーバーが `http://localhost:3000` で実行されていることを前提としています。

---

## 目次

- [ヘルスチェック](#ヘルスチェック)
- [Public API](#public-api) — 認証不要
- [Auth フロー](#auth-フロー) — Discord OAuth2 ログイン
- [Internal API](#internal-api) — セッション Cookie が必要
- [System API](#system-api) — Bearer トークンが必要
- [Admin API](#admin-api) — 管理者セッションが必要

---

## ヘルスチェック

```bash
curl http://localhost:3000/health
```

データベース接続状態を含むサーバーのヘルスステータスを返します。

---

## Public API

公開エンドポイントには認証が不要です。ベースパス: `/api/v1/public`

### メンバー一覧（ページネーション付き）

```bash
# デフォルトのページネーション
curl http://localhost:3000/api/v1/public/members

# ページネーションパラメータ付き
curl "http://localhost:3000/api/v1/public/members?page=1&per_page=20"
```

### メンバー詳細

```bash
curl http://localhost:3000/api/v1/public/members/{discord_id}
```

### イベント一覧

```bash
# 全イベント
curl http://localhost:3000/api/v1/public/events

# ステータスでフィルタ
curl "http://localhost:3000/api/v1/public/events?status=published"
```

### イベント詳細

```bash
curl http://localhost:3000/api/v1/public/events/{event_id}
```

### クラブ一覧

```bash
curl http://localhost:3000/api/v1/public/clubs
```

### クラブ詳細

```bash
curl http://localhost:3000/api/v1/public/clubs/{club_id}
```

### ギャラリー画像一覧

```bash
curl http://localhost:3000/api/v1/public/clubs/{club_id}/gallery
```

---

## Auth フロー

VRC Backend は認証に Discord OAuth2 を使用します。ベースパス: `/api/v1/auth`

### ステップ 1: ログインの開始

ユーザーのブラウザをログインエンドポイントにリダイレクトします：

```bash
# Discord の OAuth2 認可ページへのリダイレクトが返されます
curl -v http://localhost:3000/api/v1/auth/discord/login
```

ウェブアプリケーションでは、ユーザーを以下の URL に遷移させます：

```
http://localhost:3000/api/v1/auth/discord/login
```

### ステップ 2: Discord での認可

ユーザーが Discord 上でアプリケーションを認可します。Discord は設定された `DISCORD_REDIRECT_URI` にリダイレクトします。

### ステップ 3: コールバック

コールバックはサーバーによって自動的に処理されます：

```
GET /api/v1/auth/discord/callback?code=<authorization_code>&state=<state>
```

成功時、サーバーは以下を実行します：
1. 認可コードを Discord アクセストークンに交換
2. ユーザーの Discord プロフィールとギルドメンバーシップを取得
3. ユーザーレコードの作成または更新
4. セッション Cookie の設定
5. `FRONTEND_ORIGIN` へリダイレクト

### ステップ 4: セッションの使用

ログイン後、セッション Cookie はブラウザリクエストに自動的に含まれます。curl の場合は Cookie を保存して再利用します：

```bash
# Auth フローから Cookie を保存
curl -c cookies.txt -L http://localhost:3000/api/v1/auth/discord/login
# （ブラウザで Discord OAuth2 フローを完了）

# セッション Cookie を使って認証済みリクエストを送信
curl -b cookies.txt http://localhost:3000/api/v1/internal/auth/me
```

---

## Internal API

内部エンドポイントには認証済みセッション（Cookie）が必要です。ベースパス: `/api/v1/internal`

> **注意**: `-b cookies.txt` は実際のセッション Cookie に置き換えてください。[Auth フロー](#auth-フロー) を完了することで取得できます。

### 現在のユーザー情報を取得

```bash
curl -b cookies.txt http://localhost:3000/api/v1/internal/auth/me
```

### プロフィールの取得

```bash
curl -b cookies.txt http://localhost:3000/api/v1/internal/me/profile
```

### プロフィールの更新

```bash
curl -b cookies.txt \
  -X PUT \
  -H "Content-Type: application/json" \
  -d '{
    "nickname": "新しいニックネーム",
    "bio_markdown": "更新された自己紹介文"
  }' \
  http://localhost:3000/api/v1/internal/me/profile
```

### レポートの作成

```bash
curl -b cookies.txt \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{
    "target_type": "user",
    "target_id": "対象のUUID",
    "reason": "通報理由をここに記載（10〜1000文字）"
  }' \
  http://localhost:3000/api/v1/internal/reports
```

---

## System API

System エンドポイントはサービス間通信用で、Bearer トークン（`SYSTEM_API_TOKEN`）が必要です。ベースパス: `/api/v1/system`

### イベントの同期（Upsert）

```bash
curl -X POST \
  -H "Authorization: Bearer $SYSTEM_API_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "external_id": "evt-discord-001",
    "title": "週次ミートアップ",
    "description_markdown": "毎週のコミュニティ集会です",
    "status": "published",
    "host_discord_id": "123456789012345678",
    "start_time": "2026-03-20T18:00:00Z",
    "end_time": "2026-03-20T20:00:00Z",
    "location": "VRChat ワールド",
    "tags": ["meetup", "weekly"]
  }' \
  http://localhost:3000/api/v1/system/events
```

### メンバー離脱の処理

```bash
curl -X POST \
  -H "Authorization: Bearer $SYSTEM_API_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "discord_id": "123456789012345678",
    "reason": "left_guild"
  }' \
  http://localhost:3000/api/v1/system/sync/users/leave
```

---

## Admin API

Admin エンドポイントには管理者権限を持つ認証済みセッションが必要です。ベースパス: `/api/v1/internal/admin`

> **注意**: ユーザーアカウントに管理者ロールが必要です。最初の管理者をブートストラップするには、`.env` に `SUPER_ADMIN_DISCORD_ID` として自分の Discord ID を設定してください。

### ユーザー一覧（フィルタ付き）

```bash
# 全ユーザー
curl -b cookies.txt http://localhost:3000/api/v1/internal/admin/users

# ロールでフィルタ
curl -b cookies.txt "http://localhost:3000/api/v1/internal/admin/users?role=member"

# ステータスでフィルタ
curl -b cookies.txt "http://localhost:3000/api/v1/internal/admin/users?status=active"

# ページネーション付き
curl -b cookies.txt "http://localhost:3000/api/v1/internal/admin/users?page=1&per_page=25"
```

### ユーザーロールの変更

```bash
curl -b cookies.txt \
  -X PATCH \
  -H "Content-Type: application/json" \
  -d '{
    "new_role": "staff"
  }' \
  http://localhost:3000/api/v1/internal/admin/users/{user_id}/role
```

### ユーザーステータスの変更

```bash
curl -b cookies.txt \
  -X PATCH \
  -H "Content-Type: application/json" \
  -d '{
    "new_status": "suspended"
  }' \
  http://localhost:3000/api/v1/internal/admin/users/{user_id}/status
```

---

## ヒント

- **ページネーション**: ほとんどのリストエンドポイントは `?page=N&per_page=N` クエリパラメータに対応しています。
- **JSON レスポンス**: すべての API レスポンスは JSON 形式です。`jq` を使うと見やすく表示できます：
  ```bash
  curl -s http://localhost:3000/api/v1/public/members | jq .
  ```
- **詳細出力**: `curl -v` でヘッダーとステータスコードを確認できます。
- **環境変数**: System トークンをエクスポートしておくと便利です：
  ```bash
  export SYSTEM_API_TOKEN=$(grep SYSTEM_API_TOKEN .env | cut -d= -f2)
  ```

---

## 関連ドキュメント

- [Public API リファレンス](../reference/api/public.md)
- [Internal API リファレンス](../reference/api/internal.md)
- [System API リファレンス](../reference/api/system.md)
- [Auth API リファレンス](../reference/api/auth.md)
- [Admin API リファレンス](../reference/api/admin.md)
- [エラーカタログ](../reference/errors.md)
