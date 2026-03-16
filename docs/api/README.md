# API ドキュメント

このディレクトリには、VRC Backend の各種 API リファレンスが含まれています。
API は用途とアクセス権限に応じて 4 つのレイヤーに分割されています。

## API リファレンス一覧

1. **[Public API](./public.md)** (`/api/v1/public/*`)
   - 認証不要の公開データ API（メンバー一覧、イベント一覧、部活動一覧、ギャラリーなど）。
   - フロントエンドから未ログインのユーザーでもアクセス可能。

2. **[Internal API](./internal.md)** (`/api/v1/internal/*`)
   - ログイン済みユーザー向け API（自身のプロフィール編集、通報など）。
   - 管理者向け API（部活動管理、ギャラリー管理、ユーザー権限管理）。
   - フロントエンドからの利用を想定（Session Cookie 認証、CSRF 保護あり）。

3. **[System API](./system.md)** (`/api/v1/system/*`)
   - 外部システム・Bot 連携用 API（イベント同期、ユーザー離脱処理など）。
   - GAS や Discord Bot からの呼び出しを想定（Bearer Token 認証）。

4. **[Auth API](./auth.md)** (`/api/v1/auth/*`)
   - Discord OAuth2 ログインフロー処理用 API。
   - フロントエンドからのリダイレクト先として使用。

---

## 共通仕様

### ベース URL

ローカル開発環境: `http://localhost:8080/api/v1`  
本番環境: `https://<domain>/api/v1`

| レイヤー | ベースパス | 認証手段 | レート制限 |
|---|---|---|---|
| Public | `/public` | なし | 60 req / 分 (IP毎) |
| Internal | `/internal` | Session Cookie | 120 req / 分 (User/IP毎) |
| System | `/system` | Bearer Token | 30 req / 分 (固定) |
| Auth | `/auth` | なし | 10 req / 分 (IP毎) |

### API のレスポンス形式

全ての API は JSON 形式でデータを返します（HTTP ステータス 204 を除く）。

成功時の例：
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "success"
}
```

### ページネーション（一覧系 API）

一覧を取得する API（`/public/members`, `/public/events` 等）では、以下のクエリパラメータが共通で利用可能です。

| パラメータ | 型 | デフォルト | 範囲/最大値 | 説明 |
|---|---|---|---|---|
| `page` | Integer | `1` | 1〜 | 取得するページ番号 |
| `per_page` | Integer | `20` | 1〜100 | 1ページあたりの取得件数 |

**ページネーション メタデータ（レスポンスヘッダに付与）**

| ヘッダ名 | 例 | 説明 |
|---|---|---|
| `X-Total-Count` | `42` | 検索条件に合致する全アイテム数 |
| `X-Total-Pages` | `3` | 全ページ数（`Total-Count / per_page` の切り上げ） |

レスポンスボディには `items`, `total_count`, `total_pages` が含まれます：
```json
{
  "items": [...],
  "total_count": 42,
  "total_pages": 3
}
```

### エラーハンドリング

API がエラーを返す場合、以下の共通 JSON フォーマットを使用します。

```json
{
  "error": "ERR-XXXX-NNN",
  "message": "人間が読めるエラーメッセージの部分。フロントエンドで 그대로表示することも想定。",
  "details": null
}
```

**バリデーションエラー時の例（`ERR-PROF-001` 等）：**
`details` オブジェクトに、問題があったフィールドとその詳細が含まれます。
```json
{
  "error": "ERR-PROF-001",
  "message": "Validation failed",
  "details": {
    "bio_markdown": "Bio is too long (maximum 2000 characters)",
    "x_id": "Invalid X ID format"
  }
}
```

### 共通セキュリティヘッダ

全てのエラーおよび正常レスポンスにおいて、以下のセキュリティヘッダが付与されます。
- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY`
- `Strict-Transport-Security: max-age=63072000; includeSubDomains`
- `Content-Security-Policy: default-src 'none'; frame-ancestors 'none';`

### CORS について

フロントエンド（ブラウザ）から呼び出される想定の API (Public, Internal, Auth) には、CORS ミドルウェアが適用されます。
- **許可される Origin**: 環境変数 `FRONTEND_ORIGIN` および `ALLOWED_ORIGINS`（カンマ区切り）
- **許可されるメソッド**: `GET, POST, PUT, DELETE, OPTIONS`
- **許可されるヘッダ**: `Content-Type, Authorization, Cookie`
- **Credentials**: 許可（Cookie 等の送受信のため）
