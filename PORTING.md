# PORTING.md — open-easy-web お引越しファイル

> **2026-07-25 更新**: 開発方針ファイル(`CLAUDE.md`)の見出しを
> 「設計思想＆開発方針＆開発環境ルール」へ改名しました
> (設計思想・開発方針・開発環境ルールを明確に区別)。移設先でも
> `CLAUDE.md`の内容を必ず確認してください。


> このファイル1枚で、他プロジェクトへ `open-easy-web` を導入・移設できます。
> 対象バージョン: 0.1.0(2026-07-13、`aruaru-web` からの分離初版。
> 2026-07-20、デプロイ先既定パス変更・ネットワークドライブ移設時の
> 注意事項を追記)。

## -1. VPS上のデプロイ先ソースツリーがgit repoと乖離する罠(2026-07-28発見・解消)

**移設・再デプロイのたびに必ず確認すること**: VPSの`OPEN_EASYWEB_STATIC_DIR`
が指すディレクトリが、実際に`aon-co-jp/open-easy-web`の`git clone`
そのものであることを確認する。2026-07-28時点で発見した実例:
`/root/RUNO/open-easy-web/open-easy-web-wasm`は`aon-co-jp/RUNO`
(別の、エコシステム全体のメタ索引リポジトリ)のチェックアウトであり、
ソースファイル(`shell.rs`等6ファイルのみ)は`aon-co-jp/open-easy-web`
本体(15以上のモジュール)とは全くの別物・古いスナップショットだった。
このため、GitHub側の`src/shell.rs`をいくら修正・pushしても、VPS上の
WASMビルドには一切反映されない状態が続いていた(RS-Sync/open-redmineの
リンクURL更新が本番に反映されない、という形で発覚)。
**解消方法**: `/root/open-easy-web-app`に`aon-co-jp/open-easy-web`を
クリーンclone→`cargo build --target wasm32-unknown-unknown --release`→
`wasm-bindgen`→`index.html`+`pkg/`を`static/`にまとめ→
`open-easy-web.service`の`OPEN_EASYWEB_STATIC_DIR`をこの新しい
`static/`ディレクトリへ向けて`systemctl restart`。`curl`で取得した
`.wasm`バイナリに新しい文字列(更新後のURL等)が実際に含まれることを
`grep`で確認してから完了とすること(型チェック・ビルド成功のみでは
不十分、という既存方針の徹底)。

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
├── src/{lib,dom,profiles,shell,api_auth,api_upload,auth_ui,view_bridge,
│        api_free_domain,free_domain_ui,setup_wizard_ui,api_dist_sync}.rs
│                              # site management + auth + upload WASM UI
│                              # (api_free_domain/free_domain_ui: 2026-07-23
│                              #  簡単ドメイン設定ウィザード、無料DDNS/DuckDNS)
│                              # (setup_wizard_ui/api_dist_sync: 2026-07-25
│                              #  分散同期クローンDB+ディザスタリカバリ設定
│                              #  ステップ〈初回セットアップガイド Step 5〉)
├── server/                   # 別クレート(open-easy-web-server、tokio/hyper直接実装)
│   ├── Cargo.toml / Cargo.lock  # 2026-07-25: open_raid_z_core をpath依存
│   │                            # (default-features=false, offsite_backup)
│   └── src/{main,auth,users,totp,mail,sms,tls,vhost,php_detector,upload,
│             appserver_registration,dist_sync}.rs
│             # dist_sync.rs(2026-07-25新設): 分散同期先(VPS、SFTP経由)+
│             # ディザスタ用退避先(Email/Googleドライブ)の登録・一覧・削除
│             # 管理API。open-raid-zのjournal/disaster_recovery/
│             # offsite_backup/accelをそのまま再利用(再実装しない)。
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

## 11. 簡単ドメイン設定ウィザード(無料DDNS/DuckDNS、2026-07-23新設)の
CORS注意点(2026-07-23、`open-web-server`側で解消済み)

`src/api_free_domain.rs`は`open-web-server`側の管理API
(`POST /admin/ddns/setup-free-domain`・`GET /admin/sftp/
connection-info`)を呼び出すが、`api_auth.rs`(自サーバーAPI、
`RequestMode::SameOrigin`)と異なり、呼び出し先はユーザーが入力する
**別オリジンの`open-web-server`インスタンス**であるため
`RequestMode::Cors`を使っている。`open-web-server`側がCORSレスポンス
ヘッダ(`Access-Control-Allow-Origin`等)を返さない場合、ブラウザ側で
`fetch`がブロックされる。

**2026-07-23、この制約は`open-web-server`側にCORS対応
(`middleware/cors.rs`)を追加したことで解消した**——移植先で別オリジン
構成(`open-easy-web`と`open-web-server`を別ポート/別ホストで運用する
場合)を使う際は、`open-web-server`起動時に
`OPEN_WEB_SERVER_CORS_ALLOWED_ORIGINS`(このウィザードを配信する
オリジンをカンマ区切りで指定、例: `http://localhost:8080`)を設定する
だけでよい。このリポジトリ(`open-easy-web`)側のコード変更は不要
(`RequestMode::Cors`は既存のまま、ブラウザの標準CORSプロトコルに
従うだけ)。同一オリジン構成(reverse proxy経由でパスを揃える等)を
使う場合は引き続きCORS設定自体が不要。詳細は`open-web-server`側の
`PORTING.md`§4.10・`CLAUDE.md`の同日HANDOFFを参照。

## 12. スマホ対応・英語(日本語)ハイブリッド表示(2026-07-24)

`index.html`は`@media (max-width: 600px)`でスマホ縦画面向けに単一
カラム化・タップ操作向けサイズ(`min-height: 44px`)を適用している。
`src/shell.rs`のUIラベルは「英語表記の直後に(日本語)を括弧書き」形式
(例: `Save (保存)`)を採用中(見出し・ボタン・フォームラベルが対象、
段階適用中——長い説明文・エラーメッセージは対象外)。移植先でこの
パターンを踏襲する場合は、`src/shell.rs`内の既存表記を検索リファレンス
として利用すること。

## 13. 分散同期クローンDB + ディザスタリカバリの`open_raid_z_core`再利用
(2026-07-25新設)

`server/src/dist_sync.rs`は、他VPSへの分散同期クローンDB・ネット切断/
非常時のメール/Googleドライブ自動退避・CPU圧縮アクセラレーションを、
姉妹リポジトリ`open-raid-z`の`open_raid_z_core`(`journal`/
`disaster_recovery`/`offsite_backup`/`accel`)をpath依存として再利用する
ことで実装した。移植先で同種の機能が必要な場合の要点:

- **Cargo依存**: `server/Cargo.toml`に
  `open_raid_z_core = { path = "../../open-raid-z/open_runo_zfs_source/
  open_raid_z_core", default-features = false, features =
  ["offsite_backup"] }` のように`default-features = false`を必ず指定する
  こと——既定featureには`winfsp_backend`/`gpu_accel`が含まれており、
  WinFsp SDK・dxc・Windows SDKを要求してしまう(CPUフォールバックのみで
  よい場合は不要な依存)。`aruaru-dist`(`aruaru-db`側)が先行して踏襲
  済みの同じパターン。
- **`DisasterRecoveryConfig`が既に「VPS同期先」も表現できる**:
  `open_raid_z_core::offsite_backup::SftpBackupTarget`は「VPSへの分散
  同期先」も「SFTPオフサイト退避」も同じ抽象で表現できるため、
  `dist_sync.rs`はVPS同期先を全て`SftpBackupTargetConfig`へマッピング
  している(独自のレプリケーションプロトコルを新設していない)。
- **管理APIの認証方式は移植先の既存方針に合わせる**: このリポジトリの
  `/api/*`系はBearerセッショントークン(`require_session`)が既定だが、
  `dist_sync.rs`の`/admin/dist-sync/*`は`appserver_registration.rs`が
  外部サービスへ送信する際と同じ`x-admin-token`ヘッダ方式を採用した
  (環境変数`OPEN_EASYWEB_DIST_SYNC_ADMIN_TOKEN`未設定時は無効化する
  安全側デフォルト)——両方式が同一サーバー内に混在する設計になって
  いる点に注意(意図的な選択、CLAUDE.md HANDOFF参照)。
- **テスト方針**: `open-raid-z`側の`tests/offsite_backup_integration.rs`
  と同じ「実クラウド/実SMTP/実VPSには一切接続しない」方針を踏襲。
  到達不能なホスト/ポート(`127.0.0.1:1`等)を使い、`ensure_ready`が
  失敗してもpanicせず「スキップ」として正直に報告することを確認する
  形のユニットテストのみで検証している。

### 13a. 実サイトファイル書き込み経路への複製配線(2026-07-25続き)

上記13節は「登録・設定・疎通確認」の土台のみで、実際のサイトファイル
書き込みは複製されない、という既知のギャップがあった。その後の同日中に
`server/src/main.rs`の`upload_files`(`POST /api/sites/:name/upload`)
ハンドラへ実配線した。移植先で同様の「実データ書き込み→登録済み同期先へ
複製」を行いたい場合の要点:

- **配線ポイントの選び方**: 複数の書き込み経路がある場合、全てを一度に
  配線しようとせず、「ユーザーが実際にデータを書き込む、最も自然で
  境界が明確な1箇所」に絞ること(このリポジトリでは`upload_files`の
  `tokio::fs::write`直後、ディレクトリ作成のみのエンドポイントや
  インフラ設定ファイル生成は対象外とした)。
- **非ブロッキングの実装パターン**(`dist_sync.rs`の
  `spawn_replication`/`replicate_written_file`参照):
  1. 登録済み同期先が0件なら`tokio::spawn`すら行わず即座に戻る
     (`has_sync_targets()`で判定)——未設定時に一切のオーバーヘッドを
     持ち込まない、後方互換性を保証する最重要ポイント。
  2. `SftpBackupTarget::upload_segment`等のブロッキングI/O呼び出しは
     `tokio::task::spawn_blocking`へ退避してから呼ぶ(非同期ランタイムの
     ワーカースレッドを塞がないため)。
  3. 呼び出し元(HTTPハンドラ)は複製処理の完了を待たない
     (`tokio::spawn`でデタッチ)——ユーザーへのレスポンスは複製の
     成否・速度に一切影響されない。個々の同期先の失敗は他の同期先への
     複製やレスポンス自体をブロックしない(ログのみ)。
- **テスト方針(実複製の検証)**: `open_raid_z_core`側
  `tests/offsite_backup_integration.rs`のインプロセス`russh`/
  `russh-sftp`モックサーバーを、複製元クレート(`server/`)側の
  テストモジュールへそのまま移植・再利用できる(コピー&微調整のみ、
  再実装は不要)。移植する場合、`Cargo.toml`の`[dev-dependencies]`に
  `russh`/`russh-sftp`を`open_raid_z_core`と同一バージョン・featureで
  追加すること。**罠**: 主依存で`rand`の別バージョン(例: `0.8`)を
  既に使っている場合、russh 0.62が要求する`rand 0.10`系と名前が衝突し
  `cargo test`が`error[E0464]: multiple candidates for rlib dependency
  rand`で失敗する——`{ package = "rand", version = "0.10" }`で別名
  リネームして依存させることで回避できる(このリポジトリでは
  `rand_for_test_keys`という名前でリネーム)。

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
