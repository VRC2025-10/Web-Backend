# クイックスタート

> **ナビゲーション**: [ドキュメントホーム](../README.md) > [はじめに](README.md) > クイックスタート

VRC Backend を5分で起動しましょう。

---

## 1. リポジトリのクローン

```bash
git clone https://github.com/your-org/Web-Backend.git
cd Web-Backend
```

## 2. 環境ファイルのコピー

```bash
cp .env.example .env
```

## 3. Discord OAuth2 認証情報の設定

`.env` を開き、Discord 関連の変数を入力します：

```dotenv
# Discord Application の ID（Developer Portal → General Information から取得）
DISCORD_CLIENT_ID=123456789012345678

# Discord Application のシークレット（Developer Portal → OAuth2 から取得）
DISCORD_CLIENT_SECRET=your_client_secret_here

# Discord Application に登録したリダイレクト URI と一致させる
DISCORD_REDIRECT_URI=http://localhost:3000/api/v1/auth/callback

# 認証を許可する Discord サーバーの ID
DISCORD_GUILD_ID=987654321098765432

# フロントエンドアプリケーションの URL（CORS 用）
FRONTEND_ORIGIN=http://localhost:5173
```

> Discord Application をまだ作成していない場合は、[インストールガイド — Discord Application の設定](installation.md#discord-application-の設定) を参照してください。

## 4. シークレットの生成

```bash
# セッションシークレット（最低32文字）
echo "SESSION_SECRET=$(openssl rand -hex 32)" >> .env

# System API トークン（最低64文字）
echo "SYSTEM_API_TOKEN=$(openssl rand -hex 64)" >> .env
```

## 5. PostgreSQL の起動

```bash
docker compose up -d
```

`.env.example` のデフォルト `DATABASE_URL` に対応した PostgreSQL 16 コンテナが起動します。

## 6. サーバーの起動

```bash
cargo run
```

初回ビルドには数分かかります。起動時にサーバーは以下を実行します：

1. PostgreSQL への接続
2. 未適用のデータベースマイグレーションの自動実行
3. バックグラウンドタスクの起動（セッションクリーンアップ、イベントアーカイブ）
4. `http://localhost:3000` でリッスン開始

## 7. 動作確認

```bash
curl http://localhost:3000/health
```

サーバーが稼働中でデータベースに接続されていることを示すレスポンスが返されるはずです。

## 8. 公開 API の試行

```bash
curl http://localhost:3000/api/v1/public/members
```

メンバーのページネーション付きリストが返されます。公開 API には認証は不要です。

## 9. 次のステップ

| 次のステップ | リンク |
|-------------|--------|
| その他の API 使用例を確認 | [使用例](examples.md) |
| インストールオプションの全容を確認 | [インストール](installation.md) |
| API リファレンスを読む | [API リファレンス](../reference/api/README.md) |
| テストスイートの実行 | `make test` |
| コード品質の確認 | `make lint && make fmt` |

---

## クイックリファレンス

| 操作 | コマンド |
|------|---------|
| データベースの起動 | `docker compose up -d` |
| データベースの停止 | `docker compose down` |
| サーバーの実行 | `cargo run` |
| テストの実行 | `make test` |
| リリースビルド | `make build` |
| ヘルスチェック | `curl http://localhost:3000/health` |

---

## 関連ドキュメント

- [インストール](installation.md) — 全インストール方法とトラブルシューティング
- [使用例](examples.md) — 包括的な API 使用例
- [API リファレンス](../reference/api/README.md)
