# はじめに

> **ナビゲーション**: [ドキュメントホーム](../README.md) > はじめに

**VRC Backend** へようこそ — VRC ウェブプラットフォームを支える Rust/Axum REST API です。このガイドでは、ローカル開発環境のセットアップ、サーバーの起動、API の呼び出しまでを案内します。

## 対象読者

- VRC プラットフォームに貢献する**バックエンド開発者**
- VRC API と連携する**フロントエンド開発者**
- 本番環境にデプロイする**システム管理者**
- プロジェクトを探索する**コミュニティメンバー**

## VRC Backend とは？

VRC Backend は Rust と Axum で構築された高性能 REST API で、PostgreSQL をデータベースとして使用しています。メンバー、イベント、クラブ、ギャラリーなどの管理エンドポイントを提供し、Discord OAuth2 認証とロールベースの認可機能を備えています。

## ガイド一覧

| ガイド | 説明 |
|--------|------|
| [インストール](installation.md) | 前提条件、Docker セットアップ、手動セットアップ、本番デプロイを含む完全なインストールガイド |
| [クイックスタート](quickstart.md) | 5分で起動・実行 |
| [使用例](examples.md) | curl コマンドによる API 使用例集 |

## クイックリンク

- **ヘルスチェック**: `GET /health`
- **API ベースパス**: `/api/v1/{public,internal,system,auth}`
- **デフォルトポート**: `3000`

## 関連ドキュメント

- [API リファレンス](../reference/api/README.md)
- [設定リファレンス](../reference/configuration.md)
- [エラーカタログ](../reference/errors.md)
- [用語集](../reference/glossary.md)
