# Admin API リファレンス

> **ナビゲーション**: [ドキュメントホーム](../../README.md) > [リファレンス](../README.md) > [API](README.md) > Admin API

Admin API は `/api/v1/internal/admin` 配下の管理エンドポイントを提供します。すべてのリクエストには有効な**セッション Cookie** と適切な**ロールレベル**が必要です。レート制限: **ユーザーあたり 120リクエスト/分**（Internal API と共有）。CSRF 保護: `Origin` ヘッダーの検証。

---

## ロール要件

| ロール | レベル | アクセス可能な機能 |
|--------|--------|-------------------|
| `member` | 0 | —（管理アクセスなし） |
| `staff` | 1 | レポート、クラブ、ギャラリー |
| `admin` | 2 | Staff の全機能 + ユーザー管理 |
| `super_admin` | 3 | Admin の全機能（制限なし） |

---

## ユーザー管理

### ユーザー一覧

```
GET /api/v1/internal/admin/users
```

**必要ロール**: Admin+（レベル ≥ 2）

すべてのユーザーの管理詳細付きページネーションリストを返します。

**クエリパラメータ**

| パラメータ | 型 | デフォルト | 説明 |
|-----------|-----|----------|------|
| `page` | integer | 1 | ページ番号（最小: 1） |
| `per_page` | integer | 20 | 1ページあたりのアイテム数（最小: 1、最大: 100） |
| `status` | string | — | ユーザーステータスでフィルタ（例: `active`、`inactive`、`suspended`） |
| `role` | string | — | ロールでフィルタ（例: `member`、`staff`、`admin`、`super_admin`） |

**レスポンス** `200 OK`

```json
{
  "items": [
    {
      "user_id": "550e8400-e29b-41d4-a716-446655440000",
      "discord_id": "123456789012345678",
      "discord_username": "PlayerOne",
      "role": "member",
      "status": "active",
      "joined_at": "2025-01-15T09:30:00Z",
      "last_login_at": "2026-03-18T14:00:00Z"
    }
  ],
  "total_count": 150,
  "total_pages": 8
}
```

**エラーコード**

| コード | ステータス | 原因 |
|-------|-----------|------|
| `ERR-AUTH-003` | 401 | 無効または期限切れのセッション |
| `ERR-PERM-001` | 403 | ロール不足（Admin+ が必要） |
| `ERR-VALIDATION` | 400 | 無効なクエリパラメータ |

---

### ユーザーロールの変更

```
PATCH /api/v1/internal/admin/users/{id}/role
```

**必要ロール**: Admin+（レベル ≥ 2）

ユーザーのロールを変更します。以下のロール認可マトリックスに従います。

**パスパラメータ**

| パラメータ | 型 | 説明 |
|-----------|-----|------|
| `id` | string (UUID) | 対象ユーザー ID |

**リクエストボディ**

```json
{
  "new_role": "staff"
}
```

| フィールド | 型 | 必須 | バリデーション |
|-----------|-----|------|--------------|
| `new_role` | string | はい | `member`、`staff`、`admin`、`super_admin` のいずれか |

**ロール変更認可マトリックス**

| 操作者のロール | → member に変更可 | → staff に変更可 | → admin に変更可 | → super_admin に変更可 |
|---------------|:-----------------:|:----------------:|:----------------:|:---------------------:|
| `admin` | ✅ | ✅ | ❌ | ❌ |
| `super_admin` | ✅ | ✅ | ✅ | ❌ |

追加の制約：
- **自己降格の禁止** — Admin は自分自身のロールを変更できません（`ERR-PERM-002`）
- **非 super_admin による Admin 昇格の禁止** — `super_admin` のみが `admin` に昇格可能（`ERR-ROLE-001`）
- **super_admin への昇格の禁止** — API 経由で `super_admin` に昇格することはできません（`ERR-ROLE-002`）
- **super_admin の保護** — `super_admin` ロールは API 経由で降格できません（`ERR-ROLE-003`）
- **レベルチェック** — 操作者のロールレベルは対象の現在のレベルより高い必要があります（`ERR-ROLE-004`）

**レスポンス** `200 OK`

```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "previous_role": "member",
  "new_role": "staff",
  "changed_by": "550e8400-e29b-41d4-a716-446655440001",
  "changed_at": "2026-03-19T12:00:00Z"
}
```

**エラーコード**

| コード | ステータス | 原因 |
|-------|-----------|------|
| `ERR-AUTH-003` | 401 | 無効または期限切れのセッション |
| `ERR-PERM-001` | 403 | ロール不足 |
| `ERR-PERM-002` | 403 | 自分自身のロールは変更不可 |
| `ERR-ROLE-001` | 403 | 非 super_admin による Admin 昇格は不可 |
| `ERR-ROLE-002` | 403 | API 経由で super_admin への昇格は不可 |
| `ERR-ROLE-003` | 403 | super_admin ロールは保護されている |
| `ERR-ROLE-004` | 403 | 操作者のロールレベルが対象に対して不足 |
| `ERR-USER-001` | 404 | ユーザーが見つからない |
| `ERR-CSRF-001` | 403 | CSRF 検証に失敗 |

---

### ユーザーステータスの変更

```
PATCH /api/v1/internal/admin/users/{id}/status
```

**必要ロール**: Admin+（レベル ≥ 2）

ユーザーのステータスを変更します（例: 停止、再有効化）。

**パスパラメータ**

| パラメータ | 型 | 説明 |
|-----------|-----|------|
| `id` | string (UUID) | 対象ユーザー ID |

**リクエストボディ**

```json
{
  "new_status": "suspended"
}
```

| フィールド | 型 | 必須 | バリデーション |
|-----------|-----|------|--------------|
| `new_status` | string | はい | `active`、`inactive`、`suspended` のいずれか |

**レスポンス** `200 OK`

```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "previous_status": "active",
  "new_status": "suspended",
  "changed_by": "550e8400-e29b-41d4-a716-446655440001",
  "changed_at": "2026-03-19T12:00:00Z"
}
```

**エラーコード**

| コード | ステータス | 原因 |
|-------|-----------|------|
| `ERR-AUTH-003` | 401 | 無効または期限切れのセッション |
| `ERR-PERM-001` | 403 | ロール不足 |
| `ERR-USER-001` | 404 | ユーザーが見つからない |
| `ERR-CSRF-001` | 403 | CSRF 検証に失敗 |

---

## レポート

### レポート一覧

```
GET /api/v1/internal/admin/reports
```

**必要ロール**: Staff+（レベル ≥ 1）

すべてのレポートのページネーション付きリストを返します。

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
      "report_id": "550e8400-e29b-41d4-a716-446655440099",
      "reporter_id": "550e8400-e29b-41d4-a716-446655440010",
      "reporter_discord_username": "ReporterUser",
      "target_type": "user",
      "target_id": "550e8400-e29b-41d4-a716-446655440000",
      "reason": "イベント中の不適切な行為。",
      "status": "pending",
      "created_at": "2026-03-19T12:00:00Z",
      "resolved_at": null,
      "resolved_by": null
    }
  ],
  "total_count": 5,
  "total_pages": 1
}
```

**エラーコード**

| コード | ステータス | 原因 |
|-------|-----------|------|
| `ERR-AUTH-003` | 401 | 無効または期限切れのセッション |
| `ERR-PERM-001` | 403 | ロール不足（Staff+ が必要） |

---

### レポートの解決

```
PATCH /api/v1/internal/admin/reports/{id}
```

**必要ロール**: Staff+（レベル ≥ 1）

レポートのステータスを更新します（例: 解決、却下）。

**パスパラメータ**

| パラメータ | 型 | 説明 |
|-----------|-----|------|
| `id` | string (UUID) | レポート ID |

**リクエストボディ**

```json
{
  "new_status": "resolved"
}
```

| フィールド | 型 | 必須 | バリデーション |
|-----------|-----|------|--------------|
| `new_status` | string | はい | `resolved`、`dismissed` のいずれか |

**レスポンス** `200 OK`

```json
{
  "report_id": "550e8400-e29b-41d4-a716-446655440099",
  "previous_status": "pending",
  "new_status": "resolved",
  "resolved_by": "550e8400-e29b-41d4-a716-446655440001",
  "resolved_at": "2026-03-19T14:00:00Z"
}
```

**エラーコード**

| コード | ステータス | 原因 |
|-------|-----------|------|
| `ERR-AUTH-003` | 401 | 無効または期限切れのセッション |
| `ERR-PERM-001` | 403 | ロール不足 |
| `ERR-CSRF-001` | 403 | CSRF 検証に失敗 |

---

## クラブ

### クラブの作成

```
POST /api/v1/internal/admin/clubs
```

**必要ロール**: Staff+（レベル ≥ 1）

新しいクラブを作成します。

**リクエストボディ**

```json
{
  "name": "写真部",
  "description_markdown": "VRChat の写真を撮るクラブです。",
  "cover_image_url": "https://cdn.discordapp.com/attachments/.../photo-club.jpg"
}
```

| フィールド | 型 | 必須 | バリデーション |
|-----------|-----|------|--------------|
| `name` | string | はい | 1〜100文字。前後の空白はトリム |
| `description_markdown` | string | はい | 1〜5000文字 |
| `cover_image_url` | string \| null | いいえ | 有効な HTTPS URL であること。SSRF 防止が適用される（下記参照） |

**URL の SSRF 防止**

ユーザーが指定するすべての URL は検証されます：
- `https://` スキームであること
- ドメインが許可リストに含まれること（例: `cdn.discordapp.com`）
- プライベート/内部 IP 範囲の禁止（`10.x`、`172.16.x`、`192.168.x`、`127.x`、`::1`）
- バリデーション中のリダイレクトは追跡しない

**レスポンス** `201 Created`

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440001",
  "name": "写真部",
  "description_markdown": "VRChat の写真を撮るクラブです。",
  "cover_image_url": "https://cdn.discordapp.com/attachments/.../photo-club.jpg",
  "created_at": "2026-03-19T12:00:00Z"
}
```

**エラーコード**

| コード | ステータス | 原因 |
|-------|-----------|------|
| `ERR-AUTH-003` | 401 | 無効または期限切れのセッション |
| `ERR-PERM-001` | 403 | ロール不足 |
| `ERR-CSRF-001` | 403 | CSRF 検証に失敗 |
| `ERR-VALIDATION` | 400 | バリデーション失敗 |

---

## ギャラリー

### ギャラリー画像のアップロード

```
POST /api/v1/internal/admin/clubs/{id}/gallery
```

**必要ロール**: Staff+（レベル ≥ 1）

クラブのギャラリーに画像を追加します。

**パスパラメータ**

| パラメータ | 型 | 説明 |
|-----------|-----|------|
| `id` | string (UUID) | クラブ ID |

**リクエストボディ**

```json
{
  "image_url": "https://cdn.discordapp.com/attachments/.../gallery-photo.jpg",
  "caption": "バーチャルの山々に沈む夕日"
}
```

| フィールド | 型 | 必須 | バリデーション |
|-----------|-----|------|--------------|
| `image_url` | string | はい | 有効な HTTPS URL であること。SSRF 防止が適用される |
| `caption` | string \| null | いいえ | 最大500文字 |

**レスポンス** `201 Created`

```json
{
  "image_id": "550e8400-e29b-41d4-a716-446655440010",
  "club_id": "550e8400-e29b-41d4-a716-446655440001",
  "image_url": "https://cdn.discordapp.com/attachments/.../gallery-photo.jpg",
  "caption": "バーチャルの山々に沈む夕日",
  "status": "pending",
  "uploaded_at": "2026-03-19T12:00:00Z"
}
```

**エラーコード**

| コード | ステータス | 原因 |
|-------|-----------|------|
| `ERR-AUTH-003` | 401 | 無効または期限切れのセッション |
| `ERR-PERM-001` | 403 | ロール不足 |
| `ERR-CLUB-001` | 404 | クラブが見つからない |
| `ERR-CSRF-001` | 403 | CSRF 検証に失敗 |
| `ERR-VALIDATION` | 400 | バリデーション失敗 |

---

### ギャラリー画像ステータスの変更

```
PATCH /api/v1/internal/admin/gallery/{id}/status
```

**必要ロール**: Staff+（レベル ≥ 1）

ギャラリー画像のステータスを変更します（承認、却下、削除）。

**パスパラメータ**

| パラメータ | 型 | 説明 |
|-----------|-----|------|
| `id` | string (UUID) | ギャラリー画像 ID |

**リクエストボディ**

```json
{
  "new_status": "approved"
}
```

| フィールド | 型 | 必須 | バリデーション |
|-----------|-----|------|--------------|
| `new_status` | string | はい | `approved`、`rejected`、`removed` のいずれか |

**有効なステータス遷移**

```
pending → approved
pending → rejected
pending → removed
approved → removed
rejected → removed
```

**レスポンス** `200 OK`

```json
{
  "image_id": "550e8400-e29b-41d4-a716-446655440010",
  "previous_status": "pending",
  "new_status": "approved",
  "changed_by": "550e8400-e29b-41d4-a716-446655440001",
  "changed_at": "2026-03-19T14:00:00Z"
}
```

**エラーコード**

| コード | ステータス | 原因 |
|-------|-----------|------|
| `ERR-AUTH-003` | 401 | 無効または期限切れのセッション |
| `ERR-PERM-001` | 403 | ロール不足 |
| `ERR-GAL-001` | 404 | ギャラリー画像が見つからない |
| `ERR-GAL-002` | 400 | 無効なステータス遷移 |
| `ERR-CSRF-001` | 403 | CSRF 検証に失敗 |

---

## 関連ドキュメント

- [API 概要](README.md) — ベース URL、認証方式、レート制限
- [Internal API](internal.md) — 一般ユーザー向け内部エンドポイント
- [エラーカタログ](../errors.md) — 全エラーコードのリファレンス
- [用語集](../glossary.md) — ロールとドメイン用語
