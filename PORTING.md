# PORTING.md — open-easy-web お引越しファイル

> このファイル1枚で、他プロジェクトへ `open-easy-web` を導入・移設できます。
> 対象バージョン: 0.1.0(2026-07-13、`aruaru-web` からの分離初版)。

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
├── src/{lib,dom,profiles,shell}.rs   # site management WASM UI
├── index.html / pkg/(ビルド生成物、.gitignore対象)
├── scripts/
│   ├── serve.sh              # IPアドレス起動
│   ├── gen-vhost.sh          # vhost生成(高速化ディレクティブ抜き)
│   ├── setup-tls.sh / check-tls.sh / check-all-tls.sh
│   └── deploy-vps.ps1
├── deploy/
│   ├── nginx/vhost-{static,proxy,wordpress,laravel,fastapi}.conf.template
│   ├── apache/vhost-{static,proxy,wordpress,laravel,fastapi}.conf.template
│   ├── systemd/{easyweb-tls-renew,easyweb-tls-monitor}.{service,timer}
│   └── generated/(.gitignore対象)
├── PORTING.md(本ファイル)
└── CLAUDE.md
```

丸ごと移設する場合はフォルダごとコピーして
`cargo build --target wasm32-unknown-unknown` が通れば移設成功。

## 2. ビルド

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.126
cargo build --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir pkg \
  target/wasm32-unknown-unknown/debug/open_easy_web.wasm
```

## 3. vhostテンプレートの再利用

`deploy/nginx/vhost-<stack>.conf.template` /
`deploy/apache/vhost-<stack>.conf.template` は `{{DOMAIN}}` /
`{{IP}}` / `{{UPSTREAM}}` / `{{WEBROOT}}` のプレースホルダを
`sed` で置換するだけの単純なテンプレート。`scripts/gen-vhost.sh` を
経由せず、他プロジェクトのデプロイスクリプトから直接 `sed` で
利用してもよい。**高速化ディレクティブ(gzip/expires/Cache-Control/
fastcgi_buffers/upstream keepalive)は意図的に含まれていない** —
必要な場合は `open-runo`/`poem-cosmo-tauri` の
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

## 6. 動作確認

```bash
cd open-easy-web
cargo check --target wasm32-unknown-unknown
cargo test --target wasm32-unknown-unknown   # 現状ユニットテストなし(WASM UI、DOM結合のためテストは実ブラウザ手動確認が中心)
bash scripts/gen-vhost.sh --stack=proxy example.com 203.0.113.10 127.0.0.1:9000
```

## 7. 命名規約

- クレート名: `open-easy-web` — Rustパス: `open_easy_web`
- systemd unit: `easyweb-*`
- localStorageキー: `openeasyweb_site_profiles_v1` /
  `openeasyweb_active_site_id_v1`

## 8. 移植・拡張時の注意

高速化機能(gzip・静的キャッシュ・FastCGIバッファ・upstream
keepalive)は、このリポジトリではなく`open-runo`/`poem-cosmo-tauri`
側のネイティブRust実装として提供する方針を維持すること。この
リポジトリへ高速化系のNginx/Apacheディレクティブを追加で持ち込む
プルリクエスト・変更は、エコシステム全体の方針(2026-07-13分離)と
矛盾するため避けること。技術選定で迷う場合は日本語・英語両方での
Google検索とGitHub調査を行い、学習データからの推測のみに頼らない
こと。
