# ステート管理

> **ナビゲーション**: [ドキュメントホーム](../README.md) > [アーキテクチャ](README.md) > ステート管理

## 概要

このドキュメントは、VRC Backend における全ての状態マシンとエンティティライフサイクルを記述します。各ステートフルエンティティは PostgreSQL enum を使用して現在の状態を表現し、遷移はアプリケーション層で強制されます。無効な遷移はドメインエラーとなります。

---

## 1. ユーザーステータス

ユーザーには2つの状態があります。停止は管理者のアクションにより、またはメンバーが Discord ギルドから退出した場合に自動的に発生します。再有効化には管理者の明示的な介入が必要です。

```mermaid
stateDiagram-v2
    [*] --> Active : ユーザー作成<br/>（OAuth2 初回ログイン）

    Active --> Suspended : 管理者がユーザーを停止<br/>(POST /admin/users/:id/suspend)
    Active --> Suspended : メンバーがギルドを退出<br/>(POST /system/members/leave)

    Suspended --> Active : 管理者がユーザーを再有効化<br/>(POST /admin/users/:id/reactivate)

    note right of Active
        ユーザーが可能なこと:
        - ログインして全機能を使用
        - Internal API へのアクセス
        - アクティブセッションの維持
    end note

    note right of Suspended
        ユーザーが不可能なこと:
        - ログイン（OAuth2 ブロック）
        - 認証済み API へのアクセス
        全セッションが破棄される
        プロフィールが非公開に設定される
        クラブメンバーシップが削除される
    end note
```

### 遷移テーブル

| 遷移元 | 遷移先 | トリガー | アクター | 副作用 |
|-------|-------|---------|--------|-------|
| — | `active` | 初回 OAuth2 ログイン | システム | ユーザー作成 + デフォルトプロフィール |
| `active` | `suspended` | 管理者アクション | Admin / Super Admin | 全セッション削除、プロフィール非公開化 |
| `active` | `suspended` | メンバー退出 | Discord Bot（System API） | 全セッション削除、プロフィール非公開化、クラブメンバーシップ削除 |
| `suspended` | `active` | 管理者による再有効化 | Admin / Super Admin | なし（ユーザーはセッション作成のために再ログインが必要） |

---

## 2. イベントステータス

イベントは分岐する終端状態を持つ線形ライフサイクルに従います。GAS 経由で作成されたイベントは即座に公開されます。ドラフトイベントは管理者 API 経由で手動作成できます。アーカイブは設定された閾値（デフォルト 30日、大規模イベントは 60日）を超えたイベントに対して自動実行されます。

```mermaid
stateDiagram-v2
    [*] --> Draft : イベントを手動作成<br/>(POST /admin/events)

    [*] --> Published : GAS からのイベント同期<br/>(POST /system/events)

    Draft --> Published : 管理者が公開<br/>(PATCH /admin/events/:id/publish)
    Draft --> Cancelled : 管理者がキャンセル<br/>(PATCH /admin/events/:id/cancel)

    Published --> Cancelled : 管理者がキャンセル<br/>(PATCH /admin/events/:id/cancel)
    Published --> Archived : バックグラウンドスケジューラー<br/>（30/60日後に自動）

    Cancelled --> [*]
    Archived --> [*]

    note right of Draft
        Public API では非表示。
        管理者のみ閲覧可能。
    end note

    note right of Published
        Public API で表示。
        イベントフィードに一覧表示。
        作成時に Discord Webhook 送信。
    end note

    note right of Cancelled
        終端状態。
        Public API から非表示。
        監査用に保持。
    end note

    note right of Archived
        終端状態。
        Public API のイベント一覧から非表示。
        直接リンクではアクセス可能な場合がある。
    end note
```

### 遷移テーブル

| 遷移元 | 遷移先 | トリガー | アクター | 副作用 |
|-------|-------|---------|--------|-------|
| — | `draft` | 手動作成 | Admin | なし |
| — | `published` | GAS 同期 | GAS（System API） | Discord Webhook 通知 |
| `draft` | `published` | 管理者が公開 | Admin | Discord Webhook 通知 |
| `draft` | `cancelled` | 管理者がキャンセル | Admin | なし |
| `published` | `cancelled` | 管理者がキャンセル | Admin | なし |
| `published` | `archived` | 経過日数の閾値超過 | バックグラウンドスケジューラー | なし |

### アーカイブルール

- イベントは `end_time`（`end_time` がない場合は `start_time`）が設定された閾値より古い場合にアーカイブ対象となる
- デフォルト閾値: `end_time` から **30日**
- バックグラウンドスケジューラーは **24時間ごと** にチェック
- アーカイブは不可逆 — アーカイブされたイベントは再公開できない

---

## 3. ギャラリー画像ステータス

ギャラリー画像は公開される前にスタッフのレビューが必要です。モデレーションワークフローにより、コミュニティにアップロードされた全コンテンツがガイドラインを満たすことを保証します。

```mermaid
stateDiagram-v2
    [*] --> Pending : ユーザーが画像をアップロード<br/>(POST /internal/gallery)

    Pending --> Approved : スタッフが承認<br/>(PATCH /admin/gallery/:id/approve)
    Pending --> Rejected : スタッフが却下<br/>(PATCH /admin/gallery/:id/reject)

    Approved --> [*]
    Rejected --> [*]

    note right of Pending
        Public API では非表示。
        アップローダーと Staff 以上が閲覧可能。
        スタッフレビューパネルにキューイング。
    end note

    note right of Approved
        Public API で表示。
        ユーザーのギャラリーに表示。
    end note

    note right of Rejected
        Public API では非表示。
        却下通知付きでアップローダーが閲覧可能。
        アップローダーが削除可能。
    end note
```

### 遷移テーブル

| 遷移元 | 遷移先 | トリガー | アクター | 副作用 |
|-------|-------|---------|--------|-------|
| — | `pending` | 画像アップロード | 認証済みユーザー | 画像保存、レビューキューに追加 |
| `pending` | `approved` | スタッフレビュー | Staff / Admin / Super Admin | `reviewer_id` と `reviewed_at` を設定 |
| `pending` | `rejected` | スタッフレビュー | Staff / Admin / Super Admin | `reviewer_id` と `reviewed_at` を設定 |

---

## 4. レポートステータス

レポートはシンプルなトリアージワークフローに従います。スタッフがレポートをレビューし、アクションを取る（`reviewed`）か、アクション不要と判断する（`dismissed`）かを決定します。

```mermaid
stateDiagram-v2
    [*] --> Pending : ユーザーがレポートを送信<br/>(POST /internal/reports)

    Pending --> Reviewed : スタッフがレビューして<br/>アクションを実施<br/>(PATCH /admin/reports/:id)
    Pending --> Dismissed : スタッフが棄却<br/>(PATCH /admin/reports/:id)

    Reviewed --> [*]
    Dismissed --> [*]

    note right of Pending
        スタッフレポートキューに表示。
        対象コンテンツにレビューフラグ。
    end note

    note right of Reviewed
        対象に対してアクションが実施された。
        reviewer_id と reviewer_note を記録。
    end note

    note right of Dismissed
        アクション不要。
        reviewer_id と reviewer_note を記録。
    end note
```

### 遷移テーブル

| 遷移元 | 遷移先 | トリガー | アクター | 副作用 |
|-------|-------|---------|--------|-------|
| — | `pending` | レポート送信 | 認証済みユーザー | スタッフレビューキューに追加 |
| `pending` | `reviewed` | スタッフのアクション | Staff / Admin / Super Admin | `reviewer_id`、`reviewer_note`、`resolved_at` を設定。対象に対するアクション実施（例: コンテンツ削除、ユーザー警告）。 |
| `pending` | `dismissed` | スタッフの棄却 | Staff / Admin / Super Admin | `reviewer_id`、`reviewer_note`、`resolved_at` を設定。対象に対するアクションなし。 |

---

## 5. セッションライフサイクル

セッションは OAuth2 ログイン時に作成され、明示的なログアウト、ユーザー停止、または期限切れセッションの自動クリーンアップにより破棄されます。

```mermaid
stateDiagram-v2
    [*] --> Created : OAuth2 コールバック成功<br/>（トークン生成、ハッシュ保存）

    Created --> Active : セッション Cookie での初回リクエスト

    Active --> Active : 後続リクエスト<br/>（last_accessed_at 更新）

    Active --> Expired : expires_at 超過<br/>（明示的遷移なし —<br/>アクセスまたはクリーンアップ時に検出）

    Active --> Destroyed : ユーザーがログアウト<br/>(DELETE /auth/logout)
    Active --> Destroyed : ユーザーが停止<br/>（全セッション一括削除）
    Active --> Destroyed : 管理者による強制ログアウト<br/>(DELETE /admin/users/:id/sessions)

    Expired --> Destroyed : バックグラウンドスケジューラー<br/>クリーンアップ（毎時）

    Destroyed --> [*]

    note right of Created
        DB にセッション行が存在。
        クライアントに Cookie を設定。
        リクエストでまだ検証されていない。
    end note

    note right of Active
        セッションは有効。
        各リクエストで last_accessed_at が
        更新される（スライディングウィンドウ）。
    end note

    note right of Expired
        expires_at を超過したセッション。
        次回アクセス試行時に拒否。
        バックグラウンドジョブでクリーンアップ。
    end note

    note right of Destroyed
        DB からセッション行を削除。
        クライアントの Cookie を無効化。
    end note
```

### セッションプロパティ

| プロパティ | 値 |
|----------|---|
| トークン生成 | 32 バイト、`rand::OsRng`、base64url エンコード |
| ストレージ | `sessions.token_hash` に SHA-256 ハッシュ |
| デフォルト TTL | 7日間 |
| スライディングウィンドウ | 認証済みリクエストごとに `last_accessed_at` を更新 |
| クリーンアップ間隔 | 1時間ごと（バックグラウンドスケジューラー） |
| ユーザーあたりの最大セッション数 | 無制限（マルチデバイスサポート） |

---

## 6. OAuth2 フロー状態

Discord OAuth2 認可コードフローは独自の一時的な状態マシンを持ちます。これらの状態はログインプロセス中にメモリ内に存在し、データベースには永続化されません。

```mermaid
stateDiagram-v2
    [*] --> Initiated : ユーザーが「Discord でログイン」をクリック<br/>（state パラメータ生成）

    Initiated --> CodeReceived : Discord がコールバックにリダイレクト<br/>（クエリパラメータに code + state）

    CodeReceived --> StateValidated : state パラメータが<br/>保存された値と一致（CSRF チェック）
    CodeReceived --> Failed : state パラメータ不一致<br/>（CSRF 攻撃の可能性）

    StateValidated --> TokenExchanged : POST /oauth2/token 成功<br/>（access_token 受信）
    StateValidated --> Failed : トークン交換失敗<br/>（無効なコードまたは Discord エラー）

    TokenExchanged --> UserFetched : GET /users/@me 成功<br/>（Discord ユーザー情報受信）
    TokenExchanged --> Failed : ユーザー取得失敗

    UserFetched --> GuildVerified : ユーザーが VRC ギルドのメンバー
    UserFetched --> Failed : ギルド未加入<br/>（403 Forbidden）

    GuildVerified --> SessionCreated : DB にユーザーを UPSERT<br/>セッション作成

    SessionCreated --> [*] : フロントエンドへリダイレクト<br/>セッション Cookie 付き

    Failed --> [*] : フロントエンドへリダイレクト<br/>error パラメータ付き

    note right of Failed
        全ての失敗状態はフロントエンドに
        失敗理由を記述する error
        クエリパラメータ付きで
        リダイレクトされる。
    end note
```

### エラーハンドリング

| 失敗ポイント | HTTP レスポンス | ユーザー向けエラー |
|------------|---------------|-----------------|
| state 不一致 | 302 → `/login?error=csrf` | 「ログインに失敗しました。もう一度お試しください。」 |
| トークン交換失敗 | 302 → `/login?error=discord` | 「Discord 認証に失敗しました。」 |
| ユーザー取得失敗 | 302 → `/login?error=discord` | 「Discord アカウントを取得できませんでした。」 |
| ギルド未加入 | 302 → `/login?error=not_member` | 「VRC Discord サーバーのメンバーである必要があります。」 |

---

## 関連ドキュメント

- [システムコンテキスト](system-context.md) — 状態遷移に関与するアクターと外部システム
- [コンポーネント](components.md) — 状態変更を処理するコンポーネント
- [データモデル](data-model.md) — 状態フィールドのスキーマと enum 定義
- [データフロー](data-flow.md) — 状態遷移をトリガーするリクエストフロー
- [モジュール依存関係](module-dependency.md) — 状態管理に関与するモジュール
