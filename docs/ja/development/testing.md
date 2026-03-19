# テストガイド

> **ナビゲーション**: [ドキュメントホーム](../README.md) > [開発](README.md) > テスト

VRC Web-Backend のテスト戦略、ツール、パターンについて説明します。

## テストピラミッド

```
         ┌──────────┐
         │  形式検証  │  ← Kani 証明（重要な不変条件）
         │           │
        ┌┴──────────┴┐
        │ プロパティ   │  ← proptest（入力空間の探索）
        │ テスト      │
       ┌┴────────────┴┐
       │  結合テスト   │  ← vrc-backend/tests/（API + DB）
       │              │
      ┌┴──────────────┴┐
      │  単体テスト     │  ← 各モジュール内 #[cfg(test)]
      └────────────────┘
```

## クイックコマンド

| コマンド | 実行内容 |
|---------|---------|
| `make test` | 全単体テスト + 結合テスト |
| `make test-verbose` | 標準出力/エラー出力付き |
| `cargo test -- --nocapture` | フィルタなし出力 |
| `cargo test <name>` | `<name>` にマッチするテスト実行 |
| `cargo test --lib` | 単体テストのみ |
| `cargo test --test '*'` | 結合テストのみ |
| `cargo kani` | 形式検証証明 |

## 単体テスト

単体テストはテスト対象のコードと同じファイル内の `#[cfg(test)]` モジュールに配置します:

```rust
// src/domain/entities/user.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        // ...
    }
}
```

### 命名規約

- テスト名はシナリオを記述: `test_<関数名>_<シナリオ>_<期待結果>`
- `assert_eq!`、`assert_ne!`、`assert!` マクロを使用
- テストは焦点を絞る — 1つの論理的な動作に1つのアサーション

### 単体テストの実行

```bash
cargo test --lib                      # lib クレートの単体テスト
cargo test --lib -p vrc-backend       # vrc-backend のみ
cargo test --lib -p vrc-macros        # vrc-macros のみ
```

## プロパティベーステスト

[proptest](https://docs.rs/proptest/) を使用して大量の入力空間を自動的に探索します。手作業で選んだテストケースの代わりに、数千のランダム入力を生成してプロパティが全てに対して成り立つことを検証します。

### proptest で検証するプロパティ

| プロパティ | 説明 |
|-----------|------|
| VRC ID フォーマット | 有効な VRC ID は `usr_[a-zA-Z0-9]+` パターンにマッチし、無効な ID は拒否される |
| X (Twitter) ID フォーマット | 有効なスクリーンネームが期待パターンにマッチ |
| ロール順序 | ロール階層が一貫している（`Admin > Moderator > Member > Guest`） |
| Markdown サニタイズ | レンダリング結果に `<script>`、`onclick` 等の XSS ベクターが含まれない |
| ページネーション | `PageRequest` が常に有効な SQL `LIMIT`/`OFFSET` 値を生成 |

### 例

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn vrc_id_regex_matches_valid_ids(
        suffix in "[a-zA-Z0-9]{10,20}"
    ) {
        let id = format!("usr_{suffix}");
        assert!(VRC_ID_REGEX.is_match(&id));
    }

    #[test]
    fn markdown_never_produces_xss(
        input in ".*"
    ) {
        let rendered = render_markdown(&input);
        assert!(!rendered.contains("<script"));
        assert!(!rendered.contains("onclick"));
    }
}
```

### プロパティテストの実行

```bash
# デフォルト（テストごとに256ケース）
cargo test -- proptest

# 拡張（テストごとに10,000ケース）
PROPTEST_CASES=10000 cargo test -- proptest
```

proptest で失敗が見つかると `proptest-regressions/` にリグレッションファイルが書き込まれます。これらはリポジトリにコミットされ、常に再テストされます。

## 結合テスト

結合テストは `vrc-backend/tests/` にあり、HTTP ハンドラ → データベースの完全なラウンドトリップをテストします。

### 構成

```
vrc-backend/tests/
├── common/          # 共有テストヘルパー
│   └── mod.rs       # テストアプリセットアップ、認証ヘルパー
├── health_test.rs   # ヘルスエンドポイントテスト
├── auth_test.rs     # 認証フローテスト
└── ...
```

### 結合テストの実行

結合テストには稼働中の PostgreSQL が必要です:

```bash
# データベースの起動
make db-up

# 結合テスト実行
cargo test --test '*'

# 特定のテストファイル実行
cargo test --test health_test
```

### 結合テストの書き方

```rust
use axum::http::StatusCode;
use axum_test::TestServer;

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let app = setup_test_app().await;
    let server = TestServer::new(app).unwrap();

    let response = server.get("/health").await;

    assert_eq!(response.status_code(), StatusCode::OK);
}
```

## 形式検証（Kani）

[Kani](https://model-checking.github.io/kani/) は Rust 用の形式検証ツールで、有界モデル検査を使用して正しさのプロパティを証明します。

### Kani で検証する内容

| 証明 | プロパティ |
|------|-----------|
| ロール認可 | 認可されたロールは必要なロールの上位集合であり、権限昇格は不可能 |
| リダイレクト URL 検証 | 同一オリジンまたは許可リスト内の URL のみが検証を通過、オープンリダイレクトなし |
| メンバー脱退ステートマシン | 状態遷移は有効であり、どの初期状態からも無効な状態に到達不可能 |

### Kani 証明の実行

```bash
# 全証明の実行
cargo kani

# 特定の証明の実行
cargo kani --harness proof_role_authorization
```

Kani はナイトリー CI パイプラインでも実行され、リグレッションを検出します。

### Kani 証明の例

```rust
#[cfg(kani)]
mod verification {
    use super::*;

    #[kani::proof]
    fn proof_role_authorization() {
        let required: Role = kani::any();
        let actual: Role = kani::any();

        if actual.has_permission(required) {
            assert!(actual.level() >= required.level());
        }
    }
}
```

## 関連ドキュメント

- [開発セットアップ](setup.md)
- [ビルド](build.md)
- [CI/CD](ci-cd.md)
- [アーキテクチャ概要](../architecture/README.md)
