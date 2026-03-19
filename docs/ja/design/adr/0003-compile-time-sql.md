# ADR-0003: コンパイル時 SQL 検証

> **ナビゲーション**: [ドキュメントホーム](../../README.md) > [設計](../README.md) > [ADR](README.md) > ADR-0003

## ステータス

**承認済み**

## 日付

2025-01-20

## コンテキスト

SQL クエリの正しさは3つの次元で必要です:
1. **構文**: SQL が正しくパースされる
2. **スキーマ**: 参照テーブルとカラムが存在する
3. **型**: Rust 型が PostgreSQL カラム型にマッチする

従来のアプローチ（生 SQL 文字列、ORM）はこれらのチェックの1つ以上をランタイムに延期します。

## 決定

**SQLx のコンパイル時クエリ検証**とオフラインモードを使用します。

```mermaid
graph TD
    Dev["開発者がクエリを書く"]
    Compile["cargo build<br/>(PostgreSQL に接続)"]
    Verify["SQLx が検証:<br/>1. SQL 構文<br/>2. カラム存在<br/>3. 型マッピング"]

    Dev --> Compile
    Compile --> Verify

    Verify -->|"すべて正しい"| OK["ビルド成功 ✅"]
    Verify -->|"エラー発見"| Fail["ビルド失敗 ❌"]

    Prepare["cargo sqlx prepare"]
    JSON[".sqlx/ ディレクトリ"]
    CI["CI ビルド<br/>(SQLX_OFFLINE=true)"]

    Prepare --> JSON
    JSON --> CI
```

## 結果

### ポジティブ

- SQL エラーがビルド時に検出される
- スキーマ変更が即座にクエリ不整合を報告
- 型安全な結果マッピング

### ネガティブ

- 開発中に PostgreSQL の実行が必要
- クエリ変更時にオフラインメタデータの手動更新が必要

## 関連

- [ビルドシステム](../../development/build.md)
- [設計原則](../principles.md)
