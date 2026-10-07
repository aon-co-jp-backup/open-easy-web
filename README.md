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

📘 **セルフホスト時のアカウント設定・2FA(フィーチャーフォンでの確認方法含む)は
[manual-JAPAN.md](manual-JAPAN.md) を参照してください。**(多言語版あり)

---

## 🚀 なぜ関連プロジェクトと一緒に使うと強力なのか / Why using this together with related projects is powerful (2026-09-07追加)

**日本語**: `https://easy-web.tokyo` は、**好きな有料ドメイン、または
DuckDNS等の無料ドメイン/サブドメインを簡単に設定し、HTTPS(URLの
末尾の`s`)を簡単に付けられる**ことを示すための運用サイトです
(上記「ローカルモード」節・「簡単ドメイン設定ウィザード」節参照)。

- **open-easy-webはApache+Nginx互換**(vhost自動生成、上記「配信
  エンジン」節参照)、**RPoem(旧poem-cosmo-tauri)はTomcat互換**の
  アプリケーションサーバー層として機能します。この2つを組み合わせる
  ことで、高速・高セキュリティな「4層4重」通信の構成が実現できます
  (詳細は`CLAUDE.md`「アプリケーションサーバー層の役割」節・
  `open-raid-z/CLAUDE.md`参照)。
- **open-raid-zとaruaru-dbを組み合わせる**ことで、高速・高セキュリティ
  なDATABASE運用も同時に実現できます。**さらにPostgreSQLを併用する
  ことで**(open-english側で実装・実機検証済みのDUAL DB同時書き込み
  機能と同じ設計思想)、**どちらを先にインストールしても後から
  インストールしても、片方のDATABASEに障害が発生した場合、もう片方の
  DATABASEが自動的にデータを補完・復旧できます**——**正直な開示**:
  この自動補完・自動復旧の実装本体は`aruaru-db`/`open-raid-z`側に
  あり、`open-easy-web`自体はこれらを組み合わせて使うための土台
  (ドメイン割り当て・リバースプロキシ)を提供するに留まります。
- **一緒にインストールをおすすめするもの**: `aruaru-llm`
  (契約不要の独自AIチャットコマース応答サービス、`open-cuda`とSET
  構成)——Windowsインストーラーに任意タスクとして追加済み(下記
  「インストーラー」節参照)。
  **正直な開示(誇張しないこと)**: `open-cuda`自体は`aruaru-llm`
  バイナリへ静的リンクされるライブラリであり、独立したサービスとして
  別途インストールする対象ではありません(`open-english`側の
  2026-08-19/2026-08-20の調査で確認済みの事実、詳細は
  `open-english/CLAUDE.md`参照)。`open-directx`は`open-cuda`とは
  無関係な別リポジトリ(Breakout風2Dデモ、GPU描画の実験用)であり、
  `open-easy-web`・`aruaru-llm`とも機能的な依存関係はありません
  ——このため`open-cuda`/`open-directx`は本リポジトリのインストーラー
  へは同梱していません(実体の無い「同梱したふり」を避けるため)。

**English**: `https://easy-web.tokyo` is an operational demo site
showing how easy it is to **assign either a paid domain of your choice,
or a free domain/subdomain via DuckDNS, and add HTTPS** (see the "Local
mode" and "Free domain setup wizard" sections above).

- **open-easy-web is Apache+Nginx compatible** (auto-generated vhosts,
  see the "Delivery engine" section above), and **RPoem (formerly
  poem-cosmo-tauri) is Tomcat-compatible** as the application-server
  layer. Combining the two gives you a high-speed, high-security
  "4-layer, 4-fold" communication setup (see `CLAUDE.md`'s "Role of the
  application server layer" section and `open-raid-z/CLAUDE.md`).
- **Combining open-raid-z and aruaru-db** also gives you a high-speed,
  high-security DATABASE. **Adding PostgreSQL on top** (same design
  philosophy as the DUAL DB simultaneous-write feature already
  implemented and field-tested in open-english) means **regardless of
  which database you install first or second, if one DATABASE fails,
  the other automatically completes/recovers its data**. **Honest
  disclosure**: the actual auto-heal/auto-recovery implementation lives
  in `aruaru-db`/`open-raid-z` — `open-easy-web` itself only provides
  the foundation (domain assignment, reverse proxying) for using them
  together.
- **Recommended companion to install alongside**: `aruaru-llm` (a
  contract-free proprietary AI chat-commerce response service, paired
  with `open-cuda` in a SET configuration) — already added as an
  optional installer task (see the "Installer" section below).
  **Honest disclosure (not overclaiming)**: `open-cuda` itself is a
  library statically linked into the `aruaru-llm` binary, not a
  standalone service to install separately (confirmed by open-english's
  2026-08-19/2026-08-20 investigation, see `open-english/CLAUDE.md`).
  `open-directx` is an unrelated separate repository (a Breakout-style
  2D demo for experimenting with GPU rendering) with no functional
  dependency on `open-cuda`, `open-easy-web`, or `aruaru-llm` — for
  this reason, `open-cuda`/`open-directx` are NOT bundled into this
  repository's installer (to avoid pretending to bundle something with
  no substance behind it).

## いまできること

- **スマホ縦画面レスポンシブ対応・英語(日本語)ハイブリッド表示(2026-07-24追加)**:
  `index.html`が`@media (max-width: 600px)`でスマホ幅の単一カラム
  レイアウト・タップ操作向けサイズ(44px以上)に自動切り替わる。UIの
  見出し・ボタン・フォームラベルは「英語表記の直後に(日本語)」形式
  (例: `Save (保存)`)で常時両方表示(切り替えスイッチャーではない)。
- **サイト管理画面**: open-easy-web自身・WordPress・Laravel・FastAPIなど
  任意のバックエンドスタックのデプロイ先(IPアドレス/ドメイン/サブドメイン/
  ポート/パス)を複数登録し、`localStorage` に保存してワンクリックで選択・
  疎通確認できる(KUSANAGIのサイト一覧に相当)。カードごとに**「接続テスト」
  ボタン**(選択中のサイトを変えずに単純なHTTP到達性確認のみ実行)、ポート
  番号の入力検証(1〜65535)、登録済みサイト一覧の**JSONエクスポート/
  インポート**(バックアップ・他ブラウザへの持ち出し用)、削除前の確認
  ダイアログを備える。
- **システムメモリ・ディスク使用状況の円グラフ表示(2026-07-31/
  2026-08-04追加)**: このサーバーが動いているマシンの実メモリ・
  仮想メモリ(スワップ)・実HDD/SSDの使用状況を`sysinfo`クレート経由で
  取得し、GUIの円グラフ+GiB表示で確認できる(`/admin/system/memory`・
  `/admin/system/disk`、`x-admin-token`認証)。同じ画面から電源プロファイル
  (省電力/省メモリ/常時電源接続、独立チェックボックスで組み合わせ自由)も
  切り替えられる。
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
- **分散同期・ディザスタリカバリ(2026-07-25〜27追加)**: 姉妹リポジトリ
  `open-raid-z`の切断耐性ジャーナル(`open_raid_z_core::disaster_recovery`)
  を再利用し、アップロードされたサイトファイルの実書き込みを
  `DisasterRecoveryManager::protect_write`経由で保護する(電源断・
  ディスク切断時もジャーナルに記録済みのため、再起動時に自動リプレイで
  復旧できる)。登録済みVPS同期先(SFTP)へのファイル複製、Email/
  Googleドライブへのオフサイト退避先設定にも対応(`/admin/dist-sync/*`
  管理API)。正直な開示: 実SMTPサーバー/実Googleドライブアカウントでの
  E2E確認は未実施(ローカルモックのみで検証)、詳細は`CLAUDE.md`参照。
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
- **簡単ドメイン設定ウィザード(無料DDNS/DuckDNS、最大20ドメイン対応、
  2026-07-23新設)**: 固定IPを持たないDDNS環境向けに、`open-web-server`
  側の新設管理APIを1画面のウィザードから呼び出す。(a) DuckDNS
  (duckdns.org)アカウント作成への外部リンク案内、(b) 登録済みドメイン
  一覧(残り枠・個別削除ボタン付き)、(c) サブドメイン名+トークン入力→
  「追加&疎通確認」、(d) 成功後にSFTP接続コマンド例を表示(複数ドメイン
  登録時はどれを使うか選択可能)。1インスタンスにつき最大20ドメインまで
  登録・自動更新できる。**正直な開示**: DuckDNSアカウント自体の取得
  (OAuthログイン)は自動化しない。呼び出し先の`open-web-server`が別
  オリジンの場合、`open-web-server`側で`OPEN_WEB_SERVER_CORS_ALLOWED_
  ORIGINS`(2026-07-23新設)を設定すれば呼び出せるようになる(詳細は
  [PORTING.md](PORTING.md)参照)。

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

> ⚠️ **ビルド時の注意(ネットワークドライブ環境)**: リポジトリがSMB等の
> ネットワーク共有ドライブ上にある場合、`cargo build`の出力(`target/`)や
> `wasm-bindgen`の入出力をそのドライブ上で直接読み書きすると、書き込み
> 直後の読み取りが古い内容を返す(読み取りキャッシュの不整合)ことがある
> (2026-07-20に実際に発生・確認済み)。再ビルドしても変更が反映されない
> 場合は、`cargo build --target-dir <ローカルドライブの一時ディレクトリ>`
> でビルド出力先をネットワークドライブ外(ローカルのC:等)に切り替え、
> `wasm-bindgen`もそのローカルコピーに対して実行すると解消する。

## サーバー側(open-easy-web-server)のインストール(2026-07-23追加)

上記はWASMフロントエンド(`pkg/`+`index.html`)のビルド手順。バックエンド
API(`server/`配下、フォルダー作成・アップロード・AI自動PHP判定・
nginx+PHP-FPM自動構成を担う`open-easy-web-server`バイナリ)は別途、
`install.sh`(Linux、systemdサービス登録)・`install.ps1`(Windows、
サービス登録案内)・`.github/workflows/release.yml`(タグpush時に
Linux x86_64・Windows x86_64向けバイナリを自動ビルドし
[GitHub Releases](https://github.com/aon-co-jp/open-easy-web/releases)へ
添付)を用意した。**正直な開示**: `open-easy-web-server`は固定アカウント
制の認証を持ち、環境変数`OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL`が未設定だと
起動時にpanicする(誰もログインできない状態でサイレントに動き続ける
より起動失敗のほうが安全という設計判断)。この配布物にWASM
フロントエンドは含まれない(上記のビルド手順で別途生成すること)。

```
curl -fsSL https://github.com/aon-co-jp/open-easy-web/releases/latest/download/open-easy-web-server-linux-x86_64.tar.gz | tar xz
sudo ./install.sh
```

### Windows向けインストーラー(2026-08-19新設、2026-08-20 サービス登録方式に一本化)

`installer/open-easy-web-install.iss`(Inno Setup)は、既存の
`install.ps1`/`uninstall.ps1`(Windowsサービス`OpenEasyWeb`として登録)を
そのまま呼び出す薄いラッパー。open-easy-webは複数の関連リポジトリを
統括する中央サービスであるため、**常時稼働するWindowsサービスとして
登録される方式のみを正式採用**している(手動起動する単体プロセス方式は
不適切と判断し廃止した)。ビルド方法・詳細はスクリプト冒頭のコメント、
および`CLAUDE.md`のHANDOFF(2026-08-20)を参照。

### macOS向けインストール(2026-08-06追加)

`install-macos.sh`(launchd用plistを`~/Library/LaunchAgents/`へ配置)を
用意した。**正直な開示**: この開発環境はWindows機のため、実際のmacOS
環境でのビルド・`launchctl`実行・動作確認は一切行っていない
(シェル構文検証・plistのXML構文検証のみ)。Apple Silicon
(`aarch64-apple-darwin`)/Intel(`x86_64-apple-darwin`)向けのビルド自体は
Appleのプロプライエタリなツールチェーン(Xcode Command Line Tools)が
必要でWindows上ではクロスコンパイルできないため、GitHub Actions
(`macos-latest`ランナー)側でビルドする方針とした
(`.github/workflows/release.yml`の`build-macos`ジョブ、次回タグpush時に
実際に動作するか要確認)。

```
curl -fsSL https://github.com/aon-co-jp/open-easy-web/releases/latest/download/open-easy-web-server-macos-x86_64.tar.gz | tar xz
./install-macos.sh
# plist内のOPEN_EASYWEB_FIXED_ACCOUNT_EMAIL等を編集後:
launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/jp.co.aon.open-easy-web.plist
```

対になる`uninstall-macos.sh`も用意した(停止・plist削除、ユーザーデータは
削除しない)。

### インストール/アンインストール時のデータ移行(2026-07-29追加)

`install.sh`/`install.ps1`は、実行時に「既存の関連DATAを取り込むか」を
尋ねます。取り込む場合はローカルのtar.gz・GitHubリポジトリ・rclone
(Googleドライブ等)のいずれかから復元できます。対になる
`uninstall.sh`/`uninstall.ps1`(新設)は、アンインストール前に同じ
3方式でデータを退避するか尋ねます。実体は`scripts/data-portability.sh`
(`.ps1`)——GoogleドライブそのものへのOAuth認証は本ソフトウェアから
代行しません(`rclone config`で事前にご自身が設定したリモートを使う
方式です)。GitHubリポジトリの公開/非公開は、リポジトリ作成時にご自身で
選んだ設定に従います。

## IPアドレスから起動する

```bash
scripts/serve.sh 0.0.0.0 8080        # 全インターフェースで待受
scripts/serve.sh 192.168.1.50 8080   # 特定のIPアドレスのみで待受
```

## ローカルモード: Windows PC上の簡易リバースプロキシ + DuckDNS(2026-09-07追加)

**English**: This is a NEW, opt-in local mode for running
`open-easy-web-server` on your own Windows/Linux/macOS PC as a simple
plain-HTTP reverse proxy in front of another local app — for example
[open-english](https://github.com/aon-co-jp/open-english)'s local server
(`127.0.0.1:4601` by default). It is a separate code path from the
existing VPS-oriented nginx/PHP-FPM feature described above and does not
touch it. **Honest limitations**: this proxy does NOT terminate TLS —
plain HTTP only. It does NOT default to port 80/443 (binding those on
Windows normally needs admin rights / URL ACL reservations, which this
tool never attempts silently) — the default listen address is
`127.0.0.1:8090`. DuckDNS support only points a domain name at your
current IP address; it does NOT open router ports or set up port
forwarding for you, and it does NOT give you HTTPS.

**日本語**: これはWindows/Linux/macOSの自分のPC上で
`open-easy-web-server`を、別のローカルアプリ(例:
[open-english](https://github.com/aon-co-jp/open-english)のローカル
サーバー、既定`127.0.0.1:4601`)の手前に立つ**単純な平文HTTPリバース
プロキシ**として動かす、新規のopt-in機能です。上記のVPS向けnginx/
PHP-FPM機能とは完全に別の経路で、そちらには一切手を加えていません。
**正直な限界**: このプロキシはTLS終端を一切行いません(平文HTTPのみ)。
80/443番ポートへは既定でbindしません(Windowsで特権ポートへのbindは
通常管理者権限/URL ACL予約を要し、それを黙って試みることはありません)
——既定の待受アドレスは`127.0.0.1:8090`です。DuckDNS対応はドメイン名を
現在のIPアドレスへ結びつけるだけで、ルーターのポート開放・ポート
フォワーディングは行いません。HTTPS化も行いません。

### 起動方法 / How to run

```powershell
$env:OPEN_EASY_WEB_LOCAL_MODE = "1"
$env:OPEN_EASY_WEB_LOCAL_BIND = "127.0.0.1:8090"       # 既定値(省略可)
$env:OPEN_EASY_WEB_LOCAL_BACKEND = "127.0.0.1:4601"    # 既定値、open-englishのローカルサーバー
.\open-easy-web-server.exe
```

```bash
OPEN_EASY_WEB_LOCAL_MODE=1 \
OPEN_EASY_WEB_LOCAL_BIND=127.0.0.1:8090 \
OPEN_EASY_WEB_LOCAL_BACKEND=127.0.0.1:4601 \
./open-easy-web-server
```

起動後、`http://127.0.0.1:8090/`(または`OPEN_EASY_WEB_LOCAL_BIND`で
指定したアドレス)へのアクセスが、そのまま`OPEN_EASY_WEB_LOCAL_BACKEND`
(既定`127.0.0.1:4601`)へ転送されます。バックエンドが起動していない
場合はクラッシュせず`502 Bad Gateway`を返します。

### 無料のDuckDNSサブドメインを取得する / Getting a free DuckDNS subdomain

1. https://www.duckdns.org/ へアクセスし、GitHub/Google等のアカウントで
   サインインします(**アカウント作成自体はこのツールから自動化しません**
   ——ユーザー自身がブラウザで行ってください)。
2. 好きなサブドメイン名(例: `myopenenglish`)を登録すると
   `myopenenglish.duckdns.org`が使えるようになり、ページ上に表示される
   **token**(長い英数字の文字列)を控えます。
3. このサーバーの`POST /v1/duckdns/update`エンドポイントへ、
   `{"domain": "myopenenglish", "token": "<あなたのtoken>"}`を送ると、
   DuckDNS側のIPアドレスがこのPCの現在のグローバルIPへ更新されます
   (トークンはディスクへ保存されません、リクエストのたびに渡してください)。
   ```bash
   curl -X POST http://127.0.0.1:8090/v1/duckdns/update \
     -H "Content-Type: application/json" \
     -d '{"domain":"myopenenglish","token":"YOUR-TOKEN-HERE"}'
   ```
4. **これだけでは外部からアクセスできません**——ご自身のルーターで、
   このPCの待受ポート(`OPEN_EASY_WEB_LOCAL_BIND`で指定したポート)への
   ポートフォワーディングを別途設定する必要があります。

### 既に持っている有料ドメインを使う / Using a domain you already own

DuckDNSの代わりに、購入済みの独自ドメインを使うこともできます。
ドメインのDNS管理画面で、Aレコードをご自宅ルーターの**グローバル
(WAN側)IPアドレス**へ向け、ルーターでこのPCの待受ポートへポート
フォワーディングを設定してください。DuckDNSと同様、これはIPアドレスの
紐付けとポート到達性の話であり、HTTPS化は別途必要です。

### 正直な開示・TLS/HTTPSは未対応 / Honest limitation: no TLS/HTTPS

このローカルモードのリバースプロキシは**平文HTTPのみ**をサポートします。
HTTPSで公開したい場合は、ご自身で証明書を用意し、別のTLS終端
(例: `stunnel`・IIS・別のリバースプロキシ)をこの手前に置く必要が
あります——このパスでは意図的にACME/Let's Encrypt自動取得は実装して
いません(既存のVPS向け機能`server/src/tls.rs`はサーバー証明書として
別の設計であり、このローカルモードとは無関係です)。

## Android版: root化端末での外付けHDD対応 + ダウンロード導線(2026-08-04追加)

`android/`に「外付けHDDをストレージに使う(root)」機能を追加(root化
済み端末専用、`su`到達性を確認できない場合は起動を拒否——内部
ストレージへの黙示的フォールバックはしない)。トップページ
(`https://easy-web.tokyo/`)の「Completed Projects」に、open-easy-web
自身の本番/デモ(`/demo`)/ダウンロード(Windows・Linux・Android APK、
[GitHub Releases](https://github.com/aon-co-jp/open-easy-web/releases/latest))
リンクを追加。詳細は[PORTING.md §15](PORTING.md#15-android版の外付けhdd対応--リリースワークフローのsibling依存漏れ2026-08-04)参照。

## ロリポップ!向け時間指定オートクロール(cron相当、2026-10-08追加)

指定した時刻・曜日に、ロリポップ!上のPHP等のURLを自動でHTTP呼び出しする機能です。
ロリポップ側のcron機能やプランに依存せず、`open-easy-web-server`が内蔵スケジューラで実行します。

- 設定: ログイン後のメインページ「ロリポップ!時間指定オートクロール」(GUI)、または管理API
  `/admin/lolipop-cron/jobs`(`x-admin-token`認証)。ジョブ名・URL・GET/POST・実行時刻(`03:00, 15:30`のように複数可)・曜日・タイムアウトを指定
- 時刻はJST基準(環境変数`OPEN_EASYWEB_LOLIPOP_TZ_OFFSET_HOURS`で変更)。設定は`OPEN_EASYWEB_LOLIPOP_CRON_FILE`へ保存され再起動後も保持
- **後追い実行**: サーバー停止中に過ぎた時刻は、起動後に1回だけ実行(既定ON、ジョブごとに無効化可、直近24時間以内の取りこぼしのみ)
- 登録内容を、ロリポップ管理画面のcron設定へ貼れる「分 時 日 月 曜日」書式でも確認可能

**正直な開示**: 実際のロリポップ!サーバーへの呼び出しは未検証(ローカルのモック先のみで検証)。
スケジューラはサーバー起動中のみ動作します。

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

> ⚠️ **デプロイ先ソースツリーの乖離に注意(2026-07-28発見・教訓)**:
> VPS上のデプロイ先ディレクトリが、実際に`aon-co-jp/open-easy-web`の
> `git clone`そのものであることを必ず確認すること。過去に別の
> メタリポジトリ(`aon-co-jp/RUNO`)のチェックアウト配下へ、未コミット
> のまま手動でファイルを配置してしまい、GitHub側をいくら更新しても
> 本番に一切反映されない状態が長期間続いていた実例がある(フロント
> エンド・バックエンドの両方で発生)。`ls <デプロイ先>/src`と
> ローカルの`src/`・`server/src/`を突き合わせ、モジュール数が一致する
> ことを確認してから完了と報告すること。詳細な再発防止手順は
> `PORTING.md`を参照。

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
