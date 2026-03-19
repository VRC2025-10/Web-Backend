# 開発ガイド

> **ナビゲーション**: [ドキュメントホーム](../README.md) > 開発

## 概要

VRC Web-Backend の開発に参加するためのガイドです。環境構築からテスト、CI/CD パイプラインまで、開発に必要な全情報をまとめています。

## クイックコマンド

```bash
make setup     # 開発環境のセットアップ
make run       # 開発サーバー起動
make test      # 全テスト実行
make lint      # clippy + fmt チェック
make check     # lint + test + build（プリコミット）
```

## ドキュメント

| ドキュメント | 説明 |
|------------|------|
| [セットアップ](setup.md) | 開発環境の構築手順 |
| [ビルド](build.md) | ビルドシステムと最適化オプション |
| [テスト](testing.md) | テスト戦略と実行方法 |
| [CI/CD](ci-cd.md) | パイプラインの構成と実行 |
| [プロジェクト構成](project-structure.md) | ディレクトリツリーとコード構成 |

## 開発ワークフロー

```mermaid
graph LR
    A["ブランチ作成"] --> B["コード変更"]
    B --> C["make check"]
    C --> D["コミット"]
    D --> E["プッシュ"]
    E --> F["PR 作成"]
    F --> G["CI 通過"]
    G --> H["レビュー"]
    H --> I["マージ"]
```

1. `feature/` ブランチを作成
2. コードを変更
3. `make check` でローカル検証（lint + test + build）
4. Conventional Commits 形式でコミット
5. プッシュして PR を作成
6. CI が自動実行（clippy, fmt, test, build, audit）
7. レビュー後にマージ

## 関連ドキュメント

- [コントリビューティングガイド](../../../CONTRIBUTING.md)
- [設計原則](../design/principles.md)
- [アーキテクチャ概要](../architecture/README.md)
