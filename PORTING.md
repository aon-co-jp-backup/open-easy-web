# PORTING.md — open-easy-web お引越しファイル

> 🎯 **移植時の前提(2026-08-29)**: aruaru-dbはRPoemとのSETで初めて
> 「REST API不要・Cosmo有料版互換」の価値が成立する(正本:
> aruaru-db/CLAUDE.md冒頭)。本リポジトリの管理APIをGraphQLへ移行する
> 場合も、必ずこのSET連携の価値強化になるか確認すること。

> **2026-07-25 更新**: 開発方針ファイル(`CLAUDE.md`)の見出しを
> 「設計思想＆開発方針＆開発環境ルール」へ改名しました
> (設計思想・開発方針・開発環境ルールを明確に区別)。移設先でも
> `CLAUDE.md`の内容を必ず確認してください。


> セルフホスト時のアカウント設定(自分のe-mail・電話番号の登録方法)・
> 2FA(フィーチャーフォンでの確認方法含む)は [`manual-JAPAN.md`](manual-JAPAN.md)
> を参照してください。

> このファイル1枚で、他プロジェクトへ `open-easy-web` を導入・移設できます。
> 対象バージョン: 0.1.0(2026-07-13、`aruaru-web` からの分離初版。
> 2026-07-20、デプロイ先既定パス変更・ネットワークドライブ移設時の
> 注意事項を追記)。

## -2. 最近の更新(2026-08-07) / Recent update

Androidタブレット向けレイアウト(`layout-sw600dp/activity_main.xml`)の
`maxWidth`が`layout_width="match_parent"`と組み合わさって機能していない
実バグを修正(`wrap_content`へ変更)。移設先でも同じパターン
(`maxWidth`+`match_parent`)が他の`layout-sw600dp`ファイルに残っていないか
確認すること。

*English*: Fixed a real bug where `maxWidth` on the tablet layout
(`layout-sw600dp/activity_main.xml`) had no effect because it was combined
with `layout_width="match_parent"` (changed to `wrap_content`). When
porting, check other `layout-sw600dp` files for the same pattern.

## -1. VPS上のデプロイ先ソースツリーがgit repoと乖離する罠(2026-07-28発見・解消、フロント/バックエンド両方で発生)

**フロントエンドだけでなくバックエンド(`open-easy-web-server`)も同様に
乖離しうる**——2026-07-28、`/root/RUNO/open-easy-web/open-easy-web-server`
も同じ`aon-co-jp/RUNO`チェックアウトで、`auto_update.rs`・`dist_sync.rs`の
2モジュールが丸ごと欠落していた実例あり。バイナリ名が
`open-easyweb-server`(旧)→`open-easy-web-server`(新、ハイフン位置が
違う)へ変わっている点も切り替え時の見落としポイント。切り替え前に
必ず`ls <VPSのserver/src>`と`ls <ローカルのserver/src>`を突き合わせて
モジュール数が一致するか確認すること。

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
│             appserver_registration,dist_sync,system_memory,system_disk,
│             power_profile,db_encryption,auto_update}.rs
│             # dist_sync.rs(2026-07-25新設): 分散同期先(VPS、SFTP経由)+
│             # ディザスタ用退避先(Email/Googleドライブ)の登録・一覧・削除
│             # 管理API。open-raid-zのjournal/disaster_recovery/
│             # offsite_backup/accelをそのまま再利用(再実装しない)。
│             # system_memory.rs(2026-07-31新設)/system_disk.rs
│             # (2026-08-04新設): `sysinfo`クレートで実メモリ・仮想メモリ
│             # (スワップ)・実HDD/SSD使用状況を取得、GUIの円グラフ表示用に
│             # `/admin/system/memory`・`/admin/system/disk`として公開
│             # (同じ`x-admin-token`認証)。
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

## 14. インストール/アンインストール時のデータ移行(2026-07-29新設)

`scripts/data-portability.sh`(`.ps1`)——ローカルtar.gz・GitHub
リポジトリ・rclone(Googleドライブ等)の3方式でバックアップ/リストア
できる小さなスクリプト。`install.sh`/`install.ps1`が起動時に「既存の
関連DATAを取り込むか」を尋ね、対になる`uninstall.sh`/`uninstall.ps1`が
削除前に同じ3方式で退避するかを尋ねる。移植時の要点:

- バックアップ対象は`OPEN_EASYWEB_USERS_STATE`/`OPEN_EASYWEB_AI_STATE`/
  `OPEN_EASYWEB_AUTO_UPDATE_SETTINGS_FILE`(いずれも既定パスは
  `/var/www/.open-easy-web-*.json`、Windowsは
  `C:\ProgramData\open-easy-web\...`)+`dist_sync`のジャーナルディレクトリ。
  アップロード済みサイトファイル自体(`OPEN_EASYWEB_SITES_ROOT`)は
  既定で対象外(巨大になりうるため、`--include-sites-root`明示時のみ)。
- **Googleドライブ等へのOAuth認証はこのスクリプトから代行しない**
  (このエコシステム共通方針)——`rclone config`でユーザー自身が事前に
  設定したリモートを使う。GitHubの公開/非公開はリポジトリ作成時に
  ユーザー自身が選んだ設定に従うのみ。
- **罠(2026-07-29発見)**: 新規作成した`.ps1`を
  `[System.Management.Automation.Language.Parser]::ParseFile()`で構文
  検証すると`MissingEndCurlyBrace`という誤検知が出ることがある(実際の
  ファイル内容に構文エラーは無い、`ParseFile`のエンコーディング解釈に
  起因)。`[System.IO.File]::ReadAllText()`で読んだ文字列を
  `ParseInput()`へ渡す形で検証すると正しく判定できる。

## 15. Android版の外付けHDD対応 + リリースワークフローのsibling依存漏れ(2026-08-04)

`open-web-server/android`版の外付けHDD主ストレージ機能
(`ExternalStorageConfig.kt`)をそのまま移植。移植先固有の環境変数は
`OPEN_EASYWEB_SITES_ROOT`/`OPEN_EASYWEB_USERS_STATE`/
`OPEN_EASYWEB_DB_ENCRYPTION_KEY_FILE`/`OPEN_EASYWEB_AI_STATE`
(`server/src/main.rs::AppState::from_env()`が実際に読む値、`env_path()`
呼び出し箇所を直接確認して裏取りすること——推測で環境変数名を書かない)。

**実装中に発見した実バグ2件**:
1. Android版が必須環境変数`OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL`
   (`fixed_account_email()`が未設定だと`panic`する設計)を一切設定して
   おらず、外付けHDD機能とは無関係に**サーバーが一度も起動できない
   状態**だった。`FixedAccountConfig.kt`(設定ダイアログ)を追加し、
   未設定なら起動を明確に拒否するようにして解消。
2. `.github/workflows/release.yml`が`server/Cargo.toml`の
   `open_raid_z_core`への無条件path依存(2026-07-25追加、`../../
   open-raid-z/...`)をcheckoutしておらず、**次にタグをpushすると
   ビルドが失敗する**状態だった。全ジョブに`open-raid-z`のsibling
   checckoutを追加して解消。移植先でも、他リポジトリへのpath依存を
   後から追加した際は必ずCIワークフロー側のcheckout漏れが無いかを
   確認すること(この種の依存追加は往々にしてCI側の追従が漏れる)。

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

## ロリポップ!向け時間指定オートクロール(cron相当、2026-10-08)と後追い実行・設定画面
指定した時刻・曜日にURLを自動でHTTP呼び出しする内蔵スケジューラ(`server/src/lolipop_cron.rs`)。管理API `/admin/lolipop-cron/jobs`、設定画面(WASM)、
ロリポップ管理画面用のcron書式の書き出し、**サーバー停止中に過ぎた時刻の後追い実行**(既定ON・直近24時間・1回のみ)を実装。テスト7件合格、ローカルで再起動後の後追いも実測済み。
実ロリポップ(`www.ion.tokyo`)へのGET 1回でHTTP 200を確認。**未確認**: ロリポップ上での長期の自動発火。
**お引越し**: 設定は `OPEN_EASYWEB_LOLIPOP_CRON_FILE`(既定`/var/www/.open-easy-web-lolipop-cron.json`)のJSON1ファイル。新サーバーへはこのファイルをコピーし、環境変数`OPEN_EASYWEB_DIST_SYNC_ADMIN_TOKEN`を設定すれば移せる。

## Timed auto-crawl for Lolipop (cron-like, 2026-10-08), catch-up runs and the settings screen
A built-in scheduler (`server/src/lolipop_cron.rs`) that calls URLs over HTTP at chosen times/weekdays. Includes the admin API `/admin/lolipop-cron/jobs`, a WASM settings screen,
export in the Lolipop control-panel cron format, and **catch-up runs for slots missed while the server was down** (on by default, last 24 h, once only). 7 tests pass; catch-up after a restart was measured locally.
One GET to a real Lolipop site (`www.ion.tokyo`) returned HTTP 200. **Unverified**: long-term automatic firing on Lolipop.
**Moving servers**: the settings are one JSON file, `OPEN_EASYWEB_LOLIPOP_CRON_FILE` (default `/var/www/.open-easy-web-lolipop-cron.json`). Copy it and set `OPEN_EASYWEB_DIST_SYNC_ADMIN_TOKEN` on the new server.

## JSON処理の使用方針: `parse_secure_sonic`(速い型+安全)と`sonic-rs`は使える所では全て使う(2026-10-14、ユーザー指示)
正本は[`RS-JSON`](https://github.com/aon-co-jp/RS-JSON)(`README.md`・`PORTING.md`)。

**方針**: JSONを読む所は、使える所では**全て** `rust-json`(RS-JSON)の `parse_secure_sonic`(`fast`機能、結果は`sonic_rs::Value`)を使う。
`sonic-rs` を直接使える所(型付き構造体への読み込みなど)も同様。使えない所は `parse_secure`(結果は`serde_json::Value`)を使う。
外部から来る信頼できない入力(HTTP本文・ネットワーク越しのファイル・アップロード)は、**必ずサイズ・深さ・重複キーの上限つき**で読む。

**実測(7 MB・4万件)**: `serde_json` 200 ms / `parse_secure` 217 ms / **`parse_secure_sonic` 83 ms(約2.4倍速い、防御込み)** / `sonic-rs`そのまま 33 ms(約6倍、防御なし)。
`serde_json::Value`に読む限りSIMDでもほぼ速くならない(188 ms)。**速さの源は、結果を連続した領域に詰め込む値の型**。

**使える所 / 使えない所**
- ✅ 読み取り中心で、JSONを大きく・頻繁に読む所(キャッシュJSON、設定、APIの応答、ログ、インデックス)。
- ✅ 外部からの入力の入口(速さより防御が目的。`parse_secure`でも可)。
- ⚠️ `serde_json::Value`を関数間で受け渡している既存コードは、型を`sonic_rs::Value`に直す書き換えが要る(`JsonValueTrait`/`JsonContainerTrait`のメソッド)。書き換えずに`serde_json::Value`へ読むと速くならない。
- ❌ WASM(`rust_json::light`、依存ゼロ)、PHP/JavaScript側のJSON処理(Rustの部品は使えない)。
- 注意: 依存先の`sonic-rs`は内部で`unsafe`を使う(自前コードに`unsafe`は無い)。小さなJSON(数KB)では体感差が出ない。速度目的の書き換えは、実測で効く所から。

**導入**: `Cargo.toml`に `rust-json = { path = "../RS-JSON", features = ["fast"] }`。使用例は`RS-JSON/README.md`。

**このリポジトリでの適用状況**: 未適用(方針のみ)。コードの切り替えは、JSONを大きく・頻繁に読む所から実測して順に行う。

## JSON handling policy: use `parse_secure_sonic` (fast type + secure) and `sonic-rs` everywhere they can be used (2026-10-14, user instruction)
The source of truth is [`RS-JSON`](https://github.com/aon-co-jp/RS-JSON) (`README.md`, `PORTING.md`).

**Policy**: wherever JSON is read, use `parse_secure_sonic` from `rust-json` (RS-JSON, the `fast` feature; the result is `sonic_rs::Value`) **everywhere it can be used**.
Use `sonic-rs` directly where possible too (e.g. reading into typed structs). Where it cannot be used, use `parse_secure` (result: `serde_json::Value`).
Untrusted input from outside (HTTP bodies, files over the network, uploads) **must always be read with size / depth / duplicate-key limits**.

**Measured (7 MB, 40,000 records)**: `serde_json` 200 ms / `parse_secure` 217 ms / **`parse_secure_sonic` 83 ms (about 2.4x faster, with the defences)** / raw `sonic-rs` 33 ms (about 6x, no defences).
Reading into `serde_json::Value` gives almost no speed-up even with SIMD (188 ms). **The speed comes from the value type that packs the result into contiguous memory.**

**Where it can / cannot be used**
- ✅ Read-heavy code that parses large or frequent JSON (cache JSON, config, API responses, logs, indexes).
- ✅ Entrances for outside input (the goal there is defence rather than speed; `parse_secure` is fine).
- ⚠️ Existing code that passes `serde_json::Value` between functions needs its type changed to `sonic_rs::Value` (methods from `JsonValueTrait` / `JsonContainerTrait`). Reading into `serde_json::Value` without that change is not faster.
- ❌ WASM (`rust_json::light`, zero dependencies) and the JSON handling on the PHP / JavaScript side (Rust components cannot be used there).
- Note: the `sonic-rs` dependency uses `unsafe` internally (our own code has none). For small JSON (a few KB) the difference is not noticeable; do speed-motivated rewrites where measurements show a benefit.

**Adoption**: in `Cargo.toml`, `rust-json = { path = "../RS-JSON", features = ["fast"] }`. Example in `RS-JSON/README.md`.

**Status in this repository**: Not yet applied (policy only). Code will be switched over place by place, starting where measurements show large or frequent JSON reads.

## API 方針: REST 全廃・gRPC / Connect へ (2026-11-04)

### 日本語
- **REST は全廃する。新しい API は REST で作らない。** ブラウザや `curl` から呼ぶ API は **Connect の単純呼び出し**(`POST /<パッケージ>.<Service>/<Method>`、`content-type: application/json`、本文は JSON)、サービス間は **gRPC / gRPC-web**(RPoem の管理API `open_runo.admin.v1.Admin` と同じ流儀)。ストリーミングは、必要になってから。
- **既存の REST は、触るときに順に移す。** 移したら旧 REST の経路は消す。移行が終わるまで残るが、**新規には追加しない**。
- **JSON は RS-JSON に統一**する(読み込みは `from_str_secure` / `parse_secure` でサイズ・深さ・重複キーの上限つき、書き出しは `rust_json::fast`)。ブラウザの WASM に `prost` は入れない(サイズが増えるため)。
- **例外(自分たちの API ではなく、相手の規格が決めているもの)**: ACME、OAuth/OIDC のコールバック、第三者(GitHub など)からの Webhook、ヘルスチェック、静的ファイル配信、OTLP など。これらは規格どおりの HTTP を受ける。
- 参考実装: `RPoem`(`open_runo.admin.v1.Admin`)、`open-easy-web`(`server/src/connect.rs`、`proto/open_easy_web/admin/v1/admin.proto`)。
- ※ このリポジトリのコードが既に方針どおりかどうかは、各リポジトリの現状による。この節は方針の記録であり、コードの変更を意味しない。

### English
- **REST is being abolished. New APIs are not built as REST.** APIs called from browsers or `curl` use **Connect unary calls** (`POST /<package>.<Service>/<Method>`, `content-type: application/json`, JSON body); service-to-service calls use **gRPC / gRPC-web** (the same style as RPoem's admin API `open_runo.admin.v1.Admin`). Streaming only when it is actually needed.
- **Existing REST endpoints are migrated one by one, when they are touched.** The old REST route is removed once migrated. Until migration is finished they remain, but **no new ones are added**.
- **JSON is unified on RS-JSON**: reading with `from_str_secure` / `parse_secure` (size, depth and duplicate-key limits), writing with `rust_json::fast`. `prost` is not put into browser WASM (it would grow the binary).
- **Exceptions (shapes dictated by someone else's standard, not our own API)**: ACME, OAuth/OIDC callbacks, webhooks from third parties (e.g. GitHub), health checks, static file serving, OTLP, and so on. These accept plain HTTP as the standard requires.
- Reference implementations: `RPoem` (`open_runo.admin.v1.Admin`) and `open-easy-web` (`server/src/connect.rs`, `proto/open_easy_web/admin/v1/admin.proto`).
- Note: whether the code in a given repository already follows this policy depends on that repository. This section records the policy; it does not mean the code has been changed.

### Deutsch (KI-Übersetzung, redaktionell überarbeitet; noch nicht von Muttersprachlern geprüft)
- **REST wird vollständig abgeschafft; neue APIs entstehen nicht mehr als REST.** APIs, die aus dem Browser oder per `curl` aufgerufen werden, verwenden **unäre Connect-Aufrufe** (`POST /<Paket>.<Service>/<Methode>`, `content-type: application/json`, JSON-Body). Zwischen Diensten kommen **gRPC bzw. gRPC-web** zum Einsatz, im selben Stil wie die Admin-API von RPoem (`open_runo.admin.v1.Admin`). Streaming wird erst ergänzt, wenn es tatsächlich gebraucht wird.
- **Bestehende REST-Endpunkte werden schrittweise umgestellt, und zwar immer dann, wenn man ohnehin an ihnen arbeitet.** Sobald ein Endpunkt umgestellt ist, wird die alte REST-Route entfernt. Bis zum Abschluss der Migration bleiben die übrigen bestehen, es werden aber **keine neuen mehr hinzugefügt**.
- **JSON wird einheitlich über RS-JSON verarbeitet**: Gelesen wird mit `from_str_secure` bzw. `parse_secure` (mit Obergrenzen für Größe, Verschachtelungstiefe und doppelte Schlüssel), geschrieben mit `rust_json::fast`. `prost` wird nicht in das Browser-WASM aufgenommen, da es die Datei vergrößern würde.
- **Ausnahmen (Schnittstellen, deren Form ein fremder Standard vorgibt und die nicht zu unserer eigenen API gehören)**: ACME, OAuth/OIDC-Callbacks, Webhooks von Drittanbietern (z. B. GitHub), Health-Checks, die Auslieferung statischer Dateien, OTLP und Ähnliches. Diese nehmen, wie vom jeweiligen Standard verlangt, gewöhnliches HTTP entgegen.
- Referenzimplementierungen: `RPoem` (`open_runo.admin.v1.Admin`) und `open-easy-web` (`server/src/connect.rs`, `proto/open_easy_web/admin/v1/admin.proto`).
- Hinweis: Ob der Code eines einzelnen Repositorys diese Richtlinie bereits erfüllt, hängt vom jeweiligen Repository ab. Dieser Abschnitt dokumentiert lediglich die Richtlinie; er besagt nicht, dass der Code schon angepasst wurde.

### Русский (перевод выполнен с помощью ИИ и отредактирован; носителями языка пока не проверен)
- **От REST мы полностью отказываемся; новые API больше не создаются в стиле REST.** API, к которым обращаются из браузера или через `curl`, используют **унарные вызовы Connect** (`POST /<пакет>.<Service>/<Method>`, `content-type: application/json`, тело запроса — JSON). Для взаимодействия между сервисами применяются **gRPC и gRPC-web** — в том же стиле, что и административный API RPoem (`open_runo.admin.v1.Admin`). Потоковая передача данных добавляется лишь тогда, когда в ней действительно возникает необходимость.
- **Существующие REST-эндпоинты переводятся поэтапно — по мере того, как с ними приходится работать.** После перевода старый REST-маршрут удаляется. Пока миграция не завершена, оставшиеся эндпоинты сохраняются, но **новые не добавляются**.
- **Работа с JSON унифицирована на основе RS-JSON**: чтение — через `from_str_secure` или `parse_secure` (с ограничениями на размер, глубину вложенности и повторяющиеся ключи), запись — через `rust_json::fast`. `prost` в браузерный WASM не включается, поскольку это увеличило бы размер файла.
- **Исключения (интерфейсы, форму которых определяет чужой стандарт, а не наш собственный API)**: ACME, обратные вызовы OAuth/OIDC, вебхуки сторонних сервисов (например, GitHub), проверки работоспособности, раздача статических файлов, OTLP и тому подобное. Такие интерфейсы принимают обычный HTTP, как того требует соответствующий стандарт.
- Эталонные реализации: `RPoem` (`open_runo.admin.v1.Admin`) и `open-easy-web` (`server/src/connect.rs`, `proto/open_easy_web/admin/v1/admin.proto`).
- Примечание: соответствует ли код конкретного репозитория этой политике, зависит от самого репозитория. Данный раздел лишь фиксирует политику и не означает, что код уже изменён.

## API 方針の補足: 用語の整理と使い分け (2026-11-04)

### 日本語
**1つに統一するものではなく、別の層の話です。** 「REST の代わり」になるのは Connect・gRPC・GraphQL だけで、WASM は画面を作る技術です。

| 用語 | 何か | REST の代わりか |
|---|---|---|
| REST | URL と HTTP メソッドで操作を表す API の作り方 | 全廃する側 |
| **Connect** | `POST /サービス名/メソッド名` に JSON を送る形式。`curl` でもブラウザの `fetch` でも呼べる | はい(**採用**) |
| **gRPC** | 同じ「サービス/メソッド」の考え方を、バイナリ(protobuf)と HTTP/2 で行う形式。速く、ストリーミングも得意 | はい(**サービス同士に採用**) |
| gRPC-web | gRPC のブラウザ向けの変形版 | はい(必要なら) |
| GraphQL | 欲しい項目を問い合わせる形式 | 一部だけ(**問い合わせ(読み取り)専用**) |
| WASM | Rust をブラウザの中で動かす仕組み | いいえ(画面を作る技術) |

- **ブラウザの画面や `curl` から呼ぶ API は Connect**(POST、JSON を RS-JSON で読み書き)。ブラウザの WASM は素の gRPC を話せず、Connect なら `fetch` のままで、`prost` を足さずに済む(画面のサイズも増えない)。
- **サービス同士の通信(サーバー間、重い処理、ストリーミング)は gRPC。** 多くは Connect で足りるので、ストリーミングが必要になった所だけ gRPC に上げる。
- **GraphQL は、まとめて問い合わせたいときの読み取り専用。** 書き込み(操作)には使わない。
- **WASM は画面を作る技術として、そのまま使う。** 画面から呼ぶ API が Connect になっただけ。
- 外部の規格が形を決めるもの(ACME、OAuth のコールバック、第三者の Webhook、ヘルスチェック、静的ファイル、OTLP)は、規格どおりの HTTP のまま。
- **迷ったら Connect。**

### English
**These are not alternatives to unify; they sit at different layers.** Only Connect, gRPC and GraphQL can replace REST; WASM is a way to build the UI.

| Term | What it is | Replaces REST? |
|---|---|---|
| REST | API style that expresses operations through URLs and HTTP methods | The thing being abolished |
| **Connect** | `POST /Service/Method` with a JSON body; callable from `curl` and from a browser `fetch` | Yes (**adopted**) |
| **gRPC** | The same service/method idea over binary (protobuf) and HTTP/2; fast, good at streaming | Yes (**for service-to-service**) |
| gRPC-web | A browser-friendly variant of gRPC | Yes (if needed) |
| GraphQL | A query language: ask for exactly the fields you want | Partly (**read-only queries**) |
| WASM | Runs Rust inside the browser | No (a UI technology) |

- **APIs called from browser screens or `curl` use Connect** (POST, JSON read and written with RS-JSON). Browser WASM cannot speak plain gRPC; with Connect it stays a `fetch`, and `prost` is not needed (the binary does not grow).
- **Service-to-service calls (server to server, heavy work, streaming) use gRPC.** Connect is enough in most places; move a call to gRPC only where streaming is actually needed.
- **GraphQL is read-only**, for queries that gather several pieces of data. It is not used for writes (operations).
- **WASM stays as the way to build the UI.** The API it calls simply became Connect.
- Interfaces whose shape an outside standard dictates (ACME, OAuth callbacks, third-party webhooks, health checks, static files, OTLP) stay plain HTTP.
- **When in doubt, use Connect.**

### Deutsch (KI-Übersetzung, sprachlich überarbeitet; noch nicht von Muttersprachlern geprüft)
**Es geht nicht darum, eine einzige Technik auszuwählen – die Begriffe liegen auf unterschiedlichen Ebenen.** Als Ersatz für REST kommen nur Connect, gRPC und GraphQL in Frage; WASM dagegen ist eine Technik, mit der die Benutzeroberfläche gebaut wird.

| Begriff | Was ist das? | Ersatz für REST? |
|---|---|---|
| REST | Ein API-Stil, bei dem Operationen über URLs und HTTP-Methoden ausgedrückt werden | wird abgeschafft |
| **Connect** | `POST /Dienst/Methode` mit JSON im Body; aufrufbar mit `curl` ebenso wie mit `fetch` im Browser | ja (**gewählt**) |
| **gRPC** | Dasselbe Prinzip aus Dienst und Methode, aber über binäres protobuf und HTTP/2; schnell und gut für Streaming geeignet | ja (**zwischen Diensten**) |
| gRPC-web | Eine für den Browser angepasste Variante von gRPC | ja (bei Bedarf) |
| GraphQL | Abfragesprache, mit der man genau die benötigten Felder anfordert | teilweise (**nur für lesende Abfragen**) |
| WASM | Führt Rust-Code direkt im Browser aus | nein (Technik für die Oberfläche) |

- **APIs, die von Browseroberflächen oder per `curl` aufgerufen werden, verwenden Connect** (POST; JSON wird mit RS-JSON gelesen und geschrieben). WASM im Browser kann kein reines gRPC sprechen. Mit Connect bleibt es bei einem gewöhnlichen `fetch`, und `prost` ist nicht nötig – die WASM-Datei wird dadurch nicht größer.
- **Für die Kommunikation zwischen Diensten (Server zu Server, rechenintensive Aufgaben, Streaming) wird gRPC eingesetzt.** In den meisten Fällen reicht Connect aus; auf gRPC wird nur dort umgestellt, wo Streaming tatsächlich gebraucht wird.
- **GraphQL dient ausschließlich lesenden Abfragen**, wenn mehrere Informationen gebündelt abgefragt werden sollen. Für Schreibzugriffe (Operationen) wird es nicht verwendet.
- **WASM bleibt die Technik für die Oberfläche.** Geändert hat sich lediglich, dass die aufgerufene API nun auf Connect beruht.
- Schnittstellen, deren Form durch einen externen Standard festgelegt ist (ACME, OAuth-Callbacks, Webhooks von Drittanbietern, Health-Checks, statische Dateien, OTLP), bleiben gewöhnliches HTTP.
- **Im Zweifel: Connect.**

### Русский (перевод выполнен с помощью ИИ и литературно отредактирован; носителями языка пока не проверен)
**Это не варианты, из которых нужно выбрать один, а технологии разных уровней.** Заменить REST могут только Connect, gRPC и GraphQL; WASM же — это способ создания пользовательского интерфейса.

| Термин | Что это | Заменяет REST? |
|---|---|---|
| REST | Стиль построения API, при котором операции выражаются через URL и HTTP-методы | отменяется |
| **Connect** | `POST /Сервис/Метод` с телом в формате JSON; вызывается как из `curl`, так и из браузерного `fetch` | да (**выбран**) |
| **gRPC** | Тот же принцип «сервис — метод», но поверх двоичного формата protobuf и HTTP/2; работает быстро и хорошо подходит для потоковой передачи | да (**для связи между сервисами**) |
| gRPC-web | Адаптированный для браузера вариант gRPC | да (при необходимости) |
| GraphQL | Язык запросов: клиент запрашивает ровно те поля, которые ему нужны | частично (**только запросы на чтение**) |
| WASM | Позволяет выполнять код на Rust прямо в браузере | нет (технология для интерфейса) |

- **API, которые вызываются из браузерных интерфейсов или через `curl`, строятся на Connect** (POST; JSON читается и записывается через RS-JSON). Браузерный WASM не умеет работать с «чистым» gRPC. С Connect достаточно обычного `fetch`, а `prost` не требуется — размер WASM-файла не увеличивается.
- **Для обмена между сервисами (сервер — сервер, ресурсоёмкие задачи, потоковая передача) применяется gRPC.** В большинстве случаев достаточно Connect; на gRPC переходят только там, где потоковая передача действительно необходима.
- **GraphQL используется только для чтения**, когда нужно получить сразу несколько связанных данных. Для записи (операций) он не применяется.
- **WASM по-прежнему остаётся технологией для интерфейса.** Изменилось лишь то, что вызываемый им API теперь построен на Connect.
- Интерфейсы, форма которых задана внешним стандартом (ACME, обратные вызовы OAuth, вебхуки сторонних сервисов, проверки работоспособности, статические файлы, OTLP), остаются обычным HTTP.
- **Если есть сомнения — выбирайте Connect.**

## API 移行(REST → Connect)の記録 (2026-11-04)
このリポジトリの REST 風 API(`/admin/...`・`/api/...`)は、**すべて Connect(POST、RS-JSON)(`POST /open_easy_web.admin.v1.<Service>/<Method>`、JSON、読み書きは RS-JSON)へ移し、旧 REST の経路を削除した**。対応表・例外とその理由は `CLAUDE.md`「旧 REST → Connect の対応表」、スキーマは `proto/open_easy_web/admin/v1/admin.proto`。
- **引っ越し(移植)時の注意**: ヘルスチェックは `GET /healthz` のまま。外から呼ぶスクリプトを書くときは `curl -X POST -H 'content-type: application/json' -H "x-admin-token: …" -d '{}' https://…/open_easy_web.admin.v1.System/GetMemory` の形にする。
- **期限つき互換は終了**: 旧 `POST /api/auth/request-otp`・`verify-otp` は、利用者の `open-upstream-watch` を Connect 版にして配備したので削除した(2026-11-04)。
- **本文の上限**: 入口で 64 KiB(`Sites/Upload` のみ 128 MiB)。以前はハンドラごとに無制限に読んでいた。外から入る JSON はすべて RS-JSON(`server/src/json_in.rs`)で読む。
- **メールの確認リンク**: 画面のURL(`/#/confirm-email-change?token=…`)で、GET では何も確定しない。確認ボタンで `Account/ConfirmEmailChange`(POST、RS-JSON)を呼ぶ。旧形式のリンクは案内ページ(410)。
- **ファイル送信**: `Sites/Upload`(base64 の JSON)。手書きの multipart パーサーは削除した。
- **範囲外**: 画面の「無料ドメイン」パネルが呼ぶ `/admin/ddns/…`・`/admin/sftp/…` は `open-web-server` 本体の API(同リポジトリ側で別途移行)。

## 2026-11-08 本番(VPS)反映の確認
- 稼働中のバイナリは `f7c5146`(AppServerUpsertTenant呼び出し・ime-legacy-route-hit計数を含む、13:23反映済み)で最新。再反映は不要。
- オンライン画面 `easy-web.tokyo/ime` は新経路 `/ime/api/aruaru_ime.v1.Ime/Key` のみを呼び(ブラウザで変換操作、全て200)、旧 `/ime/api/key|session` の呼び出しは無い(旧は2026-12-06まで互換)。
- AppServer系の呼び先 RPoem ゲートウェイはVPS未稼働。


## 2026-11-15 RPoem「分身の術」ホスティング(別アカウントでの再開用) / Bunshin hosting (resume on another account)

### 日本語
- **目的**: 1台のVPSに `open-web-server`(=Apache相当)と `RPoem`(=Tomcat相当)を**1回だけ**入れれば、そのVPSの全Webサイト(静的/PHP)とWebアプリ(Rust等)を、サイトごとの分身(ワーカー)で非同期・マルチスレッドに収容する。サイトごとのインストールはしない。
- **仕様書(正本)**: `open-easy-web/docs/RPOEM_BUNSHIN_HOSTING_SPEC.md`(§1.1=ユーザー確認済みの方針、§5.1=分離レベル、§7=差分と段階、§9=未決、§11=実装状況)。
- **決定済み(ユーザー)**: (1) 他人のサイトを預かるかは設定で切替。**既定は `isolation = "trusted"`(自分のサイトのみ)**、`"tenant"` は明示操作(`AllowTenantMode`)で有効化する別実装(未実装、`sites.toml` で指定すると拒否される)。(2) 設定は新設の **`sites.toml`**(`OPEN_WEB_SERVER_SITES_FILE`)。既存の `domains.toml` / `web_vhosts.toml` は読み取り互換のまま。(3) 振り分け=open-web-server、監督=RPoem appserver の分担を維持。(4) REST は新設しない(Connect/gRPC、JSONはRS-JSON)。
- **完了(すべて `origin/main` へpush済みを確認)**:
  - `open-web-server` `a862f61`: `sites.rs`(`sites.toml`→既存テナント/vhost展開)、サイト別同時接続上限(超過は503+`Retry-After`、processのみ)、ポート自動割当(IDハッシュで安定・使用中/固定ポート回避)。`sites.toml` 由来テナントは `domains.toml` へ永続化しない(`managed_by`)。
  - `open-web-server` `c971a38`: `site-fleet` feature(既定オフ)。`command` 付きprocessサイトを RPoem の `SiteFleet` で起動・監視。feature無しで `command` があれば設定エラー。
  - `RPoem` `2b50b3e`: `open-runo-appserver` の `fleet` feature、`SiteFleet`(全サイトを**1つのtokioタスク**で監督、ヘルスチェックは並行)。統合テスト(8サイト同時Healthy・stop/restart・remove)成功、feature無しの既存25件も成功。
  - `open-easy-web`: 仕様書(`87a61e5`、`0ddbb86`)。
- **未了(次にやること、優先順)**:
  1. `site-fleet` ありでの**実プロセスの通し確認**(`sites.toml`のprocessサイト→起動→ヘルシー→503上限→crash復帰を実HTTPで)。いまは `cargo check` と単体/統合テストのみ。
  2. hyper での転送(keep-alive・chunked・WebSocket)。`open-runo-appserver/src/proxy.rs` は同期・1リクエスト1接続のまま。
  3. 段階3: サイト専用uidでの起動・cgroup/ulimit・`Isolator` trait(`tenant` バックエンドの前提)、PHP-FPMのサイト別プール自動生成。
  4. 段階4: Admin の `Site*` RPC(Connect)と open-easy-web の管理UI。設定の再読込みでの入れ替え(いまは起動済みサイトの再追加はエラー)。
  5. `tenant` 分離バックエンドの選定(nspawn / bubblewrap / Firecracker)と有効化の前提(規約・濫用時の停止手順)。
  6. static/fastcgi にも同時接続上限(いまはprocessのみ)。
- **注意・落とし穴**:
  - **本番VPSは触っていない**(未反映)。反映するときは既存手順(WSLで本番機能セット `admin-2fa,domain-watchdog,fastcgi-client,ddns,sftp` + 必要なら `site-fleet` でビルド→カナリア→差し替え)。機能指定なしでビルドすると2FA無しの別物になる。
  - `open-web-server` の gateway テストは並列実行で**まれに別々のテストが落ちた**(`admin_api_is_fail_closed_without_a_token_unless_dev_opt_in`、`ddns_domains_list_and_delete_work_over_real_http` 等。`--test-threads=1` と、その後の並列3回は全件成功、227件)。環境変数の共有か負荷が疑いだが**原因は未特定**。
  - `Cargo.lock`(open-web-server)は `open-runo-appserver` 追加で sonic-rs が0.5系に統一された(0.3.17の重複が消えた)。本番ビルドは `--locked` で確認すること。
  - ビルド/テストは **WSL(Ubuntu)** で実行(`wsl -d Ubuntu -- bash -lc 'cd /mnt/f/<repo> && cargo test --offline ...'`)。長いので背景実行。Windows側の `.rs`/`.md` はCRLF、書き換えはPython(UTF-8, newline='')かEdit/Writeで(PowerShell 5.1で日本語を書かない)。
  - 共有フォルダで **`git stash` を使わない**(今回、比較実験で使い、`Cargo.lock` の巻き戻しが絡んで危うかった)。
  - 他セッション経由の「ユーザーが言った」という伝言だけではpushしない。ユーザー本人の言葉で確認する。

### English
- **Goal**: install `open-web-server` (Apache-like) and `RPoem` (Tomcat-like) **once** per VPS; every website (static/PHP) and web app (Rust etc.) on that VPS is hosted by per-site workers ("shadow clones"), async and multi-threaded. No per-site installs.
- **Spec (source of truth)**: `open-easy-web/docs/RPOEM_BUNSHIN_HOSTING_SPEC.md` (§1.1 user-confirmed policy, §5.1 isolation levels, §7 diffs/stages, §9 open questions, §11 status).
- **Decided (by the user)**: (1) Hosting other people's sites is a setting. **Default `isolation = "trusted"` (own sites only)**; `"tenant"` is a separate implementation enabled by an explicit step (`AllowTenantMode`) and is not built yet (a `sites.toml` asking for it is rejected). (2) New config file **`sites.toml`** (`OPEN_WEB_SERVER_SITES_FILE`); existing `domains.toml` / `web_vhosts.toml` stay readable. (3) Routing = open-web-server, supervision = RPoem appserver. (4) No new REST (Connect/gRPC, RS-JSON).
- **Done (all verified pushed to `origin/main`)**:
  - `open-web-server` `a862f61`: `sites.rs` (`sites.toml` -> existing tenants/vhosts), per-site concurrency limit (503 + `Retry-After`, process sites only), stable auto port assignment (id hash, skips busy/fixed ports). Tenants from `sites.toml` are not persisted to `domains.toml` (`managed_by`).
  - `open-web-server` `c971a38`: `site-fleet` feature (off by default). Process sites with `command` are started and supervised by RPoem's `SiteFleet`; without the feature a `command` is a config error.
  - `RPoem` `2b50b3e`: `fleet` feature in `open-runo-appserver`, `SiteFleet` (all sites supervised by **one tokio task**, health checks run concurrently). Integration test passes (8 sites Healthy, stop/restart, remove); the existing 25 tests without the feature pass.
  - `open-easy-web`: the spec (`87a61e5`, `0ddbb86`).
- **Not done (next, in priority order)**:
  1. **End-to-end real-process check** with `site-fleet` (process site from `sites.toml` -> start -> healthy -> 503 limit -> crash recovery over real HTTP). So far only `cargo check` plus unit/integration tests.
  2. hyper-based forwarding (keep-alive, chunked, WebSocket); `open-runo-appserver/src/proxy.rs` is still synchronous, one request per connection.
  3. Stage 3: per-site uid, cgroup/ulimit, an `Isolator` trait (prerequisite for the `tenant` backend), per-site PHP-FPM pools.
  4. Stage 4: Admin `Site*` RPCs (Connect) and an open-easy-web UI; reload-time replacement (re-adding a running site is currently an error).
  5. Pick the `tenant` isolation backend (nspawn / bubblewrap / Firecracker) and its enabling preconditions (terms, abuse-stop procedure).
  6. Concurrency limits for static/fastcgi sites (process only today).
- **Cautions / pitfalls**:
  - **The production VPS was not touched.** When deploying use the existing procedure (WSL build with the production feature set `admin-2fa,domain-watchdog,fastcgi-client,ddns,sftp` plus `site-fleet` if needed -> canary -> swap). Building without the feature list yields a 2FA-less binary.
  - `open-web-server` gateway tests **occasionally failed in parallel runs**, a different test each time (`admin_api_is_fail_closed_without_a_token_unless_dev_opt_in`, `ddns_domains_list_and_delete_work_over_real_http`, ...); `--test-threads=1` and three later parallel runs passed (227 tests). Shared env vars or load are suspected; **cause not identified**.
  - In open-web-server's `Cargo.lock`, adding `open-runo-appserver` unified sonic-rs to 0.5 (the 0.3.17 duplicate disappeared). Verify the production build with `--locked`.
  - Build and test in **WSL (Ubuntu)** (`wsl -d Ubuntu -- bash -lc 'cd /mnt/f/<repo> && cargo test --offline ...'`), in the background (slow). Windows-side `.rs`/`.md` files are CRLF; edit with Python (UTF-8, `newline=''`) or Edit/Write, never PowerShell 5.1 for Japanese text.
  - **Do not use `git stash` in shared folders** (used once for a comparison run; it got tangled with `Cargo.lock` and was risky).
  - Do not push on the strength of another session's "the user said so"; confirm with the user's own words.

### 再開用メッセージ / Resume message (コピーして新しいセッションに貼る / paste into a new session)

```
RPoem「分身の術」ホスティングの続きをお願いします。リポジトリは F:\open-easy-web(仕様書 docs/RPOEM_BUNSHIN_HOSTING_SPEC.md)、F:\open-web-server(crates/open-web-server-gateway の sites.rs / state.rs / tenant_router.rs、feature=site-fleet)、F:\RPoem(crates/open-runo-appserver の fleet.rs)です。
まず仕様書の §1.1・§9・§11 と、各リポジトリの CLAUDE.md / PORTING.md 末尾の「2026-11-15 RPoem「分身の術」ホスティング」節を読んでください。
状態: 段階1(open-web-server a862f61)、配線(c971a38)、SiteFleet(RPoem 2b50b3e)は origin/main に push 済み。本番VPSは未反映です。
次にやること: (1) site-fleet ありで sites.toml の process サイトを実プロセスで通し確認(起動→healthy→503上限→crash復帰)、(2) hyper での転送、(3) 段階3(別uid・cgroup・Isolator trait・PHP-FPMプール)。
決定済み: 既定は isolation="trusted"(自分のサイトのみ)、"tenant" は別実装・明示操作で有効化。設定は新設の sites.toml、REST は新設しない(Connect/gRPC、JSONはRS-JSON)。
注意: ビルド/テストは WSL(Ubuntu)で背景実行。git stash は使わない。本番VPSは確認なしに触らない。push はユーザー本人の言葉があってから。チャットは日本語のみ。
```

```
Please continue the RPoem "shadow-clone" (bunshin) hosting work. Repositories: F:\open-easy-web (spec docs/RPOEM_BUNSHIN_HOSTING_SPEC.md), F:\open-web-server (crates/open-web-server-gateway: sites.rs / state.rs / tenant_router.rs, feature=site-fleet) and F:\RPoem (crates/open-runo-appserver: fleet.rs).
First read spec sections 1.1, 9 and 11, and the section "2026-11-15 RPoem bunshin hosting" at the end of each repository's CLAUDE.md / PORTING.md.
State: stage 1 (open-web-server a862f61), the wiring (c971a38) and SiteFleet (RPoem 2b50b3e) are pushed to origin/main. Production VPS is untouched.
Next: (1) end-to-end check of a process site from sites.toml with site-fleet on real processes (start -> healthy -> 503 limit -> crash recovery), (2) hyper-based forwarding, (3) stage 3 (per-site uid, cgroup, Isolator trait, PHP-FPM pools).
Decided: default isolation="trusted" (own sites only); "tenant" is a separate, explicitly enabled implementation. New config file sites.toml; no new REST (Connect/gRPC, RS-JSON).
Cautions: build and test in WSL (Ubuntu) in the background. No git stash. Do not touch the production VPS without asking. Push only on the user's own words. Chat in Japanese only.
```

## open-cpu の同梱と、アクセラレーターの扱い(2026-11-17)
- **同梱済み(実行時に AVX-512BW > AVX2 > NEON > スカラーを自動選択、`OPEN_CPU_MAX_ISA`で上限指定可)**: MFastAPI(`--cpu`、長いクエリ文字列の探索)、RMojo(`rmojo cpu`、`str.find`/`str.count`)、open-web-server・open-easy-web(起動時の要約、`System/GetCpu`、FastCGIヘッダー走査)。実機はNEONをSnapdragon端末2台で、AVX-512BWはCIで確認(開発PCはAVX2まで)。
- **2FastAPI**: 変換ツール(Python)で、実行時の計算処理を持たないため open-cpu は使わない。
- **open-cuda / aruaru-llm / open-directx は同梱しない**: 共有基盤を1回だけVPSに入れ、各サイトはConnect/gRPCで呼ぶ方針(本ファイル冒頭の配備アーキテクチャ方針)に従う。
- **ブラウザ(Windows/Linux/Mac/Android/iPhone)**: 計算はサーバー側で行うため、どの端末のブラウザからでも利用できる。RMojoのWASM(`wasm32-unknown-unknown`の`cargo check`は通過)は将来の遊び場候補。WASMでは open-cpu のSIMD自動選択は使わない。
- **スマホのアクセラレーター化**: `open-cpu/remote`(`open-cpu-remote`)。実測では手元PCが速いため自動判断は「手元のみ」で、手元が非力な場合に効く。詳細は`open-cpu/remote/README.md`。

## 2026-11-17 追記: 実プロセス通し確認の完了・再開用メッセージ更新(別アカウントでの再開用) / Real-process E2E done, resume message updated (resume on another account)

> この節が最新。直前の「2026-11-15 RPoem「分身の術」ホスティング」節の**未了1(実プロセス通し確認)は完了**した。それ以外の記述(目的・決定済み・コミット・注意)はそのまま有効。 / This section supersedes the 2026-11-15 one: its open item 1 (real-process E2E) is now **done**; everything else there remains valid.

### 日本語
- **完了(今回)**: `open-web-server` の `crates/open-web-server-gateway/tests/e2e_site_fleet.rs`(新規)。実バイナリを起動し、`sites.toml` の process サイトを `site-fleet` で実プロセスとして動かし、実HTTPで次を確認した(1件・3.04秒・成功、スキップではないことも確認)。
  1. 本体が分身(RPoemの`open-runo-dummy-http-server`)を起動し、Hostで振り分けて `/health` が200(固定ポートと `port = "auto"` の2サイト)。
  2. 同時接続上限(`limits.max_concurrent = 1`): 1件が待っている間の2件目は **503 + `Retry-After: 1`**。別サイトは200のまま。塞ぎを外すと待っていた1件目が成功し、枠が戻る。
  3. crash復帰: `/crash` で分身が落ちても `SiteFleet` が再起動し `/health` が200に戻る。
- **試験の実行方法**: WSLで `wsl -d Ubuntu -- bash -lc 'cd /mnt/f/open-web-server && cargo test --offline -p open-web-server-gateway --features site-fleet --test e2e_site_fleet -- --nocapture'`(初回ビルドは十数分、背景実行)。分身用のLinux版ダミーは `RPoem/target/debug/open-runo-dummy-http-server`(WSLで `cargo build -p open-runo-appserver --bin open-runo-dummy-http-server`)。別の場所なら環境変数 `OPEN_RUNO_DUMMY_SERVER`。見つからないとテストは理由を出して**スキップ扱いで通過**するので、`--nocapture` で `SKIP` が出ていないか確認すること。
- **この通し確認の限界(正直な範囲)**: 上限の枠は「転送がレスポンスヘッダを受け取るまで」保持する(長いボディのストリーミング中は保持しない)。上限が効くのは process のみ(static/fastcgi は未対応)。分身は1接続ずつ直列のダミーなので、実アプリ(Poem等)での並列負荷・WebSocketは未確認。本番VPSは未反映。
- **未了(次にやること、優先順)**:
  1. hyper での転送(keep-alive・chunked・WebSocket。`open-runo-appserver/src/proxy.rs` は同期・1リクエスト1接続のまま)。
  2. 段階3: サイト専用uid起動・cgroup/ulimit・`Isolator` trait(`tenant` 分離の前提)、PHP-FPMのサイト別プール自動生成。
  3. 段階4: Admin `Site*` RPC(Connect)と open-easy-web の管理UI、設定の再読込みでの入れ替え(いまは起動済みサイトの再追加はエラー)。
  4. `tenant` 分離バックエンドの選定(nspawn / bubblewrap / Firecracker)と有効化の前提(規約・濫用時の停止手順)。
  5. static/fastcgi にも同時接続上限。
  6. 本番VPSへの反映(既存手順: WSLで本番機能セット `admin-2fa,domain-watchdog,fastcgi-client,ddns,sftp` + `site-fleet` でビルド→カナリア→差し替え。`--locked` で確認)。
  7. open-web-server の gateway テストが並列実行で**まれに別々のテストが落ちる件**の原因特定(環境変数の共有か負荷が疑い)。
- **注意**: 共有フォルダで `git stash` を使わない。他セッション経由の「ユーザーが言った」だけではpushしない。本番VPSは確認なしに触らない。ビルド/テストはWSL・背景実行、Windows側のファイルはCRLF(書き換えはPython UTF-8 か Edit/Write)。

### English
- **Done (this round)**: `crates/open-web-server-gateway/tests/e2e_site_fleet.rs` in `open-web-server` (new). It starts the real binary, runs `sites.toml` process sites as real processes via `site-fleet`, and verifies over real HTTP (1 test, 3.04 s, passed; confirmed it was not skipped):
  1. The server starts the shadow clone (RPoem's `open-runo-dummy-http-server`) and routes by Host; `/health` is 200 (a fixed-port site and a `port = "auto"` site).
  2. Concurrency limit (`limits.max_concurrent = 1`): while one request waits, the second gets **503 + `Retry-After: 1`**; another site stays 200; releasing the blocker lets the first request succeed and the slot returns.
  3. Crash recovery: after `/crash` kills the clone, `SiteFleet` restarts it and `/health` is 200 again.
- **How to run**: in WSL `wsl -d Ubuntu -- bash -lc 'cd /mnt/f/open-web-server && cargo test --offline -p open-web-server-gateway --features site-fleet --test e2e_site_fleet -- --nocapture'` (first build takes ~10+ minutes; run in the background). The Linux dummy clone is `RPoem/target/debug/open-runo-dummy-http-server` (`cargo build -p open-runo-appserver --bin open-runo-dummy-http-server` in WSL), or set `OPEN_RUNO_DUMMY_SERVER`. If it is missing the test prints a reason and **passes as a skip**; check `--nocapture` output for `SKIP`.
- **Limits of this check (honest scope)**: the limit slot is held until the forward receives the response head (not during long body streaming); only process sites are limited (static/fastcgi are not); the clone is a serial dummy, so parallel load and WebSocket with a real app (Poem etc.) are unverified; production VPS is untouched.
- **Open (next, in priority order)**:
  1. hyper-based forwarding (keep-alive, chunked, WebSocket; `open-runo-appserver/src/proxy.rs` is still synchronous, one request per connection).
  2. Stage 3: per-site uid, cgroup/ulimit, `Isolator` trait (prerequisite for `tenant` isolation), per-site PHP-FPM pools.
  3. Stage 4: Admin `Site*` RPCs (Connect) and an open-easy-web UI; reload-time replacement (re-adding a running site is currently an error).
  4. Choose the `tenant` isolation backend (nspawn / bubblewrap / Firecracker) and its enabling preconditions (terms, abuse-stop procedure).
  5. Concurrency limits for static/fastcgi sites.
  6. Production deploy (existing procedure: WSL build with the production feature set `admin-2fa,domain-watchdog,fastcgi-client,ddns,sftp` plus `site-fleet`, canary, swap; verify with `--locked`).
  7. Find the cause of the occasional, different-each-time failures of open-web-server gateway tests under parallel runs (shared env vars or load suspected).
- **Cautions**: no `git stash` in shared folders; do not push on another session's "the user said so"; do not touch the production VPS without asking; build/test in WSL in the background; Windows-side files are CRLF (edit with Python UTF-8 or Edit/Write).

### 再開用メッセージ(最新) / Resume message (latest)

```
RPoem「分身の術」ホスティングの続きをお願いします。リポジトリは F:\open-easy-web(仕様書 docs/RPOEM_BUNSHIN_HOSTING_SPEC.md)、F:\open-web-server(crates/open-web-server-gateway の sites.rs / state.rs / tenant_router.rs と tests/e2e_site_fleet.rs、feature=site-fleet)、F:\RPoem(crates/open-runo-appserver の fleet.rs)です。
まず各リポジトリの CLAUDE.md / PORTING.md 末尾の「2026-11-17 追記: 実プロセス通し確認」節と、仕様書の §1.1・§9・§11 を読んでください。
状態: 段階1(open-web-server a862f61)、配線(c971a38)、SiteFleet(RPoem 2b50b3e)は push 済み。実プロセスの通し確認(起動・転送・503上限・crash復帰)は e2e_site_fleet で成功済み。本番VPSは未反映です。
次にやること: (1) hyper での転送(keep-alive・chunked・WebSocket)、(2) 段階3(別uid・cgroup・Isolator trait・PHP-FPMプール)、(3) Admin Site* RPC と管理UI。
決定済み: 既定は isolation="trusted"(自分のサイトのみ)、"tenant" は別実装・明示操作で有効化。設定は新設の sites.toml、REST は新設しない(Connect/gRPC、JSONはRS-JSON)。
注意: ビルド/テストは WSL(Ubuntu)で背景実行。git stash は使わない。本番VPSは確認なしに触らない。push はユーザー本人の言葉があってから。チャットは日本語のみ。
```

```
Please continue the RPoem "shadow-clone" (bunshin) hosting work. Repositories: F:\open-easy-web (spec docs/RPOEM_BUNSHIN_HOSTING_SPEC.md), F:\open-web-server (crates/open-web-server-gateway: sites.rs / state.rs / tenant_router.rs and tests/e2e_site_fleet.rs, feature=site-fleet) and F:\RPoem (crates/open-runo-appserver: fleet.rs).
First read the section "2026-11-17 追記: 実プロセス通し確認" at the end of each repository's CLAUDE.md / PORTING.md, and spec sections 1.1, 9 and 11.
State: stage 1 (open-web-server a862f61), the wiring (c971a38) and SiteFleet (RPoem 2b50b3e) are pushed. The real-process E2E (start, forward, 503 limit, crash recovery) passes in e2e_site_fleet. Production VPS is untouched.
Next: (1) hyper-based forwarding (keep-alive, chunked, WebSocket), (2) stage 3 (per-site uid, cgroup, Isolator trait, PHP-FPM pools), (3) Admin Site* RPCs and an admin UI.
Decided: default isolation="trusted" (own sites only); "tenant" is a separate, explicitly enabled implementation. New config file sites.toml; no new REST (Connect/gRPC, RS-JSON).
Cautions: build and test in WSL (Ubuntu) in the background. No git stash. Do not touch the production VPS without asking. Push only on the user's own words. Chat in Japanese only.
```


## 2026-11-18 追記: QR確認ログインのVPS反映 / QR-confirm login deployed to the VPS

**日本語**: VPS(`/root/open-easy-web-app`、`open-easy-web.service`)のバイナリはQR確認ログイン(`qr`/`otp_qr`、`origin/main`の`8570230`相当)より古いビルドだったため、`cargo build --release`(約4分)で再ビルドして再起動した(旧バイナリは`server/target/release/open-easy-web-server.before-qr-20261118`、ロールバックはこれを戻して`systemctl restart open-easy-web`)。確認: `healthz`200、`/qr-confirm.html`200、公開URLでも`open_easy_web.admin.v1.Auth/QrLoginWhoami`が期待どおりの404(`not_found`、無効なid)・`QrLoginStart`が403(TOTP未有効の宛先)、`Settings/GetLoginMode`は管理トークンなしで401。**注意**: 認証のRESTは廃止済みで、`/api/auth/qr-login/*`は404が正しい(Connectは`POST /open_easy_web.admin.v1.Auth/<Method>`)。**未了**: 実スマホのカメラでの撮影の通し試験、実SMTPでの`otp_qr`通し。

**English**: The binary on the VPS (`/root/open-easy-web-app`, `open-easy-web.service`) predated the QR-confirm login (`qr`/`otp_qr`, equivalent to `8570230` on `origin/main`), so it was rebuilt with `cargo build --release` (about 4 minutes) and restarted (old binary kept as `server/target/release/open-easy-web-server.before-qr-20261118`; roll back by restoring it and running `systemctl restart open-easy-web`). Verified: `healthz` 200, `/qr-confirm.html` 200, and on the public URL `open_easy_web.admin.v1.Auth/QrLoginWhoami` answers the expected 404 (`not_found`, invalid id) and `QrLoginStart` 403 (TOTP not enabled for that address); `Settings/GetLoginMode` returns 401 without the admin token. **Note**: the REST auth routes are abolished, so `/api/auth/qr-login/*` correctly returns 404 (Connect is `POST /open_easy_web.admin.v1.Auth/<Method>`). **Open**: a real phone-camera scan and `otp_qr` with real SMTP.

## 引っ越し・再開メモ(2026-11-19) / Moving and resuming

**日本語**
- 本番(VPS)への反映は、WSLでビルド → バックアップ → 差し替え → `systemctl restart open-easy-web` → トップが200・認証なしの管理APIが401、を確認する。ロールバックは `*.before-allrepos-20261119`。
- 一覧を作り直すときは、`gh repo list` の結果から `all_repos.rs` を再生成して同じ手順で入れる。
- 再開の最初の一手: Inno Setupでインストーラーを作り、フォルダーを替えて入れる・消す試験をする。

**English**
- To deploy to production (VPS): build in WSL, take a backup, swap the binary, run `systemctl restart open-easy-web`, then check the top page returns 200 and the admin API without a token returns 401. Roll back with `*.before-allrepos-20261119`.
- To refresh the list, regenerate `all_repos.rs` from `gh repo list` and deploy the same way.
- First step when resuming: build the installer with Inno Setup, then test install and uninstall with a different folder.

## 他の言語 / Other languages (2026-11-19)

### 简体中文

- 部署到生产环境(VPS):在 WSL 中构建 → 备份 → 替换 → `systemctl restart open-easy-web` → 确认首页返回 200、无令牌的管理接口返回 401。回滚用 `*.before-allrepos-20261119`。
- 要更新列表,先用 `gh repo list` 的结果重新生成 `all_repos.rs`,再按同样步骤部署。
- 恢复工作时第一步:用 Inno Setup 构建安装程序,换一个文件夹做安装和卸载测试。

### 繁體中文(台灣)

- 部署到正式環境(VPS):在 WSL 中建置 → 備份 → 替換 → `systemctl restart open-easy-web` → 確認首頁回傳 200、無權杖的管理介面回傳 401。回復用 `*.before-allrepos-20261119`。
- 要更新清單,先用 `gh repo list` 的結果重新產生 `all_repos.rs`,再依同樣步驟部署。
- 恢復工作時第一步:用 Inno Setup 建置安裝程式,換一個資料夾做安裝與解除安裝測試。

### 한국어

- 운영(VPS) 반영: WSL에서 빌드 → 백업 → 교체 → `systemctl restart open-easy-web` → 첫 화면이 200, 토큰 없는 관리 API가 401인지 확인합니다. 되돌릴 때는 `*.before-allrepos-20261119`.
- 목록을 새로 만들 때는 `gh repo list` 결과로 `all_repos.rs`를 다시 생성하고 같은 절차로 반영합니다.
- 작업을 이어받을 때 첫걸음: Inno Setup으로 설치 프로그램을 만들고, 폴더를 바꿔 설치·삭제를 시험합니다.

### Deutsch

- Produktiv (VPS) ausrollen: in WSL bauen → Sicherung → austauschen → `systemctl restart open-easy-web` → prüfen, dass die Startseite 200 und die Admin-API ohne Token 401 liefert. Zurück mit `*.before-allrepos-20261119`.
- Zum Aktualisieren der Liste `all_repos.rs` aus `gh repo list` neu erzeugen und genauso ausrollen.
- Erster Schritt beim Weitermachen: den Installer mit Inno Setup bauen und Installation und Deinstallation mit einem anderen Ordner testen.

### Français

- Déployer en production (VPS) : compiler dans WSL → sauvegarde → remplacer → `systemctl restart open-easy-web` → vérifier que la page d'accueil renvoie 200 et l'API d'administration sans jeton 401. Retour arrière avec `*.before-allrepos-20261119`.
- Pour rafraîchir la liste, régénérez `all_repos.rs` à partir de `gh repo list` et déployez de la même façon.
- Premier pas à la reprise : construire l'installateur avec Inno Setup, puis tester l'installation et la désinstallation avec un autre dossier.

### Español

- Desplegar en producción (VPS): compilar en WSL → copia de seguridad → sustituir → `systemctl restart open-easy-web` → comprobar que la página principal devuelve 200 y la API de administración sin token 401. Para volver atrás: `*.before-allrepos-20261119`.
- Para actualizar la lista, regenera `all_repos.rs` a partir de `gh repo list` y despliega igual.
- Primer paso al retomar: construir el instalador con Inno Setup y probar instalar y desinstalar con otra carpeta.

### Português (BR)

- Implantar em produção (VPS): compilar no WSL → backup → trocar → `systemctl restart open-easy-web` → conferir que a página inicial devolve 200 e a API de administração sem token devolve 401. Para voltar: `*.before-allrepos-20261119`.
- Para atualizar a lista, gere de novo o `all_repos.rs` a partir de `gh repo list` e implante do mesmo jeito.
- Primeiro passo ao retomar: montar o instalador com o Inno Setup e testar instalar e desinstalar com outra pasta.

### Русский

- Выкатка в продакшн (VPS): сборка в WSL → резервная копия → замена → `systemctl restart open-easy-web` → проверить, что главная отдаёт 200, а админ-API без токена — 401. Откат: `*.before-allrepos-20261119`.
- Чтобы обновить список, заново сгенерируйте `all_repos.rs` из `gh repo list` и выкатите так же.
- Первый шаг при возобновлении: собрать установщик в Inno Setup и проверить установку и удаление с другой папкой.

(简体中文: AI 翻译,尚未经母语者确认。; 繁體中文(台灣): AI 翻譯,尚未經母語者確認。; 한국어: AI 번역이며 원어민 확인 전입니다.; Deutsch: KI-Übersetzung, noch nicht von Muttersprachlern geprüft.; Français: Traduction par IA, pas encore relue par des locuteurs natifs.; Español: Traducción por IA, aún sin revisión de hablantes nativos.; Português (BR): Tradução por IA, ainda sem revisão de falantes nativos.; Русский: Перевод выполнен ИИ, носителями языка пока не проверен.)

## 2026-11-20 依存の更新(version-control通知) / Dependency update
- **日本語**: server: lettre 0.11.22→0.11.24(RUSTSEC-2026-0560〜0565)、qrcode 0.14→0.15。コードの変更は不要、`cargo test` 137件成功。本番の再ビルド・差し替えは未実施。VPSの`/root/repository/open-easy-web-app`は`server/Cargo.lock`に未コミット変更があるため触っていない。
- **English**: server: lettre 0.11.22 -> 0.11.24 (RUSTSEC-2026-0560..0565), qrcode 0.14 -> 0.15. No code changes; `cargo test` 137 passed. Production not rebuilt. The VPS clone `/root/repository/open-easy-web-app` has an uncommitted `server/Cargo.lock` change, so it was left alone.
