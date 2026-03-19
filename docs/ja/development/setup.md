# 開発環境セットアップ

> **ナビゲーション**: [ドキュメントホーム](../README.md) > [開発](README.md) > セットアップ

## 前提条件

| ツール | バージョン | 用途 |
|--------|----------|------|
| Rust | 1.85+ (Edition 2024) | コンパイラ・ツールチェーン |
| Docker | 最新 | PostgreSQL コンテナ |
| Docker Compose | v2+ | コンテナオーケストレーション |
| Git | 最新 | バージョン管理 |

## 手順

### 1. Rust のインストール

```bash
# rustup でインストール（未インストールの場合）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 必須コンポーネントの追加
rustup component add rustfmt clippy

# バージョン確認
rustc --version  # 1.85.0 以上
```

### 2. リポジトリのクローン

```bash
git clone https://github.com/VRC2025-10/Web-Backend.git
cd Web-Backend
```

### 3. 環境変数の設定

```bash
cp .env.example .env
```

`.env` ファイルを編集して、以下の必須変数を設定してください：

```bash
# データベース
DATABASE_URL=postgres://vrc:vrc_dev_password@localhost:5432/vrc_backend

# Discord OAuth2（Discord Developer Portal で取得）
DISCORD_CLIENT_ID=your_client_id
DISCORD_CLIENT_SECRET=your_client_secret
DISCORD_GUILD_ID=your_guild_id
BACKEND_BASE_URL=http://localhost:3000
FRONTEND_ORIGIN=http://localhost:3001

# シークレット生成
SESSION_SECRET=$(openssl rand -hex 32)
SYSTEM_API_TOKEN=$(openssl rand -hex 64)
```

### 4. PostgreSQL の起動

```bash
docker compose up -d
```

正常に起動したことを確認：

```bash
docker compose ps
# postgres コンテナが "running (healthy)" であること
```

### 5. ビルドと起動

```bash
# ビルド（初回は依存関係のダウンロード・コンパイルに時間がかかります）
cargo build

# 起動
cargo run
```

### 6. 動作確認

```bash
curl http://localhost:3000/health
# {"status":"healthy","version":"0.1.0","database":"connected",...}
```

## make setup（一括セットアップ）

上記の手順を自動化するには：

```bash
make setup
```

このコマンドは以下を実行します：
1. Rust ツールチェーンの確認
2. rustfmt と clippy のインストール
3. `.env` ファイルの作成（存在しない場合）
4. PostgreSQL の起動
5. プロジェクトのビルド

## オプションツール

| ツール | インストール | 用途 |
|--------|------------|------|
| cargo-watch | `cargo install cargo-watch` | ファイル変更時の自動リロード |
| cargo-audit | `cargo install cargo-audit` | 依存関係の脆弱性チェック |
| kani-verifier | `cargo install --locked kani-verifier && cargo kani setup` | 形式検証 |
| sqlx-cli | `cargo install sqlx-cli --no-default-features --features postgres` | マイグレーション管理 |

### cargo-watch の使用

```bash
# ファイル変更で自動リビルド＆再起動
make watch
# または
cargo watch -x run
```

## IDE 設定

### VS Code

推奨拡張機能：

- **rust-analyzer** — Rust 言語サーバー
- **Even Better TOML** — Cargo.toml のサポート
- **crates** — 依存関係バージョン管理
- **Error Lens** — インラインエラー表示

`.vscode/settings.json` の推奨設定：

```json
{
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.check.command": "clippy",
  "rust-analyzer.check.extraArgs": ["--", "-W", "clippy::pedantic"]
}
```

## トラブルシューティング

### データベース接続エラー

```
Failed to connect to database
```

- PostgreSQL コンテナが起動しているか確認: `docker compose ps`
- `DATABASE_URL` が正しいか確認
- ポート 5432 が使用中でないか確認

### SQLx コンパイルエラー

```
error: error communicating with database
```

- PostgreSQL が起動・接続可能であること
- マイグレーションが実行されていること（初回起動時に自動実行）
- `cargo sqlx prepare` でオフラインデータを更新

## 関連ドキュメント

- [ビルドシステム](build.md)
- [テスト](testing.md)
- [設定リファレンス](../reference/configuration.md)
- [トラブルシューティング](../guides/troubleshooting.md)
