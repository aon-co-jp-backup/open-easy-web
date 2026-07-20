# open-easy-web

**「第二のKUSANAGI」— アプリのアップロード後にIPアドレスで起動し、
ドメイン登録・HTTPS化を簡単に自動適用できる運用ツール(Rust →
WebAssembly、フレームワーク不使用)**

WordPress高速化サーバー構築キット「KUSANAGI」のように、アプリを
アップロードしたら**IPアドレスから起動 → ドメイン登録の簡易化 →
HTTPS自動化**までを一気通貫でこなすことを目指す運用ツールです。
複数サイトの接続先を登録・切替・疎通確認できる「サイト管理」画面、
WordPress・PHP + Laravel・Python + FastAPIなど任意のバックエンド
スタック向けの基本的なリバースプロキシ設定(Nginx/Apache)を自動生成
できます。**DB(データベース)への接続機能は持ちません**(意図的に
スコープ外)。

**2026-07-13、`aruaru-web` からのスコープ分離**: `aruaru-web` が
開発していた機能のうち「ドメイン/サブドメインの簡単な登録・削除」
「HTTPS自動監視・自動発行・自動更新」「アップロード後の簡単な
サイト運用」——**KUSANAGIのWeb高速化機能を除く全て**——を、この
`open-easy-web` へ引き継ぎました。KUSANAGI的な高速化機能
(gzip圧縮・静的アセットの長期キャッシュ・FastCGIバッファ調整・
upstream keepaliveプーリング)は、Nginx/Apacheの設定生成ではなく
**`open-runo`/RPoem(旧poem-cosmo-tauri)側のネイティブRust(hyperミドルウェア)
実装として統合**されました(gzip応答圧縮ミドルウェア・静的アセット
Cache-Controlミドルウェア等、詳細は両リポジトリのCLAUDE.md参照)。

📖 他の言語: [日本語](README-Japan.md) / [English](README-English.md) /
[中文](README-Chinese.md) / [한국어](README-Korea.md) / [Español](README-Spain.md) /
[Français](README-France.md) / [Deutsch](README-Germany.md) / [Italiano](README-Italy.md) /
[Русский](README-Russia.md) / [العربية](README-Arabic.md)

---

## いまできること

- **サイト管理画面**: open-easy-web自身・WordPress・Laravel・FastAPIなど
  任意のバックエンドスタックのデプロイ先(IPアドレス/ドメイン/サブドメイン/
  ポート/パス)を複数登録し、`localStorage` に保存してワンクリックで選択・
  疎通確認できる(KUSANAGIのサイト一覧に相当)。カードごとに**「接続テスト」
  ボタン**(選択中のサイトを変えずに単純なHTTP到達性確認のみ実行)、ポート
  番号の入力検証(1〜65535)、登録済みサイト一覧の**JSONエクスポート/
  インポート**(バックアップ・他ブラウザへの持ち出し用)、削除前の確認
  ダイアログを備える。
- **IPアドレスから起動**: `scripts/serve.sh` でローカル/VPS上の任意のIP・
  ポートにbindして配信できる。
- **vhost生成・HTTPS自動設定**: `scripts/gen-vhost.sh` で、
  ドメイン・IP・バックエンドスタックの組み合わせから Nginx/Apache の
  vhost(HTTP→HTTPSリダイレクト込み)を生成する。`static`(静的サイト)・
  `proxy`(任意のHTTPバックエンド向け汎用リバースプロキシ)・
  `wordpress`・`laravel`・`fastapi` の5スタックに対応。**高速化
  チューニング(gzip・静的キャッシュ・FastCGIバッファ・upstream
  keepalive)はここには含まれない**(`open-runo`/RPoem(旧poem-cosmo-tauri)側の
  ネイティブRust実装が担当)。
- **HTTPS(TLS)の自動監視・自動更新**: `scripts/setup-tls.sh` で
  Let's Encrypt(certbot)の証明書取得、`deploy/systemd/
  install-systemd-units.sh` で「1日2回の自動更新
  (`easyweb-tls-renew.timer`)」と「1日1回の失効監視
  (`easyweb-tls-monitor.timer` → `scripts/check-all-tls.sh`)」を有効化できる。
- **VPSへのデプロイ**: Windows PowerShellから `scripts/deploy-vps.ps1` を
  実行するだけで、ビルド → VPSへのアップロード → 起動までを自動化できる。
- **アカウント認証(パスワード不使用)**: 固定パスワードを一切使わず、
  メール1・メール2(セカンドメール)・電話番号のいずれかへのワンタイム
  パスワード(OTP)でログインする。認証アプリ(TOTP、Google Authenticator
  等)による2段階認証も有効化でき、**メールOTPと認証アプリコードの
  どちらか一方だけでもログイン可能**(2FA有効時、メールOTPを経由せず
  認証アプリのコードだけでログインする専用の導線も用意)。連絡先の変更は
  必ず現在の主メール宛の確認リンク経由(アカウント乗っ取り防止)。
  **2026-07-15時点、セキュリティ上の理由で公開の新規登録(サインアップ)は
  無効化されており、起動時にシードされる固定アカウント1件のみがログイン
  可能**(`server/src/main.rs`の`FIXED_ACCOUNT_EMAIL`)。複数アカウントを
  運用したい場合は、現状はこの固定アカウントの仕組みを自分の環境向けに
  書き換える必要がある。
- **AIによる自動PHP判定**: サイトへファイルをアップロードすると、外部LLM・
  契約不要の自己学習型AI(ファイル拡張子・`<?php`タグ・`wp-config.php`・
  `composer.json`等のシグネチャをスコアリング)がPHPサイトかどうかを判定し、
  該当すればnginx + PHP-FPMのvhostを自動生成・配置する。判定結果は
  手動で訂正でき、訂正のたびにAIの重みがオンライン学習(EWMA式)で
  補正される。
- **共有バックエンドへの動的登録(「分身の術」)**: ドメインごとに
  `open-runo`/RPoem(旧poem-cosmo-tauri)の新規プロセスを個別インストール
  する代わりに、既に稼働中の共有バックエンドへこのサイトのドメインを
  動的登録できる(ドメイン追加のたびにバックエンドプロセスを増やす
  必要が無い)。

## いまできないこと(正直な範囲)

- **Web高速化(KUSANAGI的なgzip圧縮・静的キャッシュ・FastCGIバッファ調整・
  upstream keepaliveプーリング)は持たない**(意図的にスコープ外——
  `open-runo`/RPoem(旧poem-cosmo-tauri)側のネイティブRust実装を参照)。
- **DB(データベース)への接続機能は持たない**。SQL実行・GraphQLクエリなど
  特定のデータベース製品に依存する機能は意図的にスコープ外。
- ページネーション・エラー時の自動リトライは未実装。
- Tauriのようなネイティブアプリ体験は提供しない(ブラウザで動くWASMのみ)。
- **実際のドメイン取得・DNSレコード登録(レジストラでの操作)はこの
  リポジトリからは行わない**。ここで自動化しているのは、取得済み
  ドメインに対する「vhost設定生成」「TLS証明書の取得・監視・自動更新」
  までであり、DNS登録自体は利用者がレジストラで行う。
- 実際のVPS契約(レンタルサーバー事業者との契約)もこのリポジトリからは
  行わない。

## ビルド方法

Node.js・npm・TypeScriptは使わない。Rustツールチェーンのみで完結する。

```bash
rustup target add wasm32-unknown-unknown        # 初回のみ
cargo install wasm-bindgen-cli --version 0.2.126 # 初回のみ(Cargo.lockのバージョンと一致させること)

cargo build --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir pkg \
  target/wasm32-unknown-unknown/debug/open_easy_web.wasm

# 静的サーバーで配信して開く(何でもよい。例:)
python -m http.server 8080
# ブラウザで http://localhost:8080/index.html を開く
```

## IPアドレスから起動する

```bash
scripts/serve.sh 0.0.0.0 8080        # 全インターフェースで待受
scripts/serve.sh 192.168.1.50 8080   # 特定のIPアドレスのみで待受
```

## 廃止済みサービスの残骸監査(dry-run、2026-07-14追加)

ドメイン/サブコンテンツを削除した後、そのサービス専用のsystemd
unit・crontabエントリ・certbot証明書更新設定が残っていないかを検出
します。**削除は一切行いません**(検出結果を一覧表示するのみ)——
実際の削除は、一覧を確認した上で人間が個別に実行してください
(誤検知で無関係なサービスを巻き添えにしないための意図的な設計)。

```bash
scripts/audit-orphaned-services.sh <廃止したサービス名やドメイン名>
# 例:
scripts/audit-orphaned-services.sh aruaru-web
```

## VPSへのデプロイ(Windows PowerShellから)

```powershell
.\scripts\deploy-vps.ps1 -VpsHost 203.0.113.10 -VpsUser root -StartServer
```

## vhost生成・ドメイン/サブドメインの登録

```bash
# open-easy-web自身(静的サイト)
scripts/gen-vhost.sh --stack=static easyweb.example.com 203.0.113.10

# 任意のバックエンドへの汎用リバースプロキシ
scripts/gen-vhost.sh --stack=proxy tool.example.com 203.0.113.10 127.0.0.1:9000

# WordPress(PHP-FPMソケット/アドレスを指定)
scripts/gen-vhost.sh --stack=wordpress blog.example.com 203.0.113.10 \
  unix:/run/php/php8.3-fpm.sock /var/www/blog

# Laravel(publicディレクトリを明示)
scripts/gen-vhost.sh --stack=laravel app.example.com 203.0.113.10 \
  unix:/run/php/php8.3-fpm.sock /var/www/app/public

# FastAPI(ASGIサーバーへのリバースプロキシ、WebSocket/ストリーミング対応)
scripts/gen-vhost.sh --stack=fastapi api.example.com 203.0.113.10 127.0.0.1:8000
```

生成された設定ファイル(`deploy/generated/` 以下、`.gitignore` 対象)を
Nginx/Apacheの設定ディレクトリに配置してリロードした後、証明書を取得する:

```bash
scripts/setup-tls.sh easyweb.example.com admin@example.com /var/www/easyweb.example.com

# 自動更新(1日2回)+ 自動監視(1日1回、失効間近を検知)を有効化
sudo deploy/systemd/install-systemd-units.sh
```

## 動作確認(このパスで実施)

- `cargo check --target wasm32-unknown-unknown` / `cargo build --target
  wasm32-unknown-unknown` / `cargo clippy --target wasm32-unknown-unknown`
  ともに成功(警告0件)。
- `wasm-bindgen --target web` で `pkg/open_easy_web.js` /
  `pkg/open_easy_web_bg.wasm` を生成し、ビルド成果物を確認済み。
- `scripts/gen-vhost.sh` を全5スタック(static/proxy/wordpress/laravel/
  fastapi)で実行し、`{{DOMAIN}}`/`{{IP}}`/`{{UPSTREAM}}`/`{{WEBROOT}}`の
  プレースホルダが正しく置換されることを確認済み。**Windows環境である
  ため、`nginx -t`/`apache2ctl configtest`によるバイナリでの実構文検証は
  この開発環境では未実施**(aruaru-webの過去パスがLinuxコンテナで
  実施した検証と同様の手順を、実際にNginx/Apacheが利用可能な環境で
  行うことを推奨。全テンプレートはaruaru-web側で検証済みだった
  テンプレートから高速化ディレクティブのみを除去した差分であり、
  文法的な破壊的変更は含まない)。
- 実際のcertbotによるLet's Encrypt発行、`scripts/deploy-vps.ps1`の実VPS
  環境での動作は未検証(詳細はCLAUDE.md参照)。

## 構成

```text
open-easy-web/
├── Cargo.toml             # ルートクレート(WASM UI、cdylib+rlib)
├── src/
│   ├── lib.rs
│   ├── dom.rs
│   ├── profiles.rs        # サイト管理(接続プロファイル)
│   ├── shell.rs           # 画面HTML組み立て
│   ├── api_auth.rs        # 認証API fetch()ラッパー
│   ├── api_upload.rs      # アップロード/ドメイン登録API fetch()ラッパー
│   ├── auth_ui.rs         # 認証UI DOM配線
│   └── view_bridge.rs     # open-runo-view(Phase 3 SSR hydration)連携
├── server/                # バックエンドREST API(別クレート、tokio/hyper直接実装)
│   ├── Cargo.toml         # バイナリ名: open-easy-web-server
│   └── src/               # auth/users/totp/mail/sms/tls/vhost/
│                          # php_detector/upload/appserver_registration/main
├── index.html
├── pkg/                   # wasm-bindgen生成物(.gitignore対象)
├── scripts/
│   ├── serve.sh / deploy-vps.ps1 / gen-vhost.sh
│   ├── setup-tls.sh / check-tls.sh / check-all-tls.sh
│   ├── switch-engine.sh / switch-app-server.sh
│   └── audit-orphaned-services.sh
├── deploy/
│   ├── nginx/vhost-{static,proxy,wordpress,laravel,fastapi,php,php-http-only}.conf.template
│   ├── apache/vhost-{static,proxy,wordpress,laravel,fastapi,php}.conf.template
│   ├── systemd/
│   └── generated/          # .gitignore対象
├── docs/
│   └── HYBRID_NETWORK_ARCHITECTURE.md
├── PORTING.md
└── CLAUDE.md
```

## 関連プロジェクト

- **aruaru-web**(分離元、高速化スコープの旧居場所): https://github.com/aon-co-jp/aruaru-web
- **open-runo**(高速化機能のネイティブRust実装先): https://github.com/aon-co-jp/open-runo
- **RPoem**(旧poem-cosmo-tauri、実装の先行地点): https://github.com/aon-co-jp/RPoem
- **aruaru-db**: https://github.com/aon-co-jp/aruaru-db
- **open-web-server**: https://github.com/aon-co-jp/open-web-server
- **open-raid-z**(開発ルールの正本): https://github.com/aon-co-jp/open-raid-z
- **rs-to-readme**: https://github.com/aon-co-jp/rs-to-readme

## License

Apache-2.0
