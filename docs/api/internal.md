# Internal API リファレンス

Internal API は、フロントエンドがユーザーのブラウザ・セッションを利用してサーバーとやり取りを行う層（BFF: Backend for Frontend）です。全エンドポイントで **Session Cookie による認証** が必要です。

## 概要と共通仕様

- **ベースパス:** `/api/v1/internal`
- **認証:** `session_id` Cookie（HttpOnly, SameSite=Lax/Strict）
- **CSRF 対策:** `POST`, `PUT`, `DELETE` リクエストの場合、Origin ヘッダが `FRONTEND_ORIGIN` と一致しているかを検証します。
- **Rate Limit:** 120 回/分（ユーザーごと・未認証時はIPごと）
- **Cache-Control:** 全レスポンスで `private, no-store` が付与されます。
- **共通エラー**: 
  - `401 ERR-AUTH-003` (セッション無効、または Cookie なし)
  - `403 ERR-AUTH-004` (アカウント自体が管理者によって停止されている)
  - `403 ERR-CSRF-001` (Origin 検証に失敗した、不正なサイトからのPOST)

---

## エンドポイント一覧

- [GET /api/v1/internal/auth/me](#get-apiv1internalauthme) - 現在の認証ユーザー情報の取得
- [POST /api/v1/internal/auth/logout](#post-apiv1internalauthlogout) - ログアウト（セッション破棄）
- [GET /api/v1/internal/me/profile](#get-apiv1internalmeprofile) - 自身のプロフィール詳細の取得
- [PUT /api/v1/internal/me/profile](#put-apiv1internalmeprofile) - 自身のプロフィールの作成/更新
- [GET /api/v1/internal/events](#get-apiv1internalevents) - ログインユーザー向けイベント一覧（未実装の拡張フィールド枠あり）
- [POST /api/v1/internal/reports](#post-apiv1internalreports) - ユーザーまたはイベントの通報（モデレーション機能）
- [POST /api/v1/internal/admin/clubs](#post-apiv1internaladminclubs) - 部活動の新規作成（Staff以上）
- [POST /api/v1/internal/admin/clubs/{id}/gallery](#post-apiv1internaladminclubsidgallery) - ギャラリー画像のアップロード（Staff以上）
- [PATCH /api/v1/internal/admin/gallery/{image_id}/status](#patch-apiv1internaladmingalleryimage_idstatus) - ギャラリー画像ステータスの更新（Staff以上）
- [GET /api/v1/internal/admin/users](#get-apiv1internaladminusers) - 管理者用ユーザー一覧（Admin以上）
- [PATCH /api/v1/internal/admin/users/{id}/role](#patch-apiv1internaladminusersidrole) - ユーザーロールの変更（Admin以上）

---

## 1. 認証・ユーザー情報

### GET /api/v1/internal/auth/me
現在アクセスしているユーザー自身の基本情報と、プロフィールの簡易情報を取得します。フロントエンド起動時の状態復元に使用します。

- **URL:** `/api/v1/internal/auth/me`
- **Method:** `GET`

#### Success Response (200 OK)

プロフィールをまだ作成していない場合は、`profile` が `null` になります。

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "discord_id": "123456789012345678",
  "discord_username": "example_user",
  "avatar_url": "https://cdn.discordapp.com/avatars/.../abc.png",
  "role": "member",
  "profile": {
    "vrc_id": "usr_abc123",
    "x_id": "@example",
    "is_public": true
  }
}
```

---

### POST /api/v1/internal/auth/logout
現在のセッションをサーバー上から削除し、ブラウザの Cookie もクリアします。

- **URL:** `/api/v1/internal/auth/logout`
- **Method:** `POST`
- **Headers:** `Origin: <FRONTEND_ORIGIN>` (CSRF 保護)

#### Request Body
（なし）

#### Success Response (204 No Content)
ボディなし。
レスポンスヘッダとして以下が返却され、ブラウザから Cookie が削除されます。
`Set-Cookie: session_id=; HttpOnly; SameSite=Strict; Max-Age=0; Path=/...`

---

## 2. プロフィール管理 (My Profile)

### GET /api/v1/internal/me/profile
自分自身のプロフィール詳細を取得します。Public 版のプロフィール取得とは異なり、非公開 (`is_public: false`) な状態でも自分のデータなら閲覧可能です。

- **URL:** `/api/v1/internal/me/profile`
- **Method:** `GET`

#### Success Response (200 OK)
```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "vrc_id": "usr_abc123",
  "x_id": "@example",
  "bio_markdown": "# 自己紹介\nよろしくおねがいします！",
  "bio_html": "<h1>自己紹介</h1>\n<p>よろしくおねがいします！</p>",
  "is_public": true,
  "updated_at": "2026-03-01T12:00:00Z"
}
```

#### Error Responses
| Status | Code | Meaning |
|---|---|---|
| 404 | `ERR-PROF-004` | プロフィールがまだ作成されていない |

---

### PUT /api/v1/internal/me/profile
自分自身のプロフィールを更新します。まだ作成していない場合は新規作成され、存在する場合は上書き更新されます。

- **URL:** `/api/v1/internal/me/profile`
- **Method:** `PUT`
- **Headers:** 
  - `Content-Type: application/json`
  - `Origin: <FRONTEND_ORIGIN>` (CSRF 保護)

#### Request Body (`UpdateProfileRequest`)

| Field | Type | Required? | Validation Rules |
|---|---|---|---|
| `vrc_id` | string \| null | No | 最大 100 文字 |
| `x_id` | string \| null | No | `@` または英数字・アンダースコア。最大 15+1 文字 `^[a-zA-Z0-9_]{1,15}$` もしくは `^@[a-zA-Z0-9_]{1,15}$` |
| `bio_markdown` | string | **Yes** | 最大 5000 文字。**XSS保護のため、`<script>` やイベントハンドラ(`onclick`等)が含まれるとエラーになります** |
| `is_public` | boolean | **Yes** | — |

**リクエスト例:**
```json
{
  "vrc_id": "usr_abc123",
  "x_id": "@example",
  "bio_markdown": "新しい自己紹介です",
  "is_public": true
}
```

#### Success Response (200 OK)
作成/更新されたプロフィールの完全なオブジェクトが返されます（[GET /me/profile](#get-apiv1internalmeprofile) と同一構造）。

#### Error Responses
| Status | Code | Meaning |
|---|---|---|
| 400 | `ERR-PROF-001` | バリデーションエラー（bioが長すぎ、x_idの形式不正 等）。詳細はレスポンスの `details` を参照 |
| 400 | `ERR-PROF-002` | `bio_markdown` の中に悪意あるコード（XSSの試み等）が検出された |

---

## 3. イベント関連

### GET /api/v1/internal/events
ログイン中のユーザー向けにイベント情報を取得します。Publicの `GET /public/events` と機能・引数は同一ですが、将来的に自分が参加予定かどうかのステータスなどのフラグが付与される拡張用エンドポイントです。

- **URL:** `/api/v1/internal/events`
- **Method:** `GET`
- **Parameters:** `page`, `per_page`, `status` (Public版と共通)

#### Success Response (200 OK)
Public 版のイベントオブジェクトに対して、`extended_info` フィールドが必ず追加されます。（現在は `{"sync_status": "synced"}` レベルのプレースホルダのみ）

```json
{
  "items": [
    {
      "id": "660-...",
      "title": "VRC 週末交流会",
      ...
      "extended_info": {
        "sync_status": "synced"
      }
    }
  ],
  ...
}
```

---

## 4. モデレーションと通報

### POST /api/v1/internal/reports
不適切なプロフィールやイベントを通報します。対象が存在するか検証され、その後コミュニティ運営向けの Webhook (Discord) が発火します。

- **URL:** `/api/v1/internal/reports`
- **Method:** `POST`
- **Headers:** 
  - `Content-Type: application/json`
  - `Origin: <FRONTEND_ORIGIN>` (CSRF 保護)

#### Request Body (`CreateReportRequest`)

| Field | Type | Required? | Validation Rules |
|---|---|---|---|
| `target_type` | string | **Yes** | `"profile"` または `"event"` であること |
| `target_id` | UUID | **Yes** | 存在する対象のリソース ID |
| `reason` | string | **Yes** | 10文字以上、1000文字以内 |

**リクエスト例:**
```json
{
  "target_type": "profile",
  "target_id": "550e8400-e29b-41d4-a716-446655440000",
  "reason": "プロフィール画像に露骨な性的コンテンツが含まれています。"
}
```

#### Success Response (201 Created)
非同期で運営に通知が飛ぶため、フロントエンドへのレスポンスは「受理した」ことのみを返します。

```json
{
  "id": "770e8400-e29b-41d4-a716-446655440000",
  "status": "accepted"
}
```

#### Error Responses
| Status | Code | Meaning |
|---|---|---|
| 400 | `ERR-MOD-003` | 通報理由の文字数が 10〜1000 文字の範囲外 |
| 404 | `ERR-MOD-001` | 指定された `target_type` および `target_id` のリソースが見つからない |
| 409 | `ERR-MOD-002` | このユーザーは、すでに対象への通報を行っている（重複不可） |

---

## 5. 部活動管理 (Admin: Clubs)

以下のエンドポイントは `staff` / `admin` / `super_admin` ロールを持つユーザーのみアクセス可能です。

### POST /api/v1/internal/admin/clubs
新しい部活動を作成します。

- **URL:** `/api/v1/internal/admin/clubs`
- **Method:** `POST`
- **Required Role:** `staff` 以上
- **Headers:** 
  - `Content-Type: application/json`
  - `Origin: <FRONTEND_ORIGIN>` (CSRF 保護)

#### Request Body (`CreateClubRequest`)

| Field | Type | Required? | Validation Rules |
|---|---|---|---|
| `name` | string | **Yes** | 部活動名 |
| `description` | string | **Yes** | 部活動の説明 |
| `cover_image_url` | string \| null | No | カバー画像の URL |

**リクエスト例:**
```json
{
  "name": "VRChat ワールド探検部",
  "description": "毎週新しいワールドを探検する部活です。",
  "cover_image_url": "https://example.com/images/club-cover.jpg"
}
```

#### Success Response (201 Created)

作成された部活動のオブジェクトが返されます。

```json
{
  "id": "880e8400-e29b-41d4-a716-446655440000",
  "name": "VRChat ワールド探検部",
  "description": "毎週新しいワールドを探検する部活です。",
  "cover_image_url": "https://example.com/images/club-cover.jpg",
  "created_at": "2026-03-01T10:00:00Z",
  "updated_at": "2026-03-01T10:00:00Z"
}
```

#### Error Responses
| Status | Code | Meaning |
|---|---|---|
| 403 | `ERR-PERM-001` | 権限不足（`staff` 以上が必要） |

---

## 6. ギャラリー管理 (Admin: Gallery)

ギャラリー画像の投稿・ステータス管理用エンドポイントです。`staff` 以上のロールが必要です。

### POST /api/v1/internal/admin/clubs/{id}/gallery
指定した部活動にギャラリー画像を追加します。画像は初期ステータス `pending` で作成されます。

- **URL:** `/api/v1/internal/admin/clubs/:id/gallery`
- **Method:** `POST`
- **Required Role:** `staff` 以上
- **Headers:** 
  - `Content-Type: application/json`
  - `Origin: <FRONTEND_ORIGIN>` (CSRF 保護)

#### Path Parameters

| Field | Type | Required? | Description |
|---|---|---|---|
| `id` | UUID | Yes | 対象部活動の UUID |

#### Request Body (`UploadGalleryImageRequest`)

| Field | Type | Required? | Validation Rules |
|---|---|---|---|
| `image_url` | string | **Yes** | 画像の URL（S3/R2 等のストレージ URL を想定） |

**リクエスト例:**
```json
{
  "image_url": "https://example.com/images/gallery-001.jpg"
}
```

#### Success Response (201 Created)

```json
{
  "id": "990e8400-e29b-41d4-a716-446655440000",
  "club_id": "880e8400-e29b-41d4-a716-446655440000",
  "uploaded_by": "550e8400-e29b-41d4-a716-446655440000",
  "image_url": "https://example.com/images/gallery-001.jpg",
  "status": "pending",
  "created_at": "2026-03-10T15:00:00Z"
}
```

#### Error Responses
| Status | Code | Meaning |
|---|---|---|
| 403 | `ERR-PERM-001` | 権限不足（`staff` 以上が必要） |
| 404 | `ERR-NOT-FOUND` | 対象の部活動が存在しない |

---

### PATCH /api/v1/internal/admin/gallery/{image_id}/status
ギャラリー画像のステータスを変更します（承認・却下・保留）。

- **URL:** `/api/v1/internal/admin/gallery/:image_id/status`
- **Method:** `PATCH`
- **Required Role:** `staff` 以上
- **Headers:** 
  - `Content-Type: application/json`
  - `Origin: <FRONTEND_ORIGIN>` (CSRF 保護)

#### Path Parameters

| Field | Type | Required? | Description |
|---|---|---|---|
| `image_id` | UUID | Yes | 対象ギャラリー画像の UUID |

#### Request Body (`UpdateGalleryStatusRequest`)

| Field | Type | Required? | Validation Rules |
|---|---|---|---|
| `status` | string | **Yes** | `"pending"`, `"approved"`, `"rejected"` のいずれか |

**リクエスト例:**
```json
{
  "status": "approved"
}
```

#### Success Response (204 No Content)
ボディなし。

#### Error Responses
| Status | Code | Meaning |
|---|---|---|
| 400 | `ERR-GALLERY-003` | 無効なステータス値が指定された |
| 403 | `ERR-PERM-001` | 権限不足（`staff` 以上が必要） |
| 404 | `ERR-NOT-FOUND` | 対象のギャラリー画像が存在しない |

---

## 7. ユーザー権限管理 (Admin: Users)

ユーザー一覧の取得とロール変更を行うエンドポイントです。`admin` 以上のロールが必要です。

### GET /api/v1/internal/admin/users
管理者用のユーザー一覧を取得します。ロールやステータスによるフィルタリングが可能です。ページネーション対応。

- **URL:** `/api/v1/internal/admin/users`
- **Method:** `GET`
- **Required Role:** `admin` 以上

#### Query Parameters

| Field | Type | Required? | Default | Description |
|---|---|---|---|---|
| `page` | integer | No | `1` | 取得するページ番号 |
| `per_page` | integer | No | `20` | 1ページあたりの件数（最大 `100`） |
| `role` | string | No | — | `member`, `staff`, `admin`, `super_admin` のいずれかでフィルタ |
| `status` | string | No | — | `active`, `suspended` のいずれかでフィルタ |

#### Success Response (200 OK)

```json
{
  "items": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "discord_id": "123456789012345678",
      "discord_username": "example_user",
      "avatar_url": "https://cdn.discordapp.com/avatars/.../abc.png",
      "role": "member",
      "status": "active",
      "created_at": "2026-03-01T10:00:00Z",
      "updated_at": "2026-03-01T12:00:00Z"
    }
  ],
  "total_count": 150,
  "total_pages": 8
}
```

#### Error Responses
| Status | Code | Meaning |
|---|---|---|
| 400 | `ERR-VALIDATION` | `role` または `status` の値が無効な列挙値 |
| 403 | `ERR-PERM-002` | 権限不足（`admin` 以上が必要） |

---

### PATCH /api/v1/internal/admin/users/{id}/role
対象ユーザーのロールを変更します。

- **URL:** `/api/v1/internal/admin/users/:id/role`
- **Method:** `PATCH`
- **Required Role:** `admin` 以上
- **Headers:** 
  - `Content-Type: application/json`
  - `Origin: <FRONTEND_ORIGIN>` (CSRF 保護)

#### Path Parameters

| Field | Type | Required? | Description |
|---|---|---|---|
| `id` | UUID | Yes | 対象ユーザーの UUID |

#### Request Body (`UpdateUserRoleRequest`)

| Field | Type | Required? | Validation Rules |
|---|---|---|---|
| `new_role` | string | **Yes** | `"member"`, `"staff"`, `"admin"`, `"super_admin"` のいずれか |

**リクエスト例:**
```json
{
  "new_role": "staff"
}
```

#### Success Response (204 No Content)
ボディなし。

#### セキュリティ制約

以下の権限チェックがサーバーサイドで実施されます：

| ルール | エラーコード | 説明 |
|---|---|---|
| `new_role` が `admin` の場合 | `ERR-ROLE-001` | 実行者が `super_admin` でなければ `403` |
| `new_role` が `super_admin` の場合 | `ERR-ROLE-002` | 実行者が `super_admin` でなければ `403` |
| 対象ユーザーが `super_admin` の場合 | `ERR-ROLE-003` | 実行者が `super_admin` でなければ変更不可 `403` |
| 実行者が `admin` 未満の場合 | `ERR-ROLE-004` | ロール変更権限なし `403` |

#### Error Responses
| Status | Code | Meaning |
|---|---|---|
| 403 | `ERR-ROLE-001` | `admin` ロールの付与は `super_admin` のみ可能 |
| 403 | `ERR-ROLE-002` | `super_admin` ロールの付与は `super_admin` のみ可能 |
| 403 | `ERR-ROLE-003` | `super_admin` ユーザーのロール変更は `super_admin` のみ可能 |
| 403 | `ERR-ROLE-004` | ロール変更には `admin` 以上の権限が必要 |
| 404 | `ERR-USER-001` | 対象ユーザーが存在しない |
