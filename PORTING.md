# PORTING.md — open-easy-web お引越しファイル

> このファイル1枚で、他プロジェクトへ `open-easy-web` を導入・移設できます。
> 対象バージョン: 0.1.0(2026-07-13、`aruaru-web` からの分離初版。
> 2026-07-20、デプロイ先既定パス変更・ネットワークドライブ移設時の
> 注意事項を追記)。

## 0. このリポジトリのスコープ

`open-easy-web` は「第二のKUSANAGI」——アプリのアップロード後にIP
アドレスで起動し、ドメイン登録・HTTPS自動化を簡単に行える、DBに
依存しない汎用デプロイ・運用ツール。2026-07-13に `aruaru-web` から
分離: **KUSANAGIの高速化機能(gzip・静的キャッシュ・FastCGIバッファ・
upstream keepalive)を除く全て**を引き継いだ。高速化機能自体は
`open-runo`/`poem-cosmo-tauri` 側でネイティブRust実装として提供される
(このリポジトリはその機能を持たない・意図的に持たせない)。

## 1. 持っていくもの(ファイル一覧)

```
open-easy-web/
├── Cargo.toml / Cargo.lock
├── src/{lib,dom,profiles,shell,api_auth,api_upload,auth_ui,view_bridge}.rs
│                              # site management + auth + upload WASM UI
├── server/                   # 別クレート(open-easy-web-server、tokio/hyper直接実装)
│   ├── Cargo.toml / Cargo.lock
│   └── src/{main,auth,users,totp,mail,sms,tls,vhost,php_detector,upload,
│             appserver_registration}.rs
├── index.html / pkg/(ビルド生成物、.gitignore対象)
├── scripts/
│   ├── serve.sh              # IPアドレス起動
│   ├── gen-vhost.sh          # vhost生成(高速化ディレクティブ抜き)
│   ├── setup-tls.sh / check-tls.sh / check-all-tls.sh
│   ├── switch-engine.sh / switch-app-server.sh
│   ├── audit-orphaned-services.sh
│   └── deploy-vps.ps1
├── deploy/
│   ├── nginx/vhost-{static,proxy,wordpress,laravel,fastapi,php,php-http-only}.conf.template
│   ├── apache/vhost-{static,proxy,wordpress,laravel,fastapi,php}.conf.template
│   ├── systemd/{easyweb-tls-renew,easyweb-tls-monitor}.{service,timer}
│   └── generated/(.gitignore対象)
├── docs/HYBRID_NETWORK_ARCHITECTURE.md
├── PORTING.md(本ファイル)
└── CLAUDE.md
```

丸ごと移設する場合はフォルダごとコピーして
`cargo build --target wasm32-unknown-unknown`(ルートのWASM UIクレート)と
`cd server && cargo build`(バックエンドAPIクレート、別Cargo.toml・別
ワークスペース)の**両方**が通れば移設成功。ルートクレートだけをコピーして
`server/`を忘れると、認証・アップロード・AI PHP判定・ドメイン自動登録
機能が丸ごと欠落するので注意(WASM UI単体では静的ファイル配信専用の
旧`scripts/serve.sh`相当の機能しか持たない)。

## 2. ビルド

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.126
cargo build --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir pkg \
  target/wasm32-unknown-unknown/debug/open_easy_web.wasm
```

> ⚠️ **移設先がネットワーク共有ドライブの場合の注意(2026-07-20、実際に
> 発生した事故から追記)**: 移設先が(このリポジトリの元々の開発環境と
> 同様に)SMB等でマウントしたネットワーク共有ドライブの場合、`cargo
> build`の`target/`出力や`wasm-bindgen`の入出力を**そのドライブ上で
> 直接読み書きすると、書き込み直後の読み取りが古い内容を返すことがある**
> (読み取りキャッシュの不整合)。この不整合により、`wasm-bindgen`が
> 生成したJSグルーコードが古いファイル名を内部参照したまま本番へ
> デプロイされ、実際に画面が白くなる事故(`WebAssembly.instantiate():
> Import #0 ... module is not an object or function`)が発生した。
> 再ビルドしても変更が反映されない場合は、`cargo build --target-dir
> <ローカルドライブの一時ディレクトリ>`でビルド出力先をネットワーク
> ドライブ外(ローカルのC:等)に切り替え、`wasm-bindgen`もそのローカル
> コピーの`.wasm`に対して実行し、生成物だけをリポジトリへコピーし
> 戻すこと。また、`wasm-bindgen`は入力`.wasm`ファイル名のstemを基に
> JSグルーコード内の相対import(`<stem>_bg.wasm`/`<stem>_bg.js`)を
> 生成するため、**入力ファイル名は最終的にデプロイする出力ファイル名と
> 一致させること**(後から出力ファイルだけをリネームすると内部参照が
> 古い名前のまま残る)。

## 3. vhostテンプレートの再利用

`deploy/nginx/vhost-<stack>.conf.template` /
`deploy/apache/vhost-<stack>.conf.template` は `{{DOMAIN}}` /
`{{IP}}` / `{{UPSTREAM}}` / `{{WEBROOT}}` のプレースホルダを
`sed` で置換するだけの単純なテンプレート。`scripts/gen-vhost.sh` を
経由せず、他プロジェクトのデプロイスクリプトから直接 `sed` で
利用してもよい。**高速化ディレクティブ(gzip/expires/Cache-Control/
fastcgi_buffers/upstream keepalive)は意図的に含まれていない** —
必要な場合は `open-runo`/RPoem(旧`poem-cosmo-tauri`)の
`with_compression`/`with_static_cache_headers` ミドルウェア
(hyperベース、`crates/open-runo-router/src/middleware_hyper.rs`)を
参照して自前のRustサーバー側に組み込むこと。

## 4. HTTPS自動監視・自動更新の移植

```bash
sudo deploy/systemd/install-systemd-units.sh
```

`easyweb-tls-renew.timer`(1日2回、certbot renew)・
`easyweb-tls-monitor.timer`(1日1回、`scripts/check-all-tls.sh`)を
`systemd` に登録する。他プロジェクトへ移植する場合はunit名の
prefix(`easyweb-`)を変更し、`ExecStart`のパスをリポジトリの実際の
配置場所に合わせること。

## 5. VPSデプロイの移植

`scripts/deploy-vps.ps1`(Windows PowerShell)はビルド→`scp`アップ
ロード→`ssh`経由の`serve.sh`起動を自動化する。他プロジェクトの
バイナリ/静的ファイルを同時にアップロードしたい場合は
`-OpenWebServerPath`相当の追加パラメータを増設する形で拡張できる。
**アップロード先の既定パスは2026-07-20時点で`/root/RUNO/open-easy-web`**
(`-RemoteAruaruPath`パラメータで上書き可能。旧既定値`/root/open-easy-web`
から変更されたので、既存VPSに旧パスで運用中の環境を移設する場合は
`systemd` unit(`WorkingDirectory`/`ExecStart`/
`Environment=OPEN_EASYWEB_STATIC_DIR`)側のパスも合わせて更新すること)。

## 6. 動作確認

```bash
cd open-easy-web
cargo check --target wasm32-unknown-unknown
cargo test --target wasm32-unknown-unknown   # 現状ユニットテストなし(WASM UI、DOM結合のためテストは実ブラウザ手動確認が中心)
bash scripts/gen-vhost.sh --stack=proxy example.com 203.0.113.10 127.0.0.1:9000

# バックエンドAPIクレート(server/)側も別途確認すること(ルートの
# cargo check だけでは検証されない)。認証(OTP/TOTP)・アップロード・
# AI PHP判定・ドメイン自動登録・appserver_registrationのテスト一式が
# ここに入っている。
cd server
cargo check
cargo test
```

## 7. 命名規約

- クレート名: `open-easy-web` — Rustパス: `open_easy_web`
- バックエンドAPIクレート名: `open-easy-web-server`(`server/`、バイナリ名も同じ)
- systemd unit: `easyweb-*`
- localStorageキー: `openeasyweb_site_profiles_v1` /
  `openeasyweb_active_site_id_v1`

## 8. 移植・拡張時の注意

高速化機能(gzip・静的キャッシュ・FastCGIバッファ・upstream
keepalive)は、このリポジトリではなく`open-runo`/RPoem(旧`poem-cosmo-tauri`)
側のネイティブRust実装として提供する方針を維持すること。この
リポジトリへ高速化系のNginx/Apacheディレクティブを追加で持ち込む
プルリクエスト・変更は、エコシステム全体の方針(2026-07-13分離)と
矛盾するため避けること。技術選定で迷う場合は日本語・英語両方での
Google検索とGitHub調査を行い、学習データからの推測のみに頼らない
こと。

## 9. TOTP検証コードをテストで用意する際の罠(2026-07-23、実際に踏んだ
バグ、TOTPを使うあらゆる移植先に該当)

「サーバー側の`verify_code`(またはそれに相当する検証関数)が受理する
6桁コードを、0〜999999を総当たりして探す」という一見安全に見える
テスト手法は、**debugビルドでは正解の番号によって数秒〜数十秒かかる
ことがあり**、その間にTOTPの時間窓(既定30秒×スキュー許容ステップ数)
を超えてしまい、間欠的にテストが失敗する(flaky)実バグを引き起こす。

**正しい対処**: TOTPライブラリ側に「指定した時刻に対する正しいコードを
直接計算する」関数(本リポジトリでは`totp::code_at(secret,
unix_time)`)を用意し、`pub`(または`pub(crate)`)にしてテストコードから
直接呼び出す。総当たりを一切行わないため、実行時間もテストの安定性も
大幅に改善する(本リポジトリでの実測: 該当テスト1件あたり23秒→0.02秒)。
TOTP/HOTPを実装する他の移植先でも、テストコードに総当たりループが
無いか確認すること。

## 10. 「ルートで`cargo test`しても実は何も検証していない」構造の罠
(2026-07-23発見)

このリポジトリはルート(WASMフロントエンド用クレート)と`server/`
(バックエンドAPIクレート)が**別々の`[workspace]`宣言を持つ**構成に
なっている。ルートディレクトリで`cargo test --workspace`を実行しても、
バックエンドの実質的なテスト(50件超)は一切実行されない——`cd server
&& cargo test`と明示的に移動する必要がある。複数クレート・複数
ワークスペースに分割されたRustプロジェクトを扱う際は、「ルートで
`cargo test --workspace`を実行すれば全部検証したはず」という思い込み
を避け、実際に何件のテストが走ったかを毎回確認すること(0件で
成功する`cargo test`は「検証した」ことにならない)。
