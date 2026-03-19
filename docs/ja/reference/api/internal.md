# Internal API リファレンス

> **ナビゲーション**: [ドキュメントホーム](../../README.md) > [リファレンス](../README.md) > [API](README.md) > Internal API

Internal API は認証済みユーザー向けのエンドポイントを提供します。すべてのリクエストには Discord OAuth2 ログイン後に設定される有効な**セッション Cookie** が必要です。レート制限: **ユーザーあたり 120リクエスト/分**。CSRF 保護: `Origin` ヘッダーが `FRONTEND_ORIGIN` と一致する必要があります。

---

## 認証

### 現在のユーザー情報を取得

```
GET /api/v1/internal/auth/me
```

現在の認証済みユーザーの情報とプロフィールサマリーを返します。

**レスポンス** `200 OK`

```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "discord_id": "123456789012345678",
  "discord_username": "PlayerOne",
  "avatar_url": "https://cdn.discordapp.com/avatars/...",
  "role": "member",
  "status": "active",
  "joined_at": "2025-01-15T09:30:00Z",
  "profile": {
    "nickname": "PlayerOne",
    "bio_markdown": "こんにちは！",
    "is_public": true
  }
}
```

**エラーコード**

| コード | ステータス | 原因 |
|-------|-----------|------|
| `ERR-AUTH-003` | 401 | 無効または期限切れのセッション |
| `ERR-AUTH-004` | 403 | アカウントが停止されている |

---

### ログアウト

```
POST /api/v1/internal/auth/logout
```

現在のセッションを破棄し、セッション Cookie をクリアします。

**レスポンス** `204 No Content`

レスポンスボディはありません。

**エラーコード**

| コード | ステータス | 原因 |
|-------|-----------|------|
| `ERR-AUTH-003` | 401 | 無効または期限切れのセッション |
| `ERR-CSRF-001` | 403 | CSRF 検証に失敗 |

---

## プロフィール

### 自分のプロフィールを取得

```
GET /api/v1/internal/me/profile
```

認証済みユーザーの完全なプロフィールを返します。

**レスポンス** `200 OK`

```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "nickname": "PlayerOne",
  "vrc_id": "usr_xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
  "x_id": "player_one",
  "bio_markdown": "VRChat 愛好家。ワールド探検が趣味です。",
  "avatar_url": "https://cdn.discordapp.com/avatars/...",
  "is_public": true,
  "updated_at": "2026-03-01T10:00:00Z"
}
```

**エラーコード**

| コード | ステータス | 原因 |
|-------|-----------|------|
| `ERR-AUTH-003` | 401 | 無効または期限切れのセッション |
| `ERR-PROF-003` | 404 | プロフィールが見つからない |

---

### 自分のプロフィールを更新

```
PUT /api/v1/internal/me/profile
```

認証済みユーザーのプロフィールを更新します。リクエストボディのすべてのフィールドはオプションです — 指定されたフィールドのみ更新されます。

**リクエストボディ**

```json
{
  "nickname": "新しいニックネーム",
  "vrc_id": "usr_xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
  "x_id": "new_x_handle",
  "bio_markdown": "更新された自己紹介文。",
  "avatar_url": "https://cdn.discordapp.com/avatars/...",
  "is_public": true
}
```

**フィールドバリデーションルール**

| フィールド | 型 | 必須 | バリデーション |
|-----------|-----|------|--------------|
| `nickname` | string \| null | いいえ | 指定時は1〜50文字。前後の空白はトリム。前後の空白文字は不可 |
| `vrc_id` | string \| null | いいえ | VRChat ユーザー ID 形式に一致すること: `usr_` に UUID が続く形式（`usr_xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`） |
| `x_id` | string \| null | いいえ | 1〜15文字。英数字とアンダースコアのみ（`[a-zA-Z0-9_]`） |
| `bio_markdown` | string \| null | いいえ | 最大2000文字。危険なコンテンツ（スクリプト、data URI など）がスキャンされる |
| `avatar_url` | string \| null | いいえ | 有効な HTTPS URL であること。許可されたドメインのみ（Discord CDN）。SSRF 防止が適用される |
| `is_public` | boolean | いいえ | Public API でプロフィールを表示するかを制御 |

**レスポンス** `200 OK`

[自分のプロフィールを取得](#自分のプロフィールを取得) と同じ形式で更新後のプロフィールを返します。

**エラーコード**

| コード | ステータス | 原因 |
|-------|-----------|------|
| `ERR-AUTH-003` | 401 | 無効または期限切れのセッション |
| `ERR-CSRF-001` | 403 | CSRF 検証に失敗 |
| `ERR-PROF-001` | 400 | プロフィールバリデーション失敗（`error.details` にフィールドレベルの詳細あり） |
| `ERR-PROF-002` | 400 | `bio_markdown` に危険なコンテンツが検出された |
| `ERR-PROF-003` | 404 | プロフィールが見つからない |

**エラーレスポンス例（バリデーション）**

```json
{
  "error": {
    "code": "ERR-PROF-001",
    "message": "プロフィールのバリデーションに失敗しました",
    "details": {
      "nickname": "50文字以内で入力してください",
      "vrc_id": "VRChat IDの形式が正しくありません"
    }
  }
}
```

---

## イベント

### イベント一覧（内部）

```
GET /api/v1/internal/events
```

認証済みユーザーに表示可能なイベントのページネーション付きリストを返します。Public API では利用できない追加ステータスが含まれる場合があります。

**クエリパラメータ**

| パラメータ | 型 | デフォルト | 説明 |
|-----------|-----|----------|------|
| `page` | integer | 1 | ページ番号（最小: 1） |
| `per_page` | integer | 20 | 1ページあたりのアイテム数（最小: 1、最大: 100） |
| `status` | string | — | イベントステータスでフィルタ |

**レスポンス** `200 OK`

```json
{
  "items": [
    {
      "event_id": "550e8400-e29b-41d4-a716-446655440000",
      "external_id": "evt-discord-001",
      "title": "VRChat 週次ミートアップ",
      "description_markdown": "毎週のコミュニティ集会に参加しましょう！",
      "status": "published",
      "host_discord_id": "123456789012345678",
      "start_time": "2026-03-20T20:00:00Z",
      "end_time": "2026-03-20T22:00:00Z",
      "location": "VRChat - コミュニティワールド",
      "tags": ["meetup", "weekly"]
    }
  ],
  "total_count": 20,
  "total_pages": 1
}
```

**エラーコード**

| コード | ステータス | 原因 |
|-------|-----------|------|
| `ERR-AUTH-003` | 401 | 無効または期限切れのセッション |
| `ERR-VALIDATION` | 400 | 無効なクエリパラメータ |
| `ERR-RATELIMIT-001` | 429 | レート制限超過 |

---

## レポート

### レポートの作成

```
POST /api/v1/internal/reports
```

ユーザー、イベント、クラブ、その他のエンティティに対するレポートを作成します。重複レポート（同じ通報者 + 同じ対象）は拒否されます。

**リクエストボディ**

```json
{
  "target_type": "user",
  "target_id": "550e8400-e29b-41d4-a716-446655440000",
  "reason": "イベント中の不適切な行為。"
}
```

**フィールドバリデーションルール**

| フィールド | 型 | 必須 | バリデーション |
|-----------|-----|------|--------------|
| `target_type` | string | はい | `user`、`event`、`club`、`gallery_image` のいずれか |
| `target_id` | string (UUID) | はい | 既存のエンティティを参照すること |
| `reason` | string | はい | 10〜1000文字 |

**レスポンス** `201 Created`

```json
{
  "report_id": "550e8400-e29b-41d4-a716-446655440099",
  "target_type": "user",
  "target_id": "550e8400-e29b-41d4-a716-446655440000",
  "reason": "イベント中の不適切な行為。",
  "status": "pending",
  "created_at": "2026-03-19T12:00:00Z"
}
```

**エラーコード**

| コード | ステータス | 原因 |
|-------|-----------|------|
| `ERR-AUTH-003` | 401 | 無効または期限切れのセッション |
| `ERR-CSRF-001` | 403 | CSRF 検証に失敗 |
| `ERR-MOD-001` | 404 | 報告対象が見つからない |
| `ERR-MOD-002` | 409 | 同一対象への重複レポート |
| `ERR-MOD-003` | 400 | 理由の文字数が範囲外（10〜1000文字） |
| `ERR-VALIDATION` | 400 | 無効な target_type または必須フィールドの欠落 |
| `ERR-RATELIMIT-001` | 429 | レート制限超過 |

---

## 関連ドキュメント

- [API 概要](README.md) — ベース URL、ページネーション、認証方式
- [Admin API](admin.md) — 管理エンドポイント（Admin+ または Staff+ が必要）
- [Auth API](auth.md) — Discord OAuth2 ログインフロー
- [エラーカタログ](../errors.md) — 全エラーコードのリファレンス
