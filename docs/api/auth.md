# Auth API リファレンス

Auth API はフロントエンドから呼び出される Discord OAuth2 による認証フロー（ログイン処理周辺）を担当します。

## 概要と共通仕様

- **ベースパス:** `/api/v1/auth`
- **認証:** なし（このレイヤーが認証プロセスそのものを開始するため）
- **レート制限:** 10 リクエスト/分（IP アドレス単位でのスパムログインを防止）
- **Cache-Control:** ヘッダ付与なし（ブラウザ側でのキャッシュを防ぐためリダイレクトを使用）

---

## OAuth2 フロー の全体像

フロントエンドがログインを完了させる際の流れです：

1. **ステップ 1:** フロントエンド（ユーザー）がバックエンドの `GET /api/v1/auth/discord/login` にアクセス（`window.location.href` 等を使用）。
2. **ステップ 2:** バックエンドは CSRF 保護用の文字列（`oauth_state`）を生成し、それを Cookie に書き込んだ上で、**Discord の認可画面へユーザーをリダイレクト (302)** します。
3. **ステップ 3:** ユーザーが Discord の画面でアプリへの権限移譲（スコープの承認）をクリックします。
4. **ステップ 4:** Discord は指定されたリダイレクトURLであるバックエンドの `GET /api/v1/auth/discord/callback` へユーザーのブラウザをリダイレクトさせます（URLパラメータに `code` と先ほどの `state` が付与されます）。
5. **ステップ 5:** バックエンドは以下の処理を一気に突き抜けます：
   - Cookie 内の `oauth_state` とパラメータの `state` を比較（CSRF検証）
   - `code` を Discord API に送って「アクセストークン」を取得。
   - そのアクセストークンを用いてユーザーのプロフィールと「目的のギルド(Discord鯖)に参加しているか」を検証します。
   - 問題なければ、DBにユーザーを保存し、ログインセッションを発行(`session_id`)。
6. **ステップ 6:** 最後に、バックエンドはフロントエンドのトップページへ**リダイレクト(302)**します。この時、発行した `session_id` が Set-Cookie でブラウザに設定されています。

この後、フロントエンドは `GET /api/v1/internal/auth/me` を呼び出すことで、「現在誰がログインしているのか」の情報を得ることができます。

---

## エンドポイント一覧

- [GET /api/v1/auth/discord/login](#get-apiv1authdiscordlogin) - ログインフローの開始
- [GET /api/v1/auth/discord/callback](#get-apiv1authdiscordcallback) - OAuth2 コールバックの受取先

---

### GET /api/v1/auth/discord/login
OAuth2 の認可フローを開始し、Discord の公式認可プロンプト画面へユーザーを転送します。

フロントエンドからの呼び出しは、`fetch()` などの AJAX ではなく、通常はブラウザのページ遷移（リンク等）として実行します。
```js
// フロントエンド実装例
window.location.href = "https://api.example.com/api/v1/auth/discord/login";
```

- **URL:** `/api/v1/auth/discord/login`
- **Method:** `GET`
- **Rate Limit**: 10 requests / minute

#### Success Response (302 Found)
Discord 側の `https://discord.com/api/oauth2/authorize` に対してリダイレクトさせます。
また、ブラウザに CSRF チェック用としての Cookie を発行します。

**設定される Cookie:**
`Set-Cookie: oauth_state={{32バイトのランダム文字列}}; HttpOnly; SameSite=Lax; Max-Age=600; Path=/...`

---

### GET /api/v1/auth/discord/callback
Discord の認証認可画面でユーザーが「承認」を押した直後に、Discord 側からユーザーのリダイレクト先として呼び出されるエンドポイントです。**フロントエンドが手動でたたく API ではありません。**

- **URL:** `/api/v1/auth/discord/callback`
- **Method:** `GET`
- **Rate Limit**: 10 requests / minute

#### Query Parameters (Discord が付与してくる)
| Field | Type | Description |
|---|---|---|
| `code` | string | Discord API から一回だけアクセストークンと交換可能な認証コード |
| `state` | string | アプリケーションが `login` で生成し渡しておいたステートトークン |

#### バックエンドの内部処理とエラー分岐

このフロー中、以下のさまざまな検証がバックエンド内部で実行されます。失敗した場合は、エラーコードを付与してフロントエンドの `/login` ページに戻します。

1. **CSRF State 検証:** （`oauth_state` Cookie == `state` パラメータ）
   - 不一致なら `csrf_failed` エラーにより異常終了。
2. **Token Exchange:** `code` と `DISCORD_CLIENT_SECRET` を使ってアクセストークンを取得。
   - 無効・期限切れなら `ERR-AUTH-001` が内部で発生。
3. **Guild メンバーシップの確認:** 指定の公式用 Discord Server (`DISCORD_GUILD_ID`) にユーザーが参加しているか確認。
   - 未参加なら `ERR-AUTH-002` が内部で発生。

#### 正常終了時の処理 (302 Found)

全てのチェックを通った場合、新しいログイントークンとして `session_id` が Cookie に焼き込まれ、ユーザーはフロントエンドのトップページ (`FRONTEND_ORIGIN`) に戻ります。

**設定される Cookie:**
`Set-Cookie: session_id={{UUID}}; HttpOnly; SameSite=Lax; Max-Age=604800; Path=/...`
*(※不要になった `oauth_state` Cookie も同時に無効化されます)*

**リダイレクト先 (LocationHeader):**
`Location: https://YOUR_FRONTEND_URL`

#### エラー時の処理 (302 Found)

処理中に何らかのエラーが起きた場合は、認証が完了していない状態でフロントエンドのログイン画面に戻されます。フロントエンドはこの URL パラメータからエラー理由をユーザーに表示します。

**リダイレクト先:**
`Location: https://YOUR_FRONTEND_URL/login?error=<error_code>`

発生しうる `error_code` パラメータ：
- `csrf_failed` - Cookie と state が一致しなかった（タブを開きっぱなしで古いなど）。
- `login_failed` - Discord からユーザー情報を取れなかった、または指定ギルドに参加していない。
- `server_error` - バックエンドDBや通信の内部的エラー。
