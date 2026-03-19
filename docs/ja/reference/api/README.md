# API 概要

> **ナビゲーション**: [ドキュメントホーム](../../README.md) > [リファレンス](../README.md) > API 概要

VRC Web-Backend REST API の全体概要です。ベース URL、レスポンス形式、認証方式、ページネーション、エラーハンドリング、レート制限、セキュリティヘッダーについて説明します。

---

## ベース URL

```
https://your-domain.example.com/api/v1
```

ローカル開発環境の場合：

```
http://localhost:3000/api/v1
```

インフラストラクチャエンドポイント（`/health`、`/metrics`）は `/api/v1` プレフィックスなしのルートで提供されます。

---

## レスポンス形式

特に注記がない限り、すべての API レスポンスは JSON（`Content-Type: application/json`）です。

### 成功レスポンス

エンドポイントはリソースをレスポンスボディに直接返します：

```json
{
  "id": "...",
  "field": "value"
}
```

### ページネーション付きレスポンス

リストエンドポイントは `PageResponse<T>` ラッパーを返します：

```json
{
  "items": [...],
  "total_count": 42,
  "total_pages": 3
}
```

レスポンスヘッダー：

| ヘッダー | 説明 |
|---------|------|
| `x-total-count` | 全ページの合計アイテム数 |
| `x-total-pages` | 合計ページ数 |

### エラーレスポンス

すべてのエラーは統一されたエンベロープ形式に従います：

```json
{
  "error": {
    "code": "ERR-XXX-NNN",
    "message": "日本語のエラーメッセージ",
    "details": {}
  }
}
```

`details` フィールドはオプションで、追加のコンテキスト（フィールドレベルのバリデーションエラーなど）を提供します。

---

## ページネーション

すべてのリストエンドポイントは以下のパラメータを受け付けます：

| パラメータ | 型 | デフォルト | 最小 | 最大 | 説明 |
|-----------|-----|----------|------|------|------|
| `page` | integer | 1 | 1 | — | ページ番号 |
| `per_page` | integer | 20 | 1 | 100 | 1ページあたりのアイテム数 |

---

## 認証方式

API はエンドポイントカテゴリに応じて3つの異なる認証メカニズムを使用します：

| 方式 | 使用箇所 | メカニズム |
|------|---------|-----------|
| **なし** | Public API、Auth API | 認証不要 |
| **セッション Cookie** | Internal API（Admin 含む） | Discord OAuth2 ログイン後に設定される `session_id` Cookie |
| **Bearer トークン** | System API、Metrics | `Authorization: Bearer <SYSTEM_API_TOKEN>` ヘッダー |

### セッション Cookie 認証

Discord OAuth2 ログイン成功後、サーバーは HTTP-only で Secure なセッション Cookie を設定します。Cookie は後続のリクエストに自動的に送信されます。CSRF 保護は `Origin` ヘッダーの検証により実施されます。

### Bearer トークン認証

システム間通信エンドポイントでは、`Authorization` ヘッダーに事前共有トークンが必要です：

```
Authorization: Bearer <token>
```

---

## セキュリティヘッダー

すべてのレスポンスに含まれるヘッダー：

| ヘッダー | 値 |
|---------|-----|
| `X-Content-Type-Options` | `nosniff` |
| `X-Frame-Options` | `DENY` |

### CORS ポリシー

- **許可オリジン**: `FRONTEND_ORIGIN` 環境変数の値
- **許可メソッド**: `GET`、`POST`、`PUT`、`PATCH`、`DELETE`、`OPTIONS`
- **許可ヘッダー**: `Content-Type`、`Authorization`
- **クレデンシャル**: `true`（Cross-Origin で Cookie を送信）

### CSRF 保護

Internal API エンドポイントは `Origin` ヘッダーを `FRONTEND_ORIGIN` と照合します。一致しない `Origin` のリクエストは `ERR-CSRF-001`（403）で拒否されます。

---

## レート制限

レート制限はティアごとに適用されます：

| ティア | スコープ | 制限 | 適用対象 |
|--------|---------|------|---------|
| Public | IP 単位 | 60リクエスト/分 | Public API |
| Auth | IP 単位 | 10リクエスト/分 | Auth API |
| Internal | ユーザー単位 | 120リクエスト/分 | Internal API（Admin 含む） |
| System | グローバル | 30リクエスト/分 | System API |

レート制限を超過すると、サーバーは `429 Too Many Requests` とエラーコード `ERR-RATELIMIT-001` を返します。

### レート制限ヘッダー

| ヘッダー | 説明 |
|---------|------|
| `X-RateLimit-Limit` | ウィンドウ内の最大リクエスト数 |
| `X-RateLimit-Remaining` | 現在のウィンドウ内の残りリクエスト数 |
| `X-RateLimit-Reset` | ウィンドウがリセットされる Unix タイムスタンプ |
| `Retry-After` | 次のリクエストが許可されるまでの秒数（429 時のみ） |

---

## キャッシュ

| ティア | Cache-Control |
|--------|--------------|
| Public API | `public, max-age=30` |
| Internal API | `private, no-cache` |
| System API | `no-store` |

---

## エンドポイント一覧

### インフラストラクチャ

| メソッド | パス | 認証 | 説明 |
|---------|------|------|------|
| GET | `/health` | なし | ヘルスチェック |
| GET | `/metrics` | Bearer | Prometheus メトリクス |

### Auth API

| メソッド | パス | 認証 | 説明 |
|---------|------|------|------|
| GET | `/api/v1/auth/discord/login` | なし | Discord OAuth2 の開始 |
| GET | `/api/v1/auth/discord/callback` | なし | OAuth2 コールバック |

### Public API

| メソッド | パス | 認証 | 説明 |
|---------|------|------|------|
| GET | `/api/v1/public/members` | なし | メンバー一覧 |
| GET | `/api/v1/public/members/{discord_id}` | なし | メンバー詳細 |
| GET | `/api/v1/public/events` | なし | イベント一覧 |
| GET | `/api/v1/public/events/{event_id}` | なし | イベント詳細 |
| GET | `/api/v1/public/clubs` | なし | クラブ一覧 |
| GET | `/api/v1/public/clubs/{id}` | なし | クラブ詳細 |
| GET | `/api/v1/public/clubs/{id}/gallery` | なし | クラブギャラリー一覧 |

### Internal API

| メソッド | パス | 認証 | 説明 |
|---------|------|------|------|
| GET | `/api/v1/internal/auth/me` | Session | 現在のユーザー情報 |
| POST | `/api/v1/internal/auth/logout` | Session | ログアウト |
| GET | `/api/v1/internal/me/profile` | Session | 自分のプロフィール取得 |
| PUT | `/api/v1/internal/me/profile` | Session | 自分のプロフィール更新 |
| GET | `/api/v1/internal/events` | Session | イベント一覧（内部） |
| POST | `/api/v1/internal/reports` | Session | レポート作成 |

### Admin API

| メソッド | パス | 認証 | 説明 |
|---------|------|------|------|
| GET | `/api/v1/internal/admin/users` | Session (Admin+) | ユーザー一覧 |
| PATCH | `/api/v1/internal/admin/users/{id}/role` | Session (Admin+) | ロール変更 |
| PATCH | `/api/v1/internal/admin/users/{id}/status` | Session (Admin+) | ステータス変更 |
| GET | `/api/v1/internal/admin/reports` | Session (Staff+) | レポート一覧 |
| PATCH | `/api/v1/internal/admin/reports/{id}` | Session (Staff+) | レポート解決 |
| POST | `/api/v1/internal/admin/clubs` | Session (Staff+) | クラブ作成 |
| POST | `/api/v1/internal/admin/clubs/{id}/gallery` | Session (Staff+) | ギャラリー画像追加 |
| PATCH | `/api/v1/internal/admin/gallery/{id}/status` | Session (Staff+) | ギャラリーステータス変更 |

### System API

| メソッド | パス | 認証 | 説明 |
|---------|------|------|------|
| POST | `/api/v1/system/events` | Bearer | イベント同期 |
| POST | `/api/v1/system/sync/users/leave` | Bearer | メンバー離脱処理 |

---

## 関連ドキュメント

- [Public API](public.md) — 公開エンドポイントの詳細仕様
- [Internal API](internal.md) — 内部エンドポイントの詳細仕様
- [Admin API](admin.md) — 管理エンドポイントの詳細仕様
- [System API](system.md) — システムエンドポイントの詳細仕様
- [Auth API](auth.md) — 認証フローの詳細仕様
- [エラーカタログ](../errors.md) — 全エラーコードのリファレンス
- [設定リファレンス](../configuration.md) — 環境変数の設定
