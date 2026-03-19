# データフロー

> **ナビゲーション**: [ドキュメントホーム](../README.md) > [アーキテクチャ](README.md) > データフロー

## 概要

このドキュメントは、VRC Backend における4つの主要なインタラクションのデータフローを記述します。各フローは Mermaid シーケンス図で図示され、認証、バリデーション、データベース操作、副作用を含む完全なリクエスト/レスポンスパスを示しています。

---

## 1. Discord OAuth2 ログイン

認証フローは Discord の OAuth2 認可コードグラントを使用します。バックエンドは認可コードをトークンに交換し、ユーザー情報を取得し、ギルドメンバーシップを検証してセッションを確立します。

```mermaid
sequenceDiagram
    participant B as ブラウザ
    participant F as フロントエンド SPA
    participant C as Caddy
    participant BK as Backend (Axum)
    participant D as Discord API
    participant DB as PostgreSQL

    B->>F: 「Discord でログイン」をクリック
    F->>B: Discord 認可 URL へリダイレクト<br/>(client_id, redirect_uri, scope, state)

    B->>D: GET /oauth2/authorize
    D->>B: Discord 同意画面
    B->>D: ユーザーが承認
    D->>B: コールバック URL へリダイレクト<br/>(code, state)

    B->>C: GET /api/v1/auth/callback?code=…&state=…
    C->>BK: リクエスト転送

    Note over BK: state パラメータを検証<br/>（CSRF 防御）

    BK->>D: POST /oauth2/token<br/>（コード交換）
    D-->>BK: access_token, refresh_token

    BK->>D: GET /users/@me<br/>(Bearer access_token)
    D-->>BK: Discord ユーザーオブジェクト<br/>(id, username, avatar)

    BK->>D: GET /users/@me/guilds<br/>(Bearer access_token)
    D-->>BK: ギルド一覧

    Note over BK: VRC Discord ギルドの<br/>メンバーであることを確認

    BK->>DB: SELECT * FROM users<br/>WHERE discord_id = $1
    DB-->>BK: ユーザー行（または null）

    alt 新規ユーザー
        BK->>DB: INSERT INTO users (discord_id, username, role, status)
        BK->>DB: INSERT INTO profiles (user_id, display_name)
    else 既存ユーザー
        BK->>DB: UPDATE users SET username = $1,<br/>updated_at = NOW()
    end

    Note over BK: セキュアなランダム<br/>セッショントークンを生成（32バイト）

    BK->>DB: INSERT INTO sessions<br/>(user_id, token_hash, expires_at)

    BK-->>C: 302 フロントエンドへリダイレクト<br/>Set-Cookie: session=[token], HttpOnly, Secure, SameSite=Lax
    C-->>B: レスポンス転送
    B->>F: セッション Cookie 付きでアプリをロード
```

### ポイント

- `state` パラメータは OAuth2 フロー中の CSRF 攻撃を防止する
- セッショントークンの SHA-256 ハッシュのみがデータベースに保存される
- セッション Cookie は `HttpOnly`、`Secure`、`SameSite=Lax`
- ギルドメンバーシップが必須 — 非メンバーは `403 Forbidden` を受け取る
- 新規ユーザーは `member` ロールとデフォルトプロフィールで自動プロビジョニングされる

---

## 2. プロフィール更新（Internal API）

認証済みユーザーがプロフィールを更新します。このフローには CSRF 検証、セッション認証、入力バリデーション、Markdown レンダリング、HTML サニタイズ、データベース永続化が含まれます。

```mermaid
sequenceDiagram
    participant B as ブラウザ
    participant C as Caddy
    participant BK as Backend (Axum)
    participant DB as PostgreSQL

    B->>C: PUT /api/v1/internal/profile<br/>Cookie: session=[token]<br/>X-CSRF-Token: [csrf_token]<br/>Body: { display_name, bio_markdown, is_public }
    C->>BK: リクエスト転送

    Note over BK: ミドルウェア: request_id,<br/>security_headers, metrics

    Note over BK: CSRF ミドルウェア:<br/>X-CSRF-Token が<br/>Cookie と一致することを検証

    Note over BK: AuthenticatedUser<Member>:<br/>セッション Cookie を抽出

    BK->>DB: SELECT u.*, s.expires_at<br/>FROM sessions s JOIN users u<br/>ON s.user_id = u.id<br/>WHERE s.token_hash = $1
    DB-->>BK: ユーザー + セッション

    Note over BK: セッションが期限切れでないことを確認<br/>ユーザーステータスが active であることを確認

    Note over BK: last_accessed_at を更新<br/>（スライディングウィンドウ）

    Note over BK: ValidatedJson<ProfileUpdate>:<br/>入力をデシリアライズ + バリデーション<br/>- display_name: 1-100 文字<br/>- bio_markdown: 最大 4000 文字

    Note over BK: bio_markdown → HTML レンダリング<br/>(pulldown-cmark)

    Note over BK: HTML サニタイズ<br/>(ammonia — スクリプト除去,<br/>安全なタグのみ許可)

    BK->>DB: INSERT INTO profiles<br/>(user_id, display_name,<br/>bio_markdown, bio_html, is_public)<br/>ON CONFLICT (user_id) DO UPDATE
    DB-->>BK: 更新されたプロフィール行

    BK-->>C: 200 OK<br/>{ profile data }
    C-->>B: レスポンス転送
```

### ポイント

- CSRF 防御はダブルサブミット Cookie パターンを使用する
- 入力バリデーションは `ValidatedJson` エクストラクター（`#[derive(Validate)]` 経由）で実行される
- Markdown はサーバーサイドでレンダリングされ、`ammonia` でサニタイズして XSS を防止する
- `bio_markdown`（ソース）と `bio_html`（レンダリング済み）の両方を保存し、読み取り時の再レンダリングを回避する
- `UPSERT` パターンにより、初回プロフィール作成と以降の更新の両方を処理する

---

## 3. Google Apps Script からのイベント同期

GAS が System API にイベントデータをプッシュします。バックエンドは Bearer トークンを検証し、ペイロードをバリデーションし、タグ付きでイベントを UPSERT し、新規イベントに対して Discord Webhook 通知を送信します。

```mermaid
sequenceDiagram
    participant GAS as Google Apps Script
    participant C as Caddy
    participant BK as Backend (Axum)
    participant DB as PostgreSQL
    participant WH as Discord Webhook

    GAS->>C: POST /api/v1/system/events<br/>Authorization: Bearer <system_token><br/>Body: { source_id, title, description,<br/>start_time, end_time, world_link, tags[] }
    C->>BK: リクエスト転送

    Note over BK: ミドルウェア: request_id,<br/>security_headers, metrics

    Note over BK: Bearer トークンが設定済み<br/>SYSTEM_API_TOKEN と<br/>一致することを検証

    Note over BK: ValidatedJson<EventSync>:<br/>デシリアライズ + バリデーション<br/>- title: 必須, 1-200 文字<br/>- start_time: 必須, 有効な ISO 8601<br/>- tags: 任意, 最大 10 件

    BK->>DB: SELECT * FROM events<br/>WHERE source_id = $1
    DB-->>BK: 既存イベント（または null）

    alt 新規イベント
        BK->>DB: BEGIN TRANSACTION
        BK->>DB: INSERT INTO events<br/>(source_id, title, description,<br/>start_time, end_time, world_link,<br/>status = 'published')
        DB-->>BK: 新規イベント行

        loop 各タグについて
            BK->>DB: INSERT INTO event_tags (name)<br/>ON CONFLICT (name) DO NOTHING
            BK->>DB: INSERT INTO event_tag_mappings<br/>(event_id, tag_id)
        end

        BK->>DB: COMMIT

        BK->>WH: POST Discord Webhook<br/>Embed: 新規イベント告知<br/>(タイトル, 時間, ワールドリンク)
        WH-->>BK: 204 No Content

    else 既存イベント（更新）
        BK->>DB: BEGIN TRANSACTION
        BK->>DB: UPDATE events SET<br/>title=$1, description=$2,<br/>start_time=$3, updated_at=NOW()<br/>WHERE source_id = $4

        BK->>DB: DELETE FROM event_tag_mappings<br/>WHERE event_id = $1
        loop 各タグについて
            BK->>DB: INSERT INTO event_tags (name)<br/>ON CONFLICT (name) DO NOTHING
            BK->>DB: INSERT INTO event_tag_mappings<br/>(event_id, tag_id)
        end

        BK->>DB: COMMIT
    end

    BK-->>C: 200 OK<br/>{ event data }
    C-->>GAS: レスポンス転送
```

### ポイント

- System API は静的 Bearer トークン（共有シークレット）を使用する — セッション管理なし
- `source_id` フィールドにより冪等な UPSERT が可能 — 繰り返しの同期で重複が作成されない
- タグ管理はタグ正規化に INSERT-or-IGNORE を、マッピングには全置換を使用する
- 同期内の全データベース操作はトランザクションで囲まれる
- Discord Webhook 通知は新規イベントのみに対して送信される（更新時は送信しない）
- Webhook が失敗してもイベントは作成される — Webhook 配信はベストエフォート

---

## 4. メンバー退出（Discord Bot → System API）

メンバーが Discord ギルドから退出または BAN された場合、Bot がバックエンドに通知します。バックエンドは単一トランザクション内で関連する全エンティティに退出処理をカスケードします。

```mermaid
sequenceDiagram
    participant BOT as Discord Bot
    participant C as Caddy
    participant BK as Backend (Axum)
    participant DB as PostgreSQL

    BOT->>C: POST /api/v1/system/members/leave<br/>Authorization: Bearer <system_token><br/>Body: { discord_id }
    C->>BK: リクエスト転送

    Note over BK: Bearer トークンを検証

    Note over BK: ValidatedJson<MemberLeave>:<br/>discord_id 形式をバリデーション

    BK->>DB: SELECT * FROM users<br/>WHERE discord_id = $1
    DB-->>BK: ユーザー行

    alt ユーザーが見つからない
        BK-->>C: 404 Not Found
    else ユーザーが見つかった
        BK->>DB: BEGIN TRANSACTION

        Note over BK: 1. ユーザーアカウントを停止
        BK->>DB: UPDATE users<br/>SET status = 'suspended',<br/>updated_at = NOW()<br/>WHERE id = $1

        Note over BK: 2. 全セッションを無効化
        BK->>DB: DELETE FROM sessions<br/>WHERE user_id = $1

        Note over BK: 3. プロフィールを非公開に設定
        BK->>DB: UPDATE profiles<br/>SET is_public = false,<br/>updated_at = NOW()<br/>WHERE user_id = $1

        Note over BK: 4. クラブメンバーシップを削除
        BK->>DB: DELETE FROM club_members<br/>WHERE user_id = $1

        BK->>DB: COMMIT

        BK-->>C: 200 OK<br/>{ status: "suspended",<br/>actions: [...] }
    end

    C-->>BOT: レスポンス転送
```

### ポイント

- 全カスケード変更は原子性のため単一データベーストランザクション内で実行される
- ユーザーは**停止**されるが削除はされない — 将来の再有効化のためにデータは保持される
- 全アクティブセッションは即座に無効化され、全デバイスで強制ログアウトされる
- 退出メンバーの情報を非表示にするためプロフィールは非公開に設定される
- クラブメンバーシップは削除されるが、所有クラブは残る（必要に応じて別途無効化）
- ギャラリー画像とレポートは**削除されない** — 監査目的で保持される
- ユーザーがギルドに戻り再ログインした場合、管理者がステータスを再有効化できる

---

## 関連ドキュメント

- [システムコンテキスト](system-context.md) — これらのフローに関与するアクターと外部システム
- [コンポーネント](components.md) — 各ステップを処理する内部コンポーネント
- [データモデル](data-model.md) — 各フローで操作されるデータベーステーブルと関係
- [ステート管理](state-management.md) — これらのフローによってトリガーされる状態遷移
- [モジュール依存関係](module-dependency.md) — リクエスト処理に関与するコードモジュール
