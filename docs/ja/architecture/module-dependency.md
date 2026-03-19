# モジュール依存関係グラフ

> **ナビゲーション**: [ドキュメントホーム](../README.md) > [アーキテクチャ](README.md) > モジュール依存関係

## 概要

このドキュメントは、VRC Backend ワークスペース内の全 Rust モジュール間の依存関係を図示します。プロジェクトはヘキサゴナルアーキテクチャに従っており、ドメインコアは外向きの依存関係を一切持たず、全アダプターはドメインコアに向かって内向きに依存します。

## ワークスペースクレート

プロジェクトは2つのクレートで構成される Cargo ワークスペースです。

| クレート | パス | 種別 | 説明 |
|---------|------|------|------|
| **`vrc-backend`** | `vrc-backend/` | `[[bin]]` + `[lib]` | メインアプリケーション — 全ドメインロジック、アダプター、ランタイム |
| **`vrc-macros`** | `vrc-macros/` | `proc-macro` | カスタム derive マクロ（`#[handler]`、`#[derive(Validate)]`、`#[derive(ErrorCode)]`） |

## モジュール依存関係グラフ

```mermaid
graph LR
    subgraph "vrc-macros (proc-macro クレート)"
        MACROS["vrc_macros<br/>#[handler]<br/>#[derive(Validate)]<br/>#[derive(ErrorCode)]"]
    end

    subgraph "vrc-backend"
        MAIN["main.rs<br/>(エントリポイント)"]
        LIB["lib.rs<br/>(再エクスポート)"]

        subgraph Config["設定"]
            CFG["config"]
        end

        subgraph "ドメインコア"
            ENT["domain::entities<br/>User, Profile, Event,<br/>Club, Gallery, Report, Session"]
            VO["domain::value_objects<br/>PageRequest, PageResponse"]
            PORTS["domain::ports<br/>7 リポジトリトレイト<br/>3 サービストレイト"]
        end

        subgraph "インバウンドアダプター"
            ROUTES["adapters::inbound::routes<br/>public, internal, system,<br/>auth, admin, health, metrics"]
            MW["adapters::inbound::middleware<br/>csrf, rate_limit,<br/>security_headers,<br/>metrics, request_id"]
            EXT["adapters::inbound::extractors<br/>ValidatedJson, ValidatedQuery,<br/>AuthenticatedUser"]
        end

        subgraph "アウトバウンドアダプター"
            PG["adapters::outbound::postgres<br/>PgUserRepo, PgProfileRepo,<br/>PgEventRepo, PgClubRepo,<br/>PgGalleryRepo, PgReportRepo,<br/>PgSessionRepo"]
            DC["adapters::outbound::discord<br/>DiscordClient"]
            MD["adapters::outbound::markdown<br/>MarkdownService"]
        end

        subgraph 認証
            ROLES["auth::roles<br/>Member, Staff, Admin,<br/>SuperAdmin (ファントム型)"]
            AUTH_EXT["auth::extractor<br/>AuthenticatedUser<R>"]
        end

        subgraph エラー
            API_ERR["errors::api<br/>ApiError, ErrorResponse"]
            DOM_ERR["errors::domain<br/>DomainError"]
            INFRA_ERR["errors::infrastructure<br/>InfraError"]
        end

        subgraph バックグラウンド
            SCHED["background::scheduler<br/>SessionCleanup,<br/>EventArchival"]
        end
    end

    %% コンパイル時依存
    MACROS -.->|"proc-macro<br/>(コンパイル時)"| ENT
    MACROS -.->|"proc-macro<br/>(コンパイル時)"| API_ERR
    MACROS -.->|"proc-macro<br/>(コンパイル時)"| ROUTES
    MACROS -.->|"proc-macro<br/>(コンパイル時)"| EXT

    %% main.rs の依存関係
    MAIN -->|"起動"| LIB
    MAIN -->|"読み込み"| CFG
    MAIN -->|"ルーター構築"| ROUTES
    MAIN -->|"適用"| MW
    MAIN -->|"作成"| PG
    MAIN -->|"作成"| DC
    MAIN -->|"スポーン"| SCHED

    %% lib.rs
    LIB -->|"再エクスポート"| ENT
    LIB -->|"再エクスポート"| PORTS
    LIB -->|"再エクスポート"| API_ERR

    %% インバウンド → ドメイン
    ROUTES -->|"エンティティ使用"| ENT
    ROUTES -->|"ポート使用"| PORTS
    ROUTES -->|"使用"| VO
    EXT -->|"バリデーション"| ENT
    EXT -->|"使用"| ROLES
    AUTH_EXT -->|"呼び出し"| PORTS

    %% ルート依存関係
    ROUTES -->|"抽出"| EXT
    ROUTES -->|"認証チェック"| AUTH_EXT
    ROUTES -->|"返却"| API_ERR

    %% アウトバウンド → ドメイン
    PG -->|"実装"| PORTS
    PG -->|"使用"| ENT
    DC -->|"実装"| PORTS
    MD -->|"実装"| PORTS

    %% エラーフロー
    PG -->|"ラップ"| INFRA_ERR
    DC -->|"ラップ"| INFRA_ERR
    INFRA_ERR -->|"変換"| DOM_ERR
    DOM_ERR -->|"変換"| API_ERR

    %% 認証
    ROLES -->|"階層定義"| ENT
    AUTH_EXT -->|"抽出"| ROLES

    %% バックグラウンド
    SCHED -->|"呼び出し"| PORTS

    %% 設定
    CFG -->|"設定"| MAIN

    %% スタイリング
    style MACROS fill:#a6d,stroke:#333,stroke-width:2px,color:#fff
    style ENT fill:#c84,stroke:#333,stroke-width:2px,color:#fff
    style VO fill:#c84,stroke:#333,stroke-width:2px,color:#fff
    style PORTS fill:#c84,stroke:#333,stroke-width:2px,color:#fff
    style ROUTES fill:#2a6,stroke:#333,stroke-width:2px,color:#fff
    style MW fill:#2a6,stroke:#333,stroke-width:2px,color:#fff
    style EXT fill:#2a6,stroke:#333,stroke-width:2px,color:#fff
    style PG fill:#48a,stroke:#333,stroke-width:2px,color:#fff
    style DC fill:#48a,stroke:#333,stroke-width:2px,color:#fff
    style MD fill:#48a,stroke:#333,stroke-width:2px,color:#fff
    style API_ERR fill:#c44,stroke:#333,stroke-width:2px,color:#fff
    style DOM_ERR fill:#c44,stroke:#333,stroke-width:2px,color:#fff
    style INFRA_ERR fill:#c44,stroke:#333,stroke-width:2px,color:#fff
```

## 依存関係ルール

ヘキサゴナルアーキテクチャは厳密な依存関係ルールを強制します。

### 許可される依存関係

| 依存元 | 依存先 | 理由 |
|-------|-------|------|
| `main.rs` | すべて | コンポジションルート — 全コンポーネントを結合する |
| インバウンドアダプター | ドメインコア | ルートはエンティティ、ポート、値オブジェクトを使用する |
| アウトバウンドアダプター | ドメインコア | リポジトリはエンティティを使用してポートトレイトを実装する |
| バックグラウンド | ドメインコア（ポート） | スケジューラーはリポジトリメソッドを呼び出す |
| 認証 | ドメインコア（エンティティ） | ロールは `user_role` enum を参照する |
| エラー | （自己完結） | エラー型はレイヤー間で変換される |

### 禁止される依存関係

| 依存元 | 依存先 | 理由 |
|-------|-------|------|
| ドメインコア | いかなるアダプター | ドメインはインフラストラクチャに依存してはならない |
| ドメインコア | 認証/エラー | ドメインエラーはドメイン内で定義される（ドメインエラーは例外 — errors 内に存在するが概念的にはドメインのもの） |
| アウトバウンドアダプター | インバウンドアダプター | アダプター同士は依存しない |
| インバウンドアダプター | アウトバウンドアダプター | ルートはポートトレイトに依存し、具象実装には依存しない |

## `vrc-macros` クレート

`vrc-macros` クレートはコンパイル時にコードを生成するプロシージャマクロを提供します。

| マクロ | 対象 | 生成コード |
|-------|------|----------|
| `#[handler]` | ルートハンドラー関数 | エラーハンドリングのボイラープレートで関数をラップし、`AppState` をインジェクトする |
| `#[derive(Validate)]` | リクエスト DTO | フィールド属性（`#[validate(length(min=1, max=100))]`）からバリデーションロジックを実装する |
| `#[derive(ErrorCode)]` | エラー enum | HTTP ステータスコードとエラーコード文字列付きの `Into<ApiError>` を実装する |

`vrc-macros` は proc-macro クレートであるため、**コンパイル時のみ**の依存関係です。ランタイムには存在せず、出力はコンパイル時に消費クレートにインライン化されます。

## 主要モジュールの説明

| モジュール | ファイル | 責務 |
|----------|---------|------|
| `main.rs` | `src/main.rs` | アプリケーションエントリポイント。設定読み込み、コネクションプール作成、Axum ルーター構築、バックグラウンドタスクスポーン、サーバー起動。 |
| `lib.rs` | `src/lib.rs` | ライブラリルート。結合テストとベンチマーク用のパブリック API を再エクスポートする。 |
| `config` | `src/config/` | 環境変数ベースの設定。起動時に全設定をパースおよびバリデーションする。 |
| `domain::entities` | `src/domain/entities/` | 不変条件を持つコアビジネスエンティティ。フレームワーク依存なし。 |
| `domain::value_objects` | `src/domain/value_objects/` | ページネーション型およびその他の再利用可能な値型。 |
| `domain::ports` | `src/domain/ports/` | リポジトリとサービスの async トレイト定義。アダプターが実装する契約。 |
| `adapters::inbound::routes` | `src/adapters/inbound/routes/` | API レイヤー（public, internal, system, auth, admin）ごとに整理された Axum ルートハンドラー。 |
| `adapters::inbound::middleware` | `src/adapters/inbound/middleware/` | HTTP の横断的関心事のための Tower ミドルウェア。 |
| `adapters::inbound::extractors` | `src/adapters/inbound/extractors/` | カスタム Axum `FromRequest` / `FromRequestParts` 実装。 |
| `adapters::outbound::postgres` | `src/adapters/outbound/postgres/` | SQLx ベースの PostgreSQL リポジトリ実装。 |
| `adapters::outbound::discord` | `src/adapters/outbound/discord/` | OAuth2 およびギルド操作用の Discord REST API クライアント。 |
| `adapters::outbound::markdown` | `src/adapters/outbound/markdown/` | Markdown → サニタイズ済み HTML レンダリングパイプライン。 |
| `auth::roles` | `src/auth/roles/` | ロール階層型とファントム型定義。 |
| `auth::extractor` | `src/auth/` | `AuthenticatedUser<R>` エクストラクター実装。 |
| `errors` | `src/errors/` | 3レイヤーエラー階層: API、Domain、Infrastructure。 |
| `background::scheduler` | `src/background/` | Tokio ベースの定期タスクランナー。セッションクリーンアップとイベントアーカイブ。 |

---

## 関連ドキュメント

- [システムコンテキスト](system-context.md) — バックエンドが全体システムにどう位置づけられるか
- [コンポーネント](components.md) — コンポーネント図と責務の詳細
- [データモデル](data-model.md) — ER 図とスキーマ詳細
- [データフロー](data-flow.md) — 主要インタラクションのシーケンス図
- [ステート管理](state-management.md) — エンティティライフサイクルの状態マシン
