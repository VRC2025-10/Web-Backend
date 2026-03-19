# System API リファレンス

> **ナビゲーション**: [ドキュメントホーム](../../README.md) > [リファレンス](../README.md) > [API](README.md) > System API

System API は外部サービス（Discord Bot、Google Apps Script など）がバックエンドとデータを同期するためのマシン間通信エンドポイントを提供します。すべてのリクエストには **Bearer トークン** 認証が必要です。レート制限: **グローバルで 30リクエスト/分**。

---

## 認証

すべての System API リクエストには `Authorization` ヘッダーが必要です：

```
Authorization: Bearer <SYSTEM_API_TOKEN>
```

トークンは `SYSTEM_API_TOKEN` 環境変数で設定され、64文字以上である必要があります。

**認証エラー**

| コード | ステータス | 原因 |
|-------|-----------|------|
| `ERR-SYNC-001` | 401 | Bearer トークンが欠落、無効、または不一致 |

---

## イベント

### イベントの Upsert

```
POST /api/v1/system/events
```

新しいイベントを作成するか、既存のイベントを更新します。イベントは `external_id` で照合されます — 指定された `external_id` を持つイベントが存在する場合は更新され、存在しない場合は新規作成されます。

**リクエストボディ**

```json
{
  "external_id": "evt-discord-001",
  "title": "VRChat 週次ミートアップ",
  "description_markdown": "毎週のコミュニティ集会に参加しましょう！\n\n## 詳細\n- Discord でのボイスチャット\n- ワールド内で集合",
  "status": "published",
  "host_discord_id": "123456789012345678",
  "start_time": "2026-03-20T20:00:00Z",
  "end_time": "2026-03-20T22:00:00Z",
  "location": "VRChat - コミュニティワールド",
  "tags": ["meetup", "weekly"]
}
```

**フィールドバリデーションルール**

| フィールド | 型 | 必須 | バリデーション |
|-----------|-----|------|--------------|
| `external_id` | string | はい | 1〜255文字。ソースシステムからの一意識別子 |
| `title` | string | はい | 1〜200文字。前後の空白はトリム |
| `description_markdown` | string \| null | いいえ | 最大5000文字 |
| `status` | string | はい | `draft`、`published`、`cancelled`、`archived` のいずれか |
| `host_discord_id` | string \| null | いいえ | 有効な Discord ユーザー ID 形式（snowflake） |
| `start_time` | string (ISO 8601) | はい | 有効な UTC 日時であること |
| `end_time` | string (ISO 8601) \| null | いいえ | 指定時は `start_time` より後であること |
| `location` | string \| null | いいえ | 最大500文字 |
| `tags` | string[] \| null | いいえ | 各タグ: 1〜50文字。イベントあたり最大20タグ |

**レスポンス** `200 OK`（更新時）/ `201 Created`（新規作成時）

```json
{
  "event_id": "550e8400-e29b-41d4-a716-446655440000",
  "external_id": "evt-discord-001",
  "title": "VRChat 週次ミートアップ",
  "description_markdown": "毎週のコミュニティ集会に参加しましょう！\n\n## 詳細\n- Discord でのボイスチャット\n- ワールド内で集合",
  "status": "published",
  "host_discord_id": "123456789012345678",
  "start_time": "2026-03-20T20:00:00Z",
  "end_time": "2026-03-20T22:00:00Z",
  "location": "VRChat - コミュニティワールド",
  "tags": ["meetup", "weekly"],
  "created_at": "2026-03-15T10:00:00Z",
  "updated_at": "2026-03-19T08:30:00Z"
}
```

**エラーコード**

| コード | ステータス | 原因 |
|-------|-----------|------|
| `ERR-SYNC-001` | 401 | 無効な System API トークン |
| `ERR-SYNC-002` | 400 | バリデーション失敗（`error.details` に詳細あり） |
| `ERR-VALIDATION` | 400 | 不正なリクエストボディ |
| `ERR-RATELIMIT-001` | 429 | レート制限超過 |

**エラーレスポンス例**

```json
{
  "error": {
    "code": "ERR-SYNC-002",
    "message": "System API バリデーションエラー",
    "details": {
      "title": "タイトルは必須です",
      "end_time": "終了時刻は開始時刻より後である必要があります"
    }
  }
}
```

---

## ユーザー同期

### メンバー離脱の処理

```
POST /api/v1/system/sync/users/leave
```

Discord ギルドからのメンバー離脱を処理します。これは**アトミック操作**で、以下を実行します：

1. ユーザーを非アクティブとしてマーク
2. 公開プロフィールを非表示化
3. 離脱理由とタイムスタンプを記録

ユーザーがシステム内に存在しない場合、リクエストは no-op として扱われ `204` を返します。

**リクエストボディ**

```json
{
  "discord_id": "123456789012345678",
  "reason": "left_guild"
}
```

**フィールドバリデーションルール**

| フィールド | 型 | 必須 | バリデーション |
|-----------|-----|------|--------------|
| `discord_id` | string | はい | 有効な Discord ユーザー ID 形式（snowflake） |
| `reason` | string \| null | いいえ | 最大500文字。`left_guild`、`kicked`、`banned`、または自由テキスト |

**レスポンス** `200 OK`（ユーザーが見つかり処理された場合）

```json
{
  "discord_id": "123456789012345678",
  "status": "inactive",
  "processed_at": "2026-03-19T12:00:00Z"
}
```

**レスポンス** `204 No Content`（ユーザーが見つからない場合 — no-op）

レスポンスボディはありません。

**エラーコード**

| コード | ステータス | 原因 |
|-------|-----------|------|
| `ERR-SYNC-001` | 401 | 無効な System API トークン |
| `ERR-SYNC-002` | 400 | バリデーション失敗 |
| `ERR-RATELIMIT-001` | 429 | レート制限超過 |

---

## 関連ドキュメント

- [API 概要](README.md) — ベース URL、認証方式、レート制限
- [設定リファレンス](../configuration.md) — `SYSTEM_API_TOKEN` の設定
- [エラーカタログ](../errors.md) — 全エラーコードのリファレンス
