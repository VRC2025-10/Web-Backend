# System API リファレンス

System API は、外部システム（Google Apps Script や Discord ボット等）から利用される**サーバー・ツー・サーバー (Machine-to-Machine)** 通信用のエンドポイントです。人間のユーザー（フロントエンド）からは利用されません。

## 概要と共通仕様

- **ベースパス:** `/api/v1/system`
- **認証:** 全エンドポイントで `Authorization: Bearer <token>` を利用します。
  - `<token>` には、バックエンドの環境変数 `SYSTEM_API_TOKEN` と完全に一致する文字列を使用します。
- **レート制限:** 30 リクエスト/分（単一のトークン・システム単位）
- **共通エラー**:
  - `401 ERR-SYNC-001` (ヘッダが欠落しているか、無効なトークンが指定された)
  - `400 ERR-SYNC-002` (リクエストボディのバリデーションに失敗した)

---

## 認証の仕組みについて

サーバー起動時、環境変数 `SYSTEM_API_TOKEN`（最低 64 文字）がメモリに読み込まれます。
System API へのリクエストを受け取った際、ミドルウェアが以下のように検証を行います：

1. リクエストヘッダの `Authorization: Bearer XXXX...` からトークン文字列を抽出。
2. その文字列を SHA-256 にてハッシュ化。
3. サーバーが保持する正しいトークンも同様に SHA-256 にてハッシュ化。
4. 定数時間比較アルゴリズム (`subtle::ConstantTimeEq`) を使って2つのハッシュを比較。

これにより、タイミング攻撃（「先頭の数文字が合っていると応答が微妙に遅くなる」ことでトークンを1文字ずつ推測される攻撃）からシステムを保護します。

> **呼び出し元の設定例（cURL）:** 
```bash
curl -H "Authorization: Bearer YOUR_SUPER_SECRET_LONG_TOKEN_STRING" \
     "https://api.example.com/api/v1/system/events"
```

---

## エンドポイント一覧

- [POST /api/v1/system/events](#post-apiv1systemevents) - イベント情報の同期（Upsert）
- [POST /api/v1/system/sync/users/leave](#post-apiv1systemsyncusersleave) - メンバーの Discord サーバー離脱処理

---

## 1. イベントの同期 (Event Sync)

### POST /api/v1/system/events
外部（Google スプレッドシート等）で管理されているイベントの情報を VRC バックエンドのデータベースに同期（新規作成または更新）します。
`external_source_id` をキーにして一意性を判断し、データが存在しなければ INSERT（新規作成）、存在すれば UPDATE（更新）を行う冪等な API です。

- **URL:** `/api/v1/system/events`
- **Method:** `POST`

#### Request Body (`SyncEventRequest`)

| Field | Type | Required? | Validation Rules & Description |
|---|---|---|---|
| `external_source_id` | string | **Yes** | 外部システム側での一意なID（空文字不可）。`"gas_event_001"`等 |
| `title` | string | **Yes** | 1〜100 文字のイベントタイトル |
| `description_markdown` | string | **Yes** | 最大 5000 文字の Markdown 本文 |
| `host_discord_id` | string \| null | No | 17〜20桁の数字文字列（Discord ID）。登録済ユーザーと紐づけるために使用。|
| `host_name` | string | **Yes** | 1〜50 文字。紐づくユーザーがいない場合などの表示用ホスト名 |
| `start_time` | ISO 8601 | **Yes** | `2026-03-15T19:00:00Z` 形式 |
| `end_time` | ISO 8601 \| null | No | `start_time` より後の時刻であること |
| `location` | string \| null | No | 最大 200 文字。VRChatでの集合場所(インスタンスURL)やワールド名など |
| `tags` | string[] \| null | No | 既存のタグ名と一致する文字列の配列 `["Social", "Beginner"]` 等 |

**Request Example:**
```json
{
  "external_source_id": "row_line_42",
  "title": "VRC 交流会",
  "description_markdown": "イベントの詳細...",
  "host_discord_id": "123456789012345678",
  "host_name": "Event Organizer",
  "start_time": "2026-03-20T21:00:00Z",
  "tags": ["Social", "Meetup"]
}
```

#### Success Response
新しくイベントが作られた場合は `201 Created`、既存のイベントが更新された場合は `200 OK` が返ります。
なお、新規作成時のみ Discord 連携用 Webhook に自動的に通知が送信されます。

**201 Created (新規作成時)**
```json
{
  "event_id": "660e8400-e29b-41d4-a716-446655440000",
  "is_new": true
}
```

**200 OK (更新時)**
```json
{
  "event_id": "660e8400-e29b-41d4-a716-446655440000",
  "is_new": false
}
```

---

## 2. メンバー離脱の同期 (Member Leave Sync)

### POST /api/v1/system/sync/users/leave
対象のユーザーが、公式の Discord サーバーを離脱（退出・Kick・Ban）した際に、ボットなどから呼び出すための API です。
この API が呼ばれると、セキュリティ保持のために以下のアクションがアトミック（トランザクション）に実行されます。

1. 対象ユーザーのステータスを `Suspended` に変更。
2. 対象ユーザーのすべてのセッションを削除（以後、フロントエンドの API は呼び出せなくなります）。
3. 対象ユーザーのプロフィールを非公開（`is_public = false`）にし、Public API から隠蔽します。

- **URL:** `/api/v1/system/sync/users/leave`
- **Method:** `POST`

#### Request Body (`SyncUserLeaveRequest`)

| Field | Type | Required? | Validation Rules & Description |
|---|---|---|---|
| `discord_id` | string | **Yes** | 離脱したユーザーの Discord ID |
| `reason` | string \| null | No | 「Server Leave」などの理由（ログ記録目的に使用、DBには現状保持しません） |

**Request Example:**
```json
{
  "discord_id": "123456789012345678",
  "reason": "Kicked by admin"
}
```

#### Success Response 

ユーザーが DB 上に見つかり、サスペンド処理が実行された場合は `200 OK` を返します。

**200 OK (通常)**
```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "action": "suspended"
}
```

**200 OK (すでにサスペンド済みの場合)**
```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "action": "already_suspended"
}
```

**204 No Content (対象が存在しない)**
サーバーに同期する前にユーザーが離脱した、または過去に登録したことがない Discord ユーザーの場合、処理すべきことがないためエラーとはせず `204 No Content` を返して終了します。
