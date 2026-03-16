# VRC Backend ドキュメント

## 概要

VRC コミュニティ向けの Web バックエンド。Rust / Axum で構築されたクリーンアーキテクチャベースの REST API。

## ドキュメント一覧

| ドキュメント | 内容 |
|---|---|
| [セットアップガイド](setup.md) | 環境変数、起動方法、認証トークンの設定 |
| [API リファレンス](api/README.md) | 全エンドポイントの仕様、リクエスト/レスポンス例 |

## アーキテクチャ

```
[ブラウザ] ──→ Public API   (認証不要、キャッシュあり: メンバー・イベント・部活動閲覧)
           ──→ Internal API (セッション認証、BFF: プロフィール編集・管理者機能)
           ──→ Auth API     (Discord OAuth2 ログイン)

[GAS/Bot]  ──→ System API   (Bearer トークン認証、M2M)
```

### API レイヤー

| レイヤー | パス | 認証 | 用途 |
|---|---|---|---|
| Public | `/api/v1/public/*` | なし | メンバー・イベント・部活動・ギャラリー閲覧 |
| Internal | `/api/v1/internal/*` | Session Cookie | ログイン済みユーザー向け操作・管理者機能 |
| System | `/api/v1/system/*` | Bearer Token | 外部システム連携 (GAS, Bot) |
| Auth | `/api/v1/auth/*` | なし | Discord OAuth2 ログインフロー |

### 権限モデル

| ロール | 説明 |
|---|---|
| `super_admin` | システムの完全な支配権。`admin` ロールの付与・剥奪が可能 |
| `admin` | `staff` ロールへの変更、システム全体のモデレーション |
| `staff` | コンテンツ管理（部活動作成、ギャラリー承認、イベント管理） |
| `member` | 一般ユーザー |

## 技術スタック

- **Runtime**: Rust + Tokio
- **Framework**: Axum 0.8
- **Database**: PostgreSQL + SQLx 0.8
- **認証**: Discord OAuth2 + セッション Cookie / Bearer Token
- **セキュリティ**: ammonia (HTMLサニタイズ), governor (レート制限), subtle (定数時間比較)
