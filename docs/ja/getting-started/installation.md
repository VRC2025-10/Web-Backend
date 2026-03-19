# インストール

> **ナビゲーション**: [ドキュメントホーム](../README.md) > [はじめに](README.md) > インストール

VRC Backend のインストールと設定に必要なすべてを解説します。

---

## 前提条件

| 要件 | バージョン | 用途 |
|------|-----------|------|
| Rust | 1.85+（Edition 2024） | バックエンドのビルドと実行 |
| PostgreSQL | 16+ | プライマリデータベース |
| Docker & Docker Compose | 最新版 | コンテナ化された開発・デプロイ |
| Discord Application | — | OAuth2 認証 |

---

## 方法1: Docker 開発環境（推奨）

Docker Compose で PostgreSQL を実行しながら、ホスト上で Rust アプリケーションを実行します。最も素早く始められる方法です。

### 1. リポジトリのクローン

```bash
git clone https://github.com/your-org/Web-Backend.git
cd Web-Backend
```

### 2. 環境変数の設定

```bash
cp .env.example .env
```

`.env` を編集し、必須変数を入力します：

```dotenv
# データベース（docker-compose.yml のデフォルトに合わせる）
DATABASE_URL=postgres://postgres:postgres@localhost:5432/vrc

# Discord OAuth2（下記「Discord Application の設定」を参照）
DISCORD_CLIENT_ID=your_discord_client_id
DISCORD_CLIENT_SECRET=your_discord_client_secret
DISCORD_REDIRECT_URI=http://localhost:3000/api/v1/auth/callback
DISCORD_GUILD_ID=your_discord_guild_id

# シークレット
SESSION_SECRET=<32文字以上>
SYSTEM_API_TOKEN=<64文字以上>

# フロントエンド
FRONTEND_ORIGIN=http://localhost:5173
```

> **ヒント**: シークレットの生成方法：
> ```bash
> openssl rand -hex 32   # SESSION_SECRET 用
> openssl rand -hex 64   # SYSTEM_API_TOKEN 用
> ```

### 3. PostgreSQL の起動

```bash
docker compose up -d
```

### 4. ビルドと実行

```bash
cargo run
```

データベースマイグレーションは起動時に自動的に実行されます。

---

## 方法2: 手動セットアップ

Docker を使わずに、すべてを手動でインストール・設定します。

### 1. Rust のインストール

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable
```

確認：

```bash
rustc --version   # 1.85+ であること
cargo --version
```

### 2. PostgreSQL のインストール

**Ubuntu/Debian:**

```bash
sudo apt update
sudo apt install postgresql-16 postgresql-client-16
sudo systemctl start postgresql
```

**macOS (Homebrew):**

```bash
brew install postgresql@16
brew services start postgresql@16
```

### 3. データベースの作成

```bash
sudo -u postgres psql -c "CREATE DATABASE vrc;"
sudo -u postgres psql -c "CREATE USER vrc_user WITH PASSWORD 'your_password';"
sudo -u postgres psql -c "GRANT ALL PRIVILEGES ON DATABASE vrc TO vrc_user;"
```

### 4. 環境変数の設定

```bash
cp .env.example .env
```

`DATABASE_URL` をローカルの PostgreSQL 設定に合わせて更新します：

```dotenv
DATABASE_URL=postgres://vrc_user:your_password@localhost:5432/vrc
```

その他の必須変数は方法1と同様に入力してください。

### 5. ビルドと実行

```bash
cargo run
```

マイグレーションは起動時に自動適用されるため、手動でのマイグレーション操作は不要です。

---

## 方法3: 本番 Docker デプロイ

`docker-compose.prod.yml` を使用して、完全なスタック（アプリケーション + PostgreSQL + Caddy リバースプロキシ）をデプロイします。

```bash
# 環境変数の設定
cp .env.example .env
# .env を本番用の値で編集（下記参照）

# フルスタックの起動
docker compose -f docker-compose.prod.yml up -d
```

### 本番環境の推奨設定

| 変数 | 推奨値 |
|------|--------|
| `COOKIE_SECURE` | `true` |
| `TRUST_X_FORWARDED_FOR` | `true` |
| `BIND_ADDRESS` | `0.0.0.0:3000`（デフォルト） |
| `DATABASE_MAX_CONNECTIONS` | 負荷に応じて調整（デフォルト: `10`） |
| `RUST_LOG` | `info` または `warn` |

`docker-compose.prod.yml` スタックには以下が含まれます：

- **app** — VRC Backend Rust アプリケーション
- **postgres** — PostgreSQL 16 データベース
- **caddy** — 自動 HTTPS 対応の Caddy リバースプロキシ

---

## Discord Application の設定

VRC Backend は認証に Discord OAuth2 を使用します。必要な認証情報を取得するには、Discord Application を作成する必要があります。

### 1. Discord Application の作成

1. [Discord Developer Portal](https://discord.com/developers/applications) にアクセス
2. **New Application** をクリックし、名前を入力
3. **Application ID** をメモ — これが `DISCORD_CLIENT_ID` になります

### 2. OAuth2 の設定

1. 左サイドバーの **OAuth2** に移動
2. **Redirects** に以下のリダイレクト URI を追加：
   - 開発環境: `http://localhost:3000/api/v1/auth/callback`
   - 本番環境: `https://your-domain.com/api/v1/auth/callback`
3. **Client Secret** をコピー — これが `DISCORD_CLIENT_SECRET` になります

### 3. Guild ID の取得

1. Discord を開き、**開発者モード** を有効化（ユーザー設定 → 詳細設定 → 開発者モード）
2. サーバー名を右クリックして **サーバー ID をコピー** を選択
3. これを `DISCORD_GUILD_ID` として設定

---

## インストールの確認

サーバーが正常に起動したことを確認するには、ヘルスチェックエンドポイントにアクセスします：

```bash
curl http://localhost:3000/health
```

サーバーが稼働中でデータベースに接続されていることを示すレスポンスが返されれば成功です。

---

## 関連ドキュメント

- [クイックスタート](quickstart.md) — 5分で起動
- [使用例](examples.md) — API 使用例集
- [設定リファレンス](../reference/configuration.md) — 全環境変数の詳細
- [環境変数クイックリファレンス](../reference/environment.md) — 環境変数の一覧表
