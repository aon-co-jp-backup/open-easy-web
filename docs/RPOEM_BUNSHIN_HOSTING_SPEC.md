# RPoem「分身の術」ホスティング仕様(案・ユーザー確認待ち)

作成日: 2026-11-09 / 状態: **仕様案(実装・VPS反映は未着手)**。2026-11-09 ユーザー回答を反映(§1.1)
依頼(原文): 「open-easy-web で RPoem を Apache と Tomcat の様に、忍者分身の術の様に、一つの VPS でインストールすれば、他の WEB サイトでも WEB アプリにも忍者分身の術で非同期・マルチスレッドで対応する仕様として」

## 1. 目的

1台の VPS に RPoem(+ open-web-server)を1回入れるだけで、複数の**Webサイト**(静的/PHP)と**Webアプリ**(Rust/Python/Ruby 等)を、サイトごとに分離した**ワーカー(分身)**で、tokio 非同期 + マルチスレッドに収容する。サイトを足す作業は「宣言1件の追加」だけにする(ドメイン別インストール不要)。

### 1.1 ユーザー確認済みの方針(2026-11-09)

- **open-web-server をインストール = Apache、RPoem もインストール = Tomcat**。両方を**1つのVPSに1回**入れれば、そのVPSの全Webサイト/Webアプリを自分で管理する(役割分担は現状維持)。
- 全サイトを忍者分身の術(サイトごとの分身)で、非同期・マルチスレッドに収容する。
- **他人のサイトまで預かるかは設定で切り替える**。既定は自分のサイトのみ(`trusted`)。Claudeでのカスタマイズ(設定変更+分離バックエンド追加)で、他人のサイトを預かる運用(`tenant`)にも変更できる設計にする(§5.1)。

## 2. 現状(調査結果。すべてコード・文書で確認した事実)

| 層 | 現状 | 場所 |
|---|---|---|
| 入口(第二のApache) | `open-web-server`(本番はnginxでなくこれ)。`#[tokio::main]`の非同期。TLS終端+`TenantRegistry`でHost(+`path_prefix`)別に振り分け。登録は `domains.toml` または Connect `Admin/AddTenant` で**再起動不要**、変更はTOMLへ永続化 | `open-web-server/crates/open-web-server-gateway/src/tenant_router.rs` |
| テナント定義 | `TenantConfig{host, backend(open_runo/poem_cosmo_tauri), backend_addr, db_uri, path_prefix, override_host, strip_prefix}`。同一Hostで複数prefix共存可。読み取りは`RwLock`で競合しない | 同上 |
| 静的/PHP | `web_vhosts.toml`(`fastcgi_addr`、`fastcgi-client` feature)。ブロッキングI/Oは`spawn_blocking` | `web_vhosts.toml.example` |
| 第二のTomcat | `open-runo-appserver`。`RuntimeProfile`(スタック・起動コマンド・env・workdir・port・health_path)、`Supervisor`(crash-loop backoff)、`ProcessLifecycleManager`(監視スレッド+HTTPヘルス+stop/restart)、`TenantDispatcher`/`SupervisedTenantRegistry`(host→upstream) | `RPoem/crates/open-runo-appserver/src/` |
| 転送 | `proxy.rs`は**同期**`std::net`、1リクエスト=1接続、chunked/WebSocket/keep-alive非対応(Phase 2)。`server.rs`は固定ワーカースレッド(`workers`) | 同 |
| 管理API | RPoem `open_runo.admin.v1.Admin`(gRPC/gRPC-web/Connect)、open-web-server `Admin/AddTenant`等(Connect)。RESTは全廃済み | |
| open-easy-web | 管理UI(`server/`)。`vhost.rs`はnginx用vhost生成(本番では実質残骸)。`appserver_registration.rs`あり | `open-easy-web/server/src/` |

**結論**: 「入口でHost別に振り分け→サイトごとの子プロセス(分身)を監視」までは既にある。**足りないのは (a) 分離の強さ、(b) サイトごとの資源上限と並列度、(c) 非同期の監督・転送、(d) 1つのサイト宣言にまとめる仕組み、(e) RPoem自身が複数サイト/アプリを内包して動かす経路**。

## 3. 用語と全体像

- **本体(本体の術者)**: open-web-server(入口・TLS・振り分け)と RPoem(管理面・監督)。
- **分身(shadow)**: 1サイト/1アプリ専用のワーカー。種別は3つ。
  1. `static` — 本体内の非同期ファイル配信(プロセス不要、軽い)。
  2. `fastcgi` — PHP-FPMプール(サイトごとに別プール・別ユーザー)。
  3. `process` — 子プロセス(Rust/Python/Ruby…)。`RuntimeProfile`で起動、upstreamへ中継。
  4.(将来)`inproc` — RPoem内にtokioタスクとして同居するRustハンドラ(同一信頼境界のサイト限定)。
- **スロット**: 1分身=1サイトの資源枠(同時接続数・CPU・メモリ・プロセス数)。

```
Internet ─TLS→ open-web-server(tokio, 全コア)
                ├ Host/prefix照合(TenantRegistry, RwLock読み)
                ├ サイト別セマフォ(同時接続上限) ← 新規
                ├ static  : 非同期ファイル配信
                ├ fastcgi : サイト専用php-fpmプール(別uid)
                └ process : サイト専用子プロセス(別uid, cgroup上限) ← 監督はRPoem appserver(tokio化)
RPoem Admin(Connect/gRPC) ─ サイトの追加/削除/再起動/状態/ログ
```

## 4. 非同期・マルチスレッド方針

1. **入口は1つの tokio マルチスレッドランタイム**(open-web-server既存)。ワーカースレッド数=コア数が既定、`[runtime] worker_threads`で上書き。サイトごとにランタイムは作らない(スレッド爆発・メモリ浪費を避ける)。
2. **サイト間の公平性はランタイム分割でなく、サイト別 `tokio::sync::Semaphore`(同時リクエスト上限)とキュー待ち上限で担保**する。上限超過は 503 + `Retry-After`(黙って落とさず計測)。1サイトの暴走が他サイトを飢えさせない。
3. **CPUを食う/ブロックする処理は `spawn_blocking` 専用プールへ**(既存方針を踏襲)。必要ならサイト別に上限付きの専用プールを持つ(未決 §9-2)。
4. **appserverの監督を tokio 化**: 現状は `std::thread` の監視スレッド+同期`proxy_once`。これを `tokio::process` + 非同期ヘルスチェック + 1つの監督タスク(サイト数が増えてもスレッドが増えない)へ置換。転送は hyper で keep-alive / chunked / WebSocket upgrade 対応(現proxyの非対応事項の解消)。
5. 各分身の内部並列度は分身側の責務(Rust/Poemなら自前のtokio、PHP-FPMなら`pm.max_children`)。本体は上限だけ設定。

## 5. 分離境界(「分身」が互いに触れない範囲)

| 項目 | 方針 | 強度 |
|---|---|---|
| ファイル | サイトごとの専用ディレクトリ `/srv/sites/<id>/`、サイト専用OSユーザー、他サイトのパスは読めない | 必須 |
| プロセス | サイトごとに専用uid/gidで起動(`setuid`相当。Windowsは対象外/別方式) | 必須(Linux) |
| 資源 | cgroup v2 / systemd transient scope で CPU・メモリ・pids上限、`ulimit`(nofile) | 既定ON、未対応環境は警告 |
| ネットワーク | 分身は `127.0.0.1:<割当ポート>` か Unixソケットのみ待受。外向きは既定許可、`deny_outbound=true`で制限(将来) | ポート衝突は本体が割当 |
| 秘密 | env は分身ごとに別、`.env`はサイトdirに0600。管理トークンは分身に渡さない | 必須 |
| DB | サイトごとに別DB/別資格情報(`db_uri`) | 必須 |
| ログ | サイト別ログファイル+サイト別ローテート、本体のアクセスログにsite idを付与 | 必須 |
| 障害 | 1分身のcrash/ハングは他に波及しない(crash-loop backoff、`GaveUp`で停止+通知) | 既存を踏襲 |

### 5.1 分離レベルの切替(`isolation`)

| レベル | 想定 | 分離方法 | 既定 |
|---|---|---|---|
| `trusted` | 自分のサイトのみ | 上表(専用uid+cgroup+専用dir) | **既定** |
| `tenant` | 他人のサイトも預かる | `trusted`に加え、分身をコンテナ/microVM(例: systemd-nspawn・bubblewrap・Firecracker等から選定)内で起動、ファイルシステム読み取り専用、外向き通信は許可リスト、サイト別DBユーザー、割当(クォータ)と課金/停止の管理面、ビルド/アップロード経路の検査 | 切替で有効化 |

- `sites.toml` の `[runtime] isolation = "trusted" | "tenant"`、サイト単位の上書き `isolation = ...` も可。
- 分離バックエンドは差し替え可能な trait(`Isolator`: `prepare / spawn / limits / teardown`)にし、`trusted`実装を先に作る。`tenant`実装は後段で同じ trait に追加するだけで、本体・設定・管理APIは変えない。
- `tenant` を有効にする前提条件: 利用規約・責任分界・濫用対応(停止手順)の整備。技術面だけでは安全にならないため、有効化は明示操作(`AllowTenantMode`)とする。

## 6. サイト宣言(設定例)

既存の `domains.toml` / `web_vhosts.toml` は**そのまま読める互換を保ち**、新たに1ファイルの `sites.toml` を足す案(内部では既存の `TenantConfig` と `RuntimeProfile` へ展開)。

```toml
[runtime]
worker_threads = 0          # 0=コア数
sites_dir = "/srv/sites"

[[site]]                    # 静的サイト
id = "blog-a"
hosts = ["blog-a.example.com"]
kind = "static"
root = "/srv/sites/blog-a/public"
limits = { max_concurrent = 200 }

[[site]]                    # PHP
id = "shop"
hosts = ["shop.example.com"]
kind = "fastcgi"
root = "/srv/sites/shop/public"
php = { version = "8.3", max_children = 8 }
user = "site-shop"
limits = { max_concurrent = 64, memory_mb = 512 }

[[site]]                    # Rust(Poem)アプリ
id = "app1"
hosts = ["app1.example.com"]
path_prefix = "/"
kind = "process"
stack = "rust_poem"
command = "./target/release/app"
workdir = "/srv/sites/app1"
port = "auto"               # 本体が空きポートを割当(env PORTに渡す)
health_path = "/health"
user = "site-app1"
db_uri = "postgres://app1@localhost/app1"
limits = { max_concurrent = 128, memory_mb = 1024, cpu_percent = 200, pids = 256 }
restart = { max_rapid_failures = 5 }
```

操作はConnect(新規REST無し): `open_runo.admin.v1.Admin` に `Site*` を追加する案 — `AddSite / RemoveSite / ListSites / RestartSite / SiteStatus / TailSiteLog`(JSONはRS-JSON、`x-admin-token`+将来は2FAのBearer)。`AddSite` は `sites.toml` へ永続化し、ノーダウンで反映(既存`TenantRegistry`と同じ流儀)。

## 7. 既存との差分(実装が要るもの)

| # | 差分 | 既存の何を変える | 規模 |
|---|---|---|---|
| 1 | `sites.toml` → `TenantConfig`/`RuntimeProfile`/vhost への展開層 | 新規(tenant_router・appserverは流用) | 小 |
| 2 | サイト別セマフォ+503制御+計測 | open-web-server gatewayへ追加 | 小 |
| 3 | appserver監督のtokio化、hyper転送(keep-alive/chunked/WebSocket) | `process_lifecycle.rs`・`proxy.rs`・`server.rs`置換 | 大 |
| 4 | 別uid起動+cgroup/ulimit | `Supervisor`の起動部 | 中(Linux限定) |
| 5 | ポート自動割当と衝突回避 | appserver | 小 |
| 6 | PHP-FPMのサイト別プール自動生成 | open-easy-web `vhost.rs`のnginx生成を、FPMプール生成へ(nginx前提を外す) | 中 |
| 7 | Admin `Site*` RPC(Connect)+ open-easy-web 管理UI | RPoem/open-web-server admin proto、`connect.rs` | 中 |
| 8 | サイト別ログ・状態(Healthy/BackingOff/GaveUp)の画面表示 | open-easy-web | 小 |
| 9 | `inproc`(RPoem内同居)分身 | 後段。Rustの動的ロードは行わず、同一バイナリにcrate同梱が前提 | 後回し |

**段階案**: P1=差分1・2・5(設定とセマフォ、既存機構のみ)→ P2=3(非同期化)→ P3=4・6(分離強化・PHPプール)→ P4=7・8(管理面)→ P5=9。`tenant` 分離(§5.1)は P3 の `Isolator` trait 導入後、別段階で追加。各段階で実機の負荷試験(同時N接続・1サイト暴走時の他サイト影響・crash復帰)を通してから次へ。

## 8. 方針適合

- REST新設なし。管理は Connect、サービス間は gRPC(`API-POLICY.md`)。例外(ACME・ヘルスチェック・静的配信)のみ素のHTTP。
- JSONは RS-JSON(`from_str_secure`で上限付き読込)。
- VPS本番は今回触らない。実装は本仕様の確認後、まずローカル(WSL)で検証→カナリア→差し替えの既存手順。

## 9. 未決事項(ユーザーに決めてほしいこと)

1. ~~分離の強さ~~ → **決定済み**: 設定で切替(§5.1)。未決は `tenant` 用バックエンドの選定(nspawn/bubblewrap/Firecracker)と着手時期。
2. サイト別の `spawn_blocking` 専用プールを持つか(公平性は上がるがスレッド増)。
3. Windows/macOSのローカル運用での分離(uid/cgroup相当が無い)をどこまで揃えるか。PC単体では「警告つきで分離なし」を既定とする案。
4. **決定済み(2026-11-09)**: `sites.toml` を新設する。既存 `domains.toml`+`web_vhosts.toml` は読み取り互換を維持。
5. 本体の置き場所: **決定済み**(open-web-server=Apache、RPoem=Tomcat、各1回インストール)。ただしRPoemは本番で現在非稼働のため、P1は open-web-server 側のみで完結させる案。
6. 本番の機能セット(`admin-2fa,domain-watchdog,fastcgi-client,ddns,sftp`)に影響しない形で段階導入すること(ビルド時のfeature指定漏れが過去に別物を生んだ)。

## 10. 参照

- `RPoem/docs/HYBRID_NETWORK_ARCHITECTURE.md` §0.9(第二のApache/Tomcat)
- `RPoem/crates/open-runo-appserver/`、`open-web-server/crates/open-web-server-gateway/src/tenant_router.rs`
- `open-web-server/domains.toml.example`、`web_vhosts.toml.example`
- `F:\runo\API-POLICY.md`

## 11. 実装状況

- **段階1(2026-11-09、`open-web-server` 作業ツリー、未コミット)**: `sites.rs`(`sites.toml`→既存テナント/vhost展開、`OPEN_WEB_SERVER_SITES_FILE`)、サイト別同時接続上限(超過は503+`Retry-After`、現状は process のみ・転送のレスポンスヘッダ受信まで枠を保持)、ポート自動割当(IDハッシュ・再起動後も同じ・固定/使用中を回避)。`sites.toml` 由来のテナントは `domains.toml` へ永続化しない。`isolation="tenant"` は拒否。既存の `cargo test -p open-web-server-gateway` 225件成功。段階2以降は未着手。
- **段階2の一部(2026-11-09、`RPoem/crates/open-runo-appserver`、`fleet` feature)**: `SiteFleet` — 全サイトの子プロセスを**1つのtokioタスク**で監督(従来は1サイト1監視スレッド)。tickは既存`Supervisor`を流用(crash-loop backoff・GaveUpの挙動は二重実装しない)、ヘルスチェックだけ`tokio::net`で全サイト並行。`add/remove/stop/restart/status/list`。テスト: 8サイト同時Healthy・stop/restartでPID変化・他サイト無影響・remove。既定ビルド(feature無し)の既存25件も成功。**未実施**: hyperによる転送(keep-alive/chunked/WebSocket)、`Supervisor`の別ユーザー起動・cgroup(段階3)、open-web-serverからの`SiteFleet`接続(`sites.toml`のprocessサイトを実際に起動する配線)、`/crash`でのcrash復帰の実機確認(テストはstop→restartで代替)。
