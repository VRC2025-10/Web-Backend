# Public API リファレンス

> **ナビゲーション**: [ドキュメントホーム](../../README.md) > [リファレンス](../README.md) > [API](README.md) > Public API

Public API はコミュニティデータへの読み取り専用アクセスを認証なしで提供します。すべてのエンドポイントは **IP あたり 60リクエスト/分** でレート制限され、`Cache-Control: public, max-age=30` で配信されます。

---

## Members（メンバー）

### メンバー一覧

```
GET /api/v1/public/members
```

公開メンバーサマリーのページネーション付きリストを返します。

**クエリパラメータ**

| パラメータ | 型 | デフォルト | 説明 |
|-----------|-----|----------|------|
| `page` | integer | 1 | ページ番号（最小: 1） |
| `per_page` | integer | 20 | 1ページあたりのアイテム数（最小: 1、最大: 100） |

**レスポンス** `200 OK`

```json
{
  "items": [
    {
      "discord_id": "123456789012345678",
      "nickname": "PlayerOne",
      "avatar_url": "https://cdn.discordapp.com/avatars/...",
      "joined_at": "2025-01-15T09:30:00Z"
    }
  ],
  "total_count": 42,
  "total_pages": 3
}
```

**レスポンスヘッダー**

| ヘッダー | 例 |
|---------|-----|
| `x-total-count` | `42` |
| `x-total-pages` | `3` |

**エラーコード**

| コード | ステータス | 原因 |
|-------|-----------|------|
| `ERR-VALIDATION` | 400 | 無効なページネーションパラメータ |
| `ERR-RATELIMIT-001` | 429 | レート制限超過 |

---

### メンバー詳細

```
GET /api/v1/public/members/{discord_id}
```

特定のメンバーの公開プロフィールを返します。

**パスパラメータ**

| パラメータ | 型 | 説明 |
|-----------|-----|------|
| `discord_id` | string | Discord ユーザー ID |

**レスポンス** `200 OK`

```json
{
  "discord_id": "123456789012345678",
  "nickname": "PlayerOne",
  "avatar_url": "https://cdn.discordapp.com/avatars/...",
  "vrc_id": "usr_xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
  "x_id": "player_one",
  "bio_markdown": "こんにちは！VRChat を楽しんでいます。",
  "joined_at": "2025-01-15T09:30:00Z",
  "role": "member"
}
```

**エラーコード**

| コード | ステータス | 原因 |
|-------|-----------|------|
| `ERR-PROF-003` | 404 | メンバーが見つからない、またはプロフィールが非公開 |
| `ERR-RATELIMIT-001` | 429 | レート制限超過 |

---

## Events（イベント）

### イベント一覧

```
GET /api/v1/public/events
```

イベントのページネーション付きリストを返します。デフォルトでは公開済みイベントが対象です。

**クエリパラメータ**

| パラメータ | 型 | デフォルト | 説明 |
|-----------|-----|----------|------|
| `page` | integer | 1 | ページ番号（最小: 1） |
| `per_page` | integer | 20 | 1ページあたりのアイテム数（最小: 1、最大: 100） |
| `status` | string | `published` | イベントステータスでフィルタ |

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
  "total_count": 15,
  "total_pages": 1
}
```

**エラーコード**

| コード | ステータス | 原因 |
|-------|-----------|------|
| `ERR-VALIDATION` | 400 | 無効なページネーションまたはステータスパラメータ |
| `ERR-RATELIMIT-001` | 429 | レート制限超過 |

---

### イベント詳細

```
GET /api/v1/public/events/{event_id}
```

特定のイベントの詳細を返します。

**パスパラメータ**

| パラメータ | 型 | 説明 |
|-----------|-----|------|
| `event_id` | string (UUID) | イベント識別子 |

**レスポンス** `200 OK`

```json
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
```

**エラーコード**

| コード | ステータス | 原因 |
|-------|-----------|------|
| `ERR-EVT-001` | 404 | イベントが見つからない |
| `ERR-RATELIMIT-001` | 429 | レート制限超過 |

---

## Clubs（クラブ）

### クラブ一覧

```
GET /api/v1/public/clubs
```

クラブのページネーション付きリストを返します。

**クエリパラメータ**

| パラメータ | 型 | デフォルト | 説明 |
|-----------|-----|----------|------|
| `page` | integer | 1 | ページ番号（最小: 1） |
| `per_page` | integer | 20 | 1ページあたりのアイテム数（最小: 1、最大: 100） |

**レスポンス** `200 OK`

```json
{
  "items": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440001",
      "name": "写真部",
      "description_markdown": "VRChat の写真を撮るクラブです。",
      "cover_image_url": "https://example.com/images/photo-club.jpg",
      "member_count": 12,
      "created_at": "2025-06-01T12:00:00Z"
    }
  ],
  "total_count": 5,
  "total_pages": 1
}
```

**エラーコード**

| コード | ステータス | 原因 |
|-------|-----------|------|
| `ERR-VALIDATION` | 400 | 無効なページネーションパラメータ |
| `ERR-RATELIMIT-001` | 429 | レート制限超過 |

---

### クラブ詳細

```
GET /api/v1/public/clubs/{id}
```

特定のクラブの詳細を返します。

**パスパラメータ**

| パラメータ | 型 | 説明 |
|-----------|-----|------|
| `id` | string (UUID) | クラブ識別子 |

**レスポンス** `200 OK`

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440001",
  "name": "写真部",
  "description_markdown": "VRChat の写真を撮るクラブです。\n\n## 活動内容\n- 毎週のフォトウォーク\n- 月例コンテスト",
  "cover_image_url": "https://example.com/images/photo-club.jpg",
  "member_count": 12,
  "created_at": "2025-06-01T12:00:00Z",
  "updated_at": "2026-02-10T08:30:00Z"
}
```

**エラーコード**

| コード | ステータス | 原因 |
|-------|-----------|------|
| `ERR-CLUB-001` | 404 | クラブが見つからない |
| `ERR-RATELIMIT-001` | 429 | レート制限超過 |

---

### クラブギャラリー一覧

```
GET /api/v1/public/clubs/{id}/gallery
```

クラブの承認済みギャラリー画像のページネーション付きリストを返します。

**パスパラメータ**

| パラメータ | 型 | 説明 |
|-----------|-----|------|
| `id` | string (UUID) | クラブ識別子 |

**クエリパラメータ**

| パラメータ | 型 | デフォルト | 説明 |
|-----------|-----|----------|------|
| `page` | integer | 1 | ページ番号（最小: 1） |
| `per_page` | integer | 20 | 1ページあたりのアイテム数（最小: 1、最大: 100） |

**レスポンス** `200 OK`

```json
{
  "items": [
    {
      "image_id": "550e8400-e29b-41d4-a716-446655440010",
      "image_url": "https://example.com/gallery/photo1.jpg",
      "caption": "バーチャルの山々に沈む夕日",
      "uploaded_at": "2026-01-15T14:30:00Z"
    }
  ],
  "total_count": 24,
  "total_pages": 2
}
```

**エラーコード**

| コード | ステータス | 原因 |
|-------|-----------|------|
| `ERR-CLUB-001` | 404 | クラブが見つからない |
| `ERR-VALIDATION` | 400 | 無効なページネーションパラメータ |
| `ERR-RATELIMIT-001` | 429 | レート制限超過 |

---

## 関連ドキュメント

- [API 概要](README.md) — ベース URL、ページネーション、認証方式
- [Internal API](internal.md) — 認証済みユーザー向けエンドポイント
- [エラーカタログ](../errors.md) — 全エラーコードのリファレンス
- [使用例](../../getting-started/examples.md) — curl による API 使用例
