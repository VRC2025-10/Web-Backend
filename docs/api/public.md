# Public API リファレンス

Public API は認証不要で、VRC バックエンドが管理する公開データにアクセスするためのエンドポイントです。主にフロントエンドの公開ページ（トップページ、メンバー一覧、公開イベント一覧）で使用されます。

## エンドポイント一覧

- [GET /api/v1/public/members](#get-apiv1publicmembers) - メンバー一覧の取得
- [GET /api/v1/public/members/{user_id}](#get-apiv1publicmembersuser_id) - メンバープロフィールの詳細取得
- [GET /api/v1/public/events](#get-apiv1publicevents) - イベント一覧の取得
- [GET /api/v1/public/events/{event_id}](#get-apiv1publiceventsevent_id) - イベント詳細の取得
- [GET /api/v1/public/clubs](#get-apiv1publicclubs) - 部活動一覧の取得
- [GET /api/v1/public/clubs/{id}](#get-apiv1publicclubsid) - 部活動詳細の取得
- [GET /api/v1/public/clubs/{id}/gallery](#get-apiv1publicclubsidgallery) - 部活動ギャラリー画像一覧の取得

---

## 1. メンバー (Members)

### GET /api/v1/public/members
公開設定になっているメンバー（ユーザー）のプロフィール一覧を取得します。

- **URL:** `/api/v1/public/members`
- **Method:** `GET`
- **Auth Required:** No
- **Rate Limit**: 60 requests / minute (IP basis)
- **Cache-Control**: `public, max-age=30, stale-while-revalidate=60`

#### Query Parameters

| Field | Type | Required? | Default | Description |
|---|---|---|---|---|
| `page` | integer | No | `1` | 取得するページ番号 |
| `per_page` | integer | No | `20` | 1ページあたりの件数（最大 `100`） |

#### Success Response (200 OK)

総件数・総ページ数はレスポンスヘッダ `X-Total-Count`, `X-Total-Pages` とボディに含まれます。
`bio_summary` は Markdown をプレーンテキスト化し、最大120文字に丸めたものです（120文字を超える場合は末尾に `...` が付与されます）。

```json
{
  "items": [
    {
      "user_id": "550e8400-e29b-41d4-a716-446655440000",
      "discord_username": "example_user",
      "avatar_url": "https://cdn.discordapp.com/avatars/123456789012345/abc.png",
      "vrc_id": "usr_abc123",
      "x_id": "@example",
      "bio_summary": "VRChatでよく遊んでいます。週末のイベントに参加することが多いです...",
      "updated_at": "2026-03-01T12:00:00Z"
    }
  ],
  "total_count": 42,
  "total_pages": 3
}
```

#### Error Responses

| Status | Code | Meaning |
|---|---|---|
| 429 | `ERR-RATELIMIT-001` | リクエストのレートリミット（60 回/分）を超過した |
| 500 | `ERR-INTERNAL` | サーバー内部エラー |

---

### GET /api/v1/public/members/{user_id}
指定した `user_id` の、公開設定になっているプロフィールの詳細を取得します。

- **URL:** `/api/v1/public/members/:user_id`
- **Method:** `GET`
- **Auth Required:** No
- **Rate Limit**: 60 requests / minute (IP basis)
- **Cache-Control**: `public, max-age=60`

#### Path Parameters

| Field | Type | Required? | Description |
|---|---|---|---|
| `user_id` | UUID | Yes | 対象ユーザーの UUID |

#### Success Response (200 OK)

`bio_markdown` はユーザーが入力した生のMarkdownです。
`bio_html` はサーバーサイドでマークダウンをパースし、ammonia によって `script` や `iframe` といった危険なタグを除外したサニタイズ済みの HTML 文字列です。フロントエンドでは `dangerouslySetInnerHTML` 等でそのまま描画可能です。

```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "discord_username": "example_user",
  "avatar_url": "https://cdn.discordapp.com/avatars/123456789012345/abc.png",
  "vrc_id": "usr_abc123",
  "x_id": "@example",
  "bio_markdown": "# 自己紹介\nこんにちは！**VRChat**でよく遊んでいます。\n\n## 好きなこと\n- ワールド巡り",
  "bio_html": "<h1>自己紹介</h1>\n<p>こんにちは！<strong>VRChat</strong>でよく遊んでいます。</p>\n<h2>好きなこと</h2>\n<ul>\n<li>ワールド巡り</li>\n</ul>",
  "updated_at": "2026-03-01T12:00:00Z"
}
```

#### Error Responses

| Status | Code | Meaning |
|---|---|---|
| 400 | (N/A) | `user_id` が有効な UUID フォーマットではない |
| 404 | `ERR-PROF-004` | プロフィールが存在しない、退会済み、または非公開(`is_public=false`)の設定になっている |

---

## 2. イベント (Events)

### GET /api/v1/public/events
イベント一覧を取得します。任意でステータスによる絞り込みが可能です。

- **URL:** `/api/v1/public/events`
- **Method:** `GET`
- **Auth Required:** No
- **Rate Limit**: 60 requests / minute (IP basis)
- **Cache-Control**: `public, max-age=30, stale-while-revalidate=60`

#### Query Parameters

| Field | Type | Required? | Default | Description |
|---|---|---|---|---|
| `page` | integer | No | `1` | 取得するページ番号 |
| `per_page` | integer | No | `20` | 1ページあたりの件数（最大 `100`） |
| `status` | string | No | — | `draft`, `published`, `cancelled`, `archived` のいずれか。指定なしの場合は全て。 |

**注記:** フロントエンド要件で「下書きのイベント等はトップに出したくない」場合は、`?status=published` を付与してリクエストしてください。

#### Success Response (200 OK)

Public API 経由の場合、内部情報である `extended_info` フィールドは含まれません。

```json
{
  "items": [
    {
      "id": "660e8400-e29b-41d4-a716-446655440000",
      "title": "VRC 週末交流会 #12",
      "description_markdown": "今週末の交流会です！誰でも歓迎！",
      "host_name": "イベント主催者",
      "host_user_id": "550e8400-e29b-41d4-a716-446655440000",
      "event_status": "published",
      "start_time": "2026-03-15T19:00:00Z",
      "end_time": "2026-03-15T21:00:00Z",
      "location": "VRChat - My Home World",
      "tags": [
        { "id": "uuid-tag-1", "name": "Social", "color": "#3B82F6" },
        { "id": "uuid-tag-2", "name": "Meetup", "color": "#10B981" }
      ],
      "created_at": "2026-03-01T10:00:00Z",
      "updated_at": "2026-03-02T10:00:00Z"
    }
  ],
  "total_count": 5,
  "total_pages": 1
}
```

#### Error Responses

| Status | Code | Meaning |
|---|---|---|
| 400 | `ERR-VALIDATION` | `status` の値が前述の列挙値以外 |

---

### GET /api/v1/public/events/{event_id}
特定のイベントの詳細情報を取得します。

- **URL:** `/api/v1/public/events/:event_id`
- **Method:** `GET`
- **Auth Required:** No
- **Rate Limit**: 60 requests / minute (IP basis)
- **Cache-Control**: `public, max-age=60`

#### Path Parameters

| Field | Type | Required? | Description |
|---|---|---|---|
| `event_id` | UUID | Yes | 対象イベントの UUID |

#### Success Response (200 OK)

Public API 経由の場合、内部情報である `extended_info` フィールドは含まれません。

```json
{
  "id": "660e8400-e29b-41d4-a716-446655440000",
  "title": "VRC 週末交流会 #12",
  "description_markdown": "今週末の交流会です！誰でも歓迎！",
  "host_name": "イベント主催者",
  "host_user_id": "550e8400-e29b-41d4-a716-446655440000",
  "event_status": "published",
  "start_time": "2026-03-15T19:00:00Z",
  "end_time": "2026-03-15T21:00:00Z",
  "location": "VRChat - My Home World",
  "tags": [
    { "id": "uuid-tag-1", "name": "Social", "color": "#3B82F6" }
  ],
  "created_at": "2026-03-01T10:00:00Z",
  "updated_at": "2026-03-02T10:00:00Z"
}
```

#### Error Responses

| Status | Code | Meaning |
|---|---|---|
| 404 | `ERR-NOT-FOUND` | 対象のイベントが存在しない |

---

## 3. 部活動 (Clubs)

### GET /api/v1/public/clubs
部活動の一覧を取得します。

- **URL:** `/api/v1/public/clubs`
- **Method:** `GET`
- **Auth Required:** No
- **Rate Limit**: 60 requests / minute (IP basis)
- **Cache-Control**: `public, max-age=30, stale-while-revalidate=60`

#### Success Response (200 OK)

```json
[
  {
    "id": "880e8400-e29b-41d4-a716-446655440000",
    "name": "VRChat ワールド探検部",
    "description": "毎週新しいワールドを探検する部活です。",
    "cover_image_url": "https://example.com/images/club-cover.jpg",
    "created_at": "2026-03-01T10:00:00Z",
    "updated_at": "2026-03-02T10:00:00Z"
  }
]
```

#### Error Responses

| Status | Code | Meaning |
|---|---|---|
| 429 | `ERR-RATELIMIT-001` | リクエストのレートリミット（60 回/分）を超過した |
| 500 | `ERR-INTERNAL` | サーバー内部エラー |

---

### GET /api/v1/public/clubs/{id}
指定した部活動の詳細情報を取得します。

- **URL:** `/api/v1/public/clubs/:id`
- **Method:** `GET`
- **Auth Required:** No
- **Rate Limit**: 60 requests / minute (IP basis)
- **Cache-Control**: `public, max-age=60`

#### Path Parameters

| Field | Type | Required? | Description |
|---|---|---|---|
| `id` | UUID | Yes | 対象部活動の UUID |

#### Success Response (200 OK)

```json
{
  "id": "880e8400-e29b-41d4-a716-446655440000",
  "name": "VRChat ワールド探検部",
  "description": "毎週新しいワールドを探検する部活です。",
  "cover_image_url": "https://example.com/images/club-cover.jpg",
  "created_at": "2026-03-01T10:00:00Z",
  "updated_at": "2026-03-02T10:00:00Z"
}
```

#### Error Responses

| Status | Code | Meaning |
|---|---|---|
| 404 | `ERR-NOT-FOUND` | 対象の部活動が存在しない |

---

### GET /api/v1/public/clubs/{id}/gallery
指定した部活動の承認済み（`approved`）ギャラリー画像の一覧を取得します。ページネーション対応。

- **URL:** `/api/v1/public/clubs/:id/gallery`
- **Method:** `GET`
- **Auth Required:** No
- **Rate Limit**: 60 requests / minute (IP basis)
- **Cache-Control**: `public, max-age=60`

#### Path Parameters

| Field | Type | Required? | Description |
|---|---|---|---|
| `id` | UUID | Yes | 対象部活動の UUID |

#### Query Parameters

| Field | Type | Required? | Default | Description |
|---|---|---|---|---|
| `page` | integer | No | `1` | 取得するページ番号 |
| `per_page` | integer | No | `20` | 1ページあたりの件数（最大 `100`） |

#### Success Response (200 OK)

```json
{
  "items": [
    {
      "id": "990e8400-e29b-41d4-a716-446655440000",
      "club_id": "880e8400-e29b-41d4-a716-446655440000",
      "uploaded_by": "550e8400-e29b-41d4-a716-446655440000",
      "image_url": "https://example.com/images/gallery-001.jpg",
      "status": "approved",
      "created_at": "2026-03-10T15:00:00Z"
    }
  ],
  "total_count": 12,
  "total_pages": 1
}
```

#### Error Responses

| Status | Code | Meaning |
|---|---|---|
| 404 | `ERR-NOT-FOUND` | 対象の部活動が存在しない |
