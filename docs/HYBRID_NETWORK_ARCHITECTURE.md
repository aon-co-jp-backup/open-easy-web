# Hybrid Network Architecture — Technical Rules / 技術ルールファイル

**Status:** v1.0 (2026-07-15) — §0.9 added: 第二のApache/Tomcat/React positioning declaration and phased roadmap (universal runtime host + React-compatible view layer); scope extended to open-easy-web. Previous: Draft v0.9 (2026-07-12) — §0.5 updated with the exact user-confirmed phrasing for the Poem/Tauri reproduction mission (including in-browser execution); fixed stale one-directional lead/mirror claims in open-runo/poem-cosmo-tauri READMEs (JP+EN); left-aligned the docs-links lists in open-runo/poem-cosmo-tauri READMEs (JP+EN, were a flowing middot-separated paragraph); merged with §0.8 next-session directive, §0.7 concrete ZFS+ACID implementation, §0.6 postponed-item closure log, §0.5 relationship correction, zero-data-loss mission, open-web-server audit findings, aruaru-db UPSERT fix, and JP+EN research rule

**Tooling honesty note:** this session has no general web-search tool (only `github.com`/`crates.io`/`npmjs.com`-class dev-infra domains are network-reachable). Requests to "Google search in Japanese and English, check GitHub, read blogs/SNS/X, confirm official doc URLs" for unresearched/unverified topics could not be executed literally in this session. Where verification was possible, it was done by reading this repo family's own source code directly (which caught several real stale-doc bugs — see §0.6) rather than external research. §0.8's directive to research before implementing the 4-layer transport stack stands for whichever future session has web-search available; if this constraint hasn't changed, say so explicitly rather than fabricating research findings.
**Scope:** `open-runo`, `poem-cosmo-tauri`, `open-web-server`, `open-easy-web`, `aruaru-db`, `open-raid-z`
**Mission:** Guaranteed delivery + guaranteed read/write for data that must never be lost — 3D online game paid items, online finance, online securities/brokerage. See §0.
**Portability:** This file is written to be dependency-free of any single repo. Copy it as-is into any project in the `aon-co-jp` family; only the "Per-Project Status" table needs updating.

> ⚠️ **Research disclosure**: This document was authored without live web access (no general web-search tool was available in this session — only `github.com`, `crates.io`, `npmjs.com` and similar dev-infra domains were reachable). Claims about "2026-07 state of the art" reflect the author's trained knowledge (cutoff ~2026-01) plus inspection of this codebase, not a fresh literature/web survey. Treat performance claims and library version numbers as **to be verified** before being treated as fact — flag them in review rather than citing them externally.

---

## 0. Mission (使命) — Zero Data Loss for High-Stakes Transactions

This stack's reason for existing, stated plainly so it isn't diluted by later feature work:

> **Deliver and persist data that must never be lost, over a network that must never silently fail, for use cases where "eventually consistent" or "probably arrived" is not an acceptable answer.**

Concrete target domains:
- **3D online game paid items** (課金アイテム) — a purchase or item-transfer that silently fails or duplicates is a real financial loss for a real user, not a cosmetic bug.
- **Online finance / online brokerage (オンライン証券)** — order execution, balance updates, and trade confirmations must be durable and exactly-once, even across network layer switches (L1↔L2↔L3) or mid-session failover.

This mission subsumes and reorganizes the goals in §1 — treat §1–§4 as *how* we achieve zero-loss delivery, not as separate, competing goals. Speed and the 4-layer transport story matter **only insofar as they never compromise** this guarantee. Any optimization that trades durability for latency must be opt-in, explicit, and off by default for these domains (see §4 rules, unchanged and still binding).

**Non-negotiable properties for money/asset-bearing data:**
1. **At-least-once delivery + exactly-once application** — the network layer may retry; the DB layer (`aruaru-db`, ACID) must de-duplicate via idempotency keys/transaction IDs so retries never double-charge or double-grant an item.
2. **Durable before acknowledged** — a client is only told "success" after the write is committed and (where `open-raid-z` is in the path) checksummed/persisted — not merely buffered in a fast transport layer.
3. **Layer-switch transparency** — if a session migrates from QUIC (L2) to TCP (L3) mid-transaction (see §1 table), the in-flight transaction must survive the switch or be safely retried, never silently dropped.
4. **Auditability** — every asset/financial write must be traceable end-to-end across the transport and storage layers for reconciliation and dispute resolution.

## 0.5 `open-runo` ⇔ `poem-cosmo-tauri` の関係と同期ルール(2026-07-12 確認、同日中に訂正)

> ⚠️ **訂正(同日中)**: 本節の初版は「共有クレートが完全一致 = 2プロジェクトの
> 規模・機能・役割はほぼ同じ」と誤って結論づけていた。これは誤り。
> 「中心技術(共有クレート)だけ同期を取る」ことと「2プロジェクトの規模・
> 機能・役割が同じである」ことは**全く別の話**であり、混同してはならない。

この2リポジトリは同一のクレート構成(`crates/open-runo-*`)を持つ姉妹関係にあり、
過去に別セッションで独立に開発が進んだ結果、コードが乖離することがある。

- **実装の先行方向に決まりはない。** 「必ずこちらが先行してあちらにミラーする」
  という一方向ルールは存在しない。どちらのリポジトリで先に変更・修正が
  行われても構わない。**乖離に気づいた側が、もう一方へその差分をミラーする**
  (2026-07-12 の `open-runo-db::federated_config` 移植はこの運用の実例)。
- **同期(ミラー)の対象は「共有クレート内の実コード」のみ。** README の
  ブランディング文言、`CLAUDE.md`/`PORTING.md`/`docs/HANDOFF.md` 等の
  セッション固有メモ・意思決定記録は意図的に同期対象外とする
  (各リポジトリの経緯・文脈が異なるため)。

### 規模・機能・役割は全く別物(重要、混同厳禁)

- **`open-runo` = GraphQL Federation プラットフォームという「製品」そのもの。**
  WunderGraph Cosmo の有料版機能(Federation・Schema Registry・SCIM・
  Persisted Queries・Cache/Backup・AI Routing・DUAL DATABASE・
  Versionless API 等)を OSS で提供することが役割。スコープは
  「Federation プラットフォームとして何を提供するか」に閉じている。
- **`poem-cosmo-tauri` = それに加えて、Poem(Web フレームワーク本体)と
  Tauri(デスクトップフレームワーク本体)そのものを一から自作・再現すると
  いう、全く別次元の役割を追加で背負っている。** これは「Federation
  プラットフォームの一機能」ではなく、「他社製フレームワーク2つを
  ゼロから作り直すR&Dプロジェクト」という、スケール感も目的も異なる
  仕事である。`open-runo` にはこの役割は無い。**Poemとブラウザ内実行
  機能搭載も含めたTauri両方共に、一から開発して完全互換で再現する。**
  (2026-07-12、ユーザー確定の表現として本文にそのまま記録)
- **現状、共有クレート(`crates/`・`apps/`)が両リポジトリで完全一致して
  いるのは、「2プロジェクトの規模・役割が同じだから」ではなく、
  「これまでの実装がその都度ミラーされてきたため、役割の違いがまだ
  実コードの分岐として表面化していないだけ」である。** 将来、
  `poem-cosmo-tauri` 側で Poem/Tauri 再現の作業が本格的に進めば、
  `open-runo` には無い独自コードが大量に増える見込みであり、それは
  正常な乖離であって「同期漏れ」ではない。
- **今後この2リポジトリを扱う際は、「共有クレートの中心技術部分だけを
  見比べて同期する」作業と、「それぞれのプロジェクトが本来目指す規模・
  役割の違いを評価する」作業を、常に別のものとして扱うこと。** 前者の
  完了(diffが無い)を根拠に後者(規模・役割の同等性)を結論づけては
  ならない。

## 0.6 Postponed-Item Closure Log (2026-07-12) — `poem-cosmo-tauri`

Per explicit user instruction to stop treating this repo's larger Poem/
Tauri-reimplementation mission as an excuse to defer real work, the
following previously-postponed/skipped Poem-parity and Cosmo-parity gaps
were closed in this session (see `poem-cosmo-tauri` commit history and
`docs/poem-parity.md`/`docs/cosmo-parity.md` for full detail):

- **gRPC server-streaming** (`grpc.health.v1.Health/Watch`) — closed the
  "no streaming" gap.
- **gRPC per-service `NOT_FOUND`** — `Check`/`Watch` no longer claim
  `SERVING` for a service name this server doesn't expose.
- **gRPC reflection** (`grpc.reflection.v1.ServerReflection`,
  `list_services` only) — closed the "no reflection" gap for the common
  service-discovery case (`grpcurl <addr> list`).
- **Non-multipart file attachment** (`POST /api/schemas/upload-raw`) —
  closed the "file attachment besides Multipart" gap.
- **Two stale-documentation bugs found and fixed**: `docs/cosmo-parity.md`
  had claimed OTLP export and MCP Server integration were still
  unimplemented; both were already fully implemented and tested. Also
  fixed a stale doc comment inside `mcp.rs` itself claiming resources/
  prompts weren't implemented, when they demonstrably were (tested).
  **Lesson**: verify current code state directly before trusting a
  parity doc's "not done yet" claim — docs drift out of date in both
  directions (overstating *and* understating completion), and both
  directions cause real problems if relied on uncritically.
- **EDFS** (Event-Driven Federated Subscriptions, Redis Pub/Sub only —
  Kafka/NATS not attempted) — bridges the existing in-process
  `broadcast::Sender<SchemaEvent>` across instances via Redis, so GraphQL
  Subscriptions work correctly in a load-balanced multi-instance
  deployment.
- **Cosmo Connect** (scoped to `grpc.health.v1.Health` only, not full
  dynamic `.proto`-driven schema composition) — a real gRPC client
  function plus a `grpcHealthCheck` GraphQL field, proving "gRPC service
  reachable from GraphQL" end to end over real network calls.

**Still genuinely deferred (environment-blocked, not skipped out of
laziness)**: DNS-01 ACME challenge (needs a real DNS provider API this
sandbox has no access to), macOS packaging (no macOS available in this
environment), Linux system-tray visual confirmation (WSLg has no tray
host panel — the binary itself runs fine), gRPC reflection's non-
`list_services` request kinds, and full Cosmo Connect schema composition
for services beyond `Health`. These are named explicitly here rather
than left as vague "future work" so the next session can tell at a
glance which gaps are genuinely blocked versus simply not yet attempted.

**Toolchain note**: none of the above could be compiled via this
session's sandboxed cargo (1.75) against the real workspace — a
pre-existing, already-documented constraint (this workspace's own
Cargo.lock pins `indexmap` 2.14.0, which requires the unstable
`edition2024` Cargo feature). Each change was instead verified by
extracting the new/changed code into an isolated, exact-version-pinned
standalone crate and running its tests there. Confirm with a real
toolchain/CI before treating any of this as fully closed.

## 0.7 ZFS互換 × ACID互換ハイブリッド — 具体的実装(2026-07-12, `aruaru-db`)

`open-web-server`/`open-runo`/`poem-cosmo-tauri`/`aruaru-db`/`open-raid-z`
の5リポジトリ全体で目指す「ZFS互換とACID互換のハイブリッド」を、まず
`aruaru-db`で具体的なコードとして実装した(§1の目標アーキテクチャの一部を
前倒しで着手)。

### 実装内容

`crates/aruaru-core/src/storage/mod.rs`(`PersistentStore`)に:

- **`compute_checksum(data) -> [u8; 32]`**: SHA-256。`open-raid-z`の
  `open_raid_z_core::checksum::compute_checksum`と**アルゴリズム・型とも
  完全同一**(バイト単位で相互検証可能——§0.5でいう「共有すべき中心技術」
  の実例)。
- **`__checksums`パーティション**: `save_row`で書き込みバイト列のチェック
  サムを必ず記録(ZFSの「全書き込みにチェックサムを付与」)。
- **`scan_table`での読み込み時検証**: 保存時チェックサムと再計算値が
  不一致なら`StorageError::ChecksumMismatch`を返す(黙って壊れたデータを
  返さない。ZFSの「全読み込みで検証」)。チェックサム未記録の既存データは
  スキップし後方互換を維持。
- **`scrub()`**: 全行のチェックサムを検証し破損箇所一覧を返す
  (`zpool scrub`相当。最初の不一致で打ち切らない)。

### なぜ「ハイブリッド」か(直交する2つの保証)

- **ACID**(既存): `BEGIN`/`COMMIT`/`ROLLBACK`とGit-on-SQLコミットが、
  「正しい順序で確定した」ことを保証する。
- **ZFS互換チェックサム**(新規): 「確定後に保存バイトが破損していない」
  ことを保証する。

どちらか一方では「正しい順序で、かつ壊れていないデータが確定している」
とは言えない——両方揃って初めて、§0のゼロロス使命(課金アイテム・金融
データを紛失しない)に必要な完全性が得られる。

### 他リポジトリへの展開(次のステップ)

- `open-web-server-ledger`のWAL書き込みステップに同じチェックサム層を
  接続する(§1の優先順位6番)。
- `open-runo`/`poem-cosmo-tauri`側の`open-runo-db`クレート(federated
  バックエンド)にも同様のチェックサム検証を追加できるか検討する。
- `open-raid-z`側は逆に、`aruaru-db`のGit-on-SQLコミットIDをZFS
  スナップショットのタグとして扱えないか(README-Japan.mdで「次回新規
  開発予定」と記載されていたアイデア)を次回検討する。

### 検証状況

`compute_checksum`自体は分離クレート(sha2のみ)で標準SHA-256テスト
ベクタと一致することを確認済み。`PersistentStore`本体(fjall統合部分)は
このセッションのサンドボックスでは検証できなかった——`fjall`自体がrustc
1.76+を要求し、このサンドボックスのrustc 1.75では(edition2024問題とは
別の、より根本的な制約として)ビルドすら開始できないため。既存の動作実績
あるパターンを踏襲した最小限の追加であることを目視レビューで確認したが、
実CI/実ツールチェーンでの確認を推奨する。

## 0.8 次回セッションへの指示(2026-07-12 ユーザー確定)

次回セッションでは、以下を統廃合・融合して実装すること:

1. **ZFS互換 × ACID互換ハイブリッドの継続・融合**: §0.7で`aruaru-db`に
   実装したチェックサム層(compute_checksum・`__checksums`パーティション・
   `scan_table`検証・`scrub()`)を土台に、他リポジトリへの展開を進める
   (§0.7末尾に記載した「他リポジトリへの展開」の3項目——
   `open-web-server-ledger`のWAL接続、`open-runo-db`federatedバックエンドへの
   適用、`open-raid-z`側でのGit-on-SQLコミットIDとZFSスナップショットの
   紐付け)。「ZFS互換」と「ACID互換」を別々の機能として作るのではなく、
   常に**統合された1つの書き込み・読み込みパスとして融合**すること。

2. **4層4重の最先端通信技術をGoogle検索・GitHub検索で調査した上で実装**:
   §4のルール(「Web検索・GitHub調査は日本語と英語の両方で行う」)に従い、
   2026年7月時点の最先端の通信技術(QUIC/HTTP3・MPTCP/MPQUIC・SCTP等)を
   実際に検索・分析した上で、4層(TCP-IP・UDP-IP・QUIC/MPQUIC・
   MPTCP/SCTP)×4重の通信スタックとして実装すること。調査せず記憶だけで
   実装しないこと(§4のルールをそのまま適用)。

3. **上記1と2を、§0のゼロロス使命(3Dオンラインゲーム課金アイテム・
   金融・証券・クレジットカードデータをネットワーク上で絶対に紛失しない)
   の実現手段として、単なる個別機能の寄せ集めではなく統廃合・融合した
   1つの技術として実装すること**。具体的には: 4層4重の通信技術で
   ハイスピード・ハイセキュリティにデータを届け、ZFS互換×ACID互換
   ハイブリッドの最先端DATABASE技術で読み書き・保存する、という
   一気通貫のパイプラインとして設計・実装すること。

4. 実装後は、本ドキュメント(開発方針＆開発環境ルールファイル)・
   10ケ国語README・他プロジェクトへのお引越し可能ファイル(本ドキュメントと
   `docs/i18n/hybrid-network/`一式)を最新の実装内容に合わせて更新し、
   関連リポジトリ全て(`open-web-server`・`open-runo`・`poem-cosmo-tauri`・
   `aruaru-db`・`open-raid-z`)へ読み書き・統合・pushすること。


## 0.9 v1.0 ポジショニング宣言 — 「第二のApache」「第二のTomcat」「第二のReact」(2026-07-15 ユーザー指示)

本節はユーザーの明示指示により追加された、エコシステム全体の対外的・対内的な
ポジショニング定義であり、§0の使命(ゼロデータロス)の上位互換の物語である。
§0を置き換えるものではなく、§0を達成した上で目指す完成形を示す。

### 0.9.1 役割対応表

| 既存世界の役割 | 本エコシステムでの担当 | 意味 |
|---|---|---|
| **第二のApache** (HTTPサーバ/リバースプロキシ/vhost) | `open-web-server` (+ 管理UIとして `open-easy-web`) | TenantRegistryマルチドメインルーティング + app_proxy。静的配信・TLS・vhost・テナント分離を担う入口層 |
| **第二のTomcat** (アプリケーションコンテナ) | `open-runo` / `poem-cosmo-tauri` | ただし**Javaに限定しない汎用ランタイムホスト**。§0.9.2の言語×フレームワーク行列を第一級サポート |
| **第二のReact** (UIコンポーネント/宣言的ビュー) | `open-runo` / `poem-cosmo-tauri` 内に新設するビュー層クレート | Reactの機能を**完全互換で一から** Rust+Poem 系技術で再実装・完全移植する(§0.5のPoem+Tauri完全互換再現ミッションの一部として遂行) |

### 0.9.2 汎用ランタイムホストの対応行列(第二のTomcat)

「ほとんどの有名言語+フレームワークに汎用対応」の具体定義。優先実装順:

1. **Rust + Poem** (native、最優先。自エコシステムの中心技術)
2. **Python + FastAPI** (uvicorn/ASGIプロセス管理 + reverse dispatch)
3. **PHP + Laravel** (PHP-FPM連携 — `open-easy-web-server`が既にPHP-FPM自動設定を持つため知見を共有)
4. **Ruby + Ruby on Rails** (Puma/Rackプロセス管理)
5. **Dart + Flutter** (Flutter Web静的ビルド配信 + Dartサーバプロセス)
6. 以降、Node.js等は同一のプロセス管理抽象(§0.9.3)の追加プロファイルとして拡張

### 0.9.3 実装アーキテクチャ(段階的、§0.8の方針を踏襲)

- **appserverクレート**(`open-runo-appserver`、姉妹リポジトリ間で§0.5同期規則に従いミラー):
  - `RuntimeProfile` — 言語×フレームワークごとの起動コマンド・ヘルスチェック・
    環境変数・ポート割当を宣言的に定義
  - `ProcessSupervisor` — 子プロセスのspawn/監視/再起動(crash-loop backoff付き)
  - `Dispatcher` — open-web-server の app_proxy から受けたリクエストを
    プロファイルに応じて upstream へ中継(将来は4層トランスポート§1に載せる)
- **view層クレート**(第二のReact、名称 `open-runo-view` 予定):
  - Phase 1: VDOM + 宣言的コンポーネントモデル + イベントハンドラ(Rust、wasm32ターゲット)
  - Phase 2: hooks相当(state/effect)、差分レンダラ
  - Phase 3: SSR(Poemハンドラ統合)+ hydration
  - `open-easy-web`(wasm-bindgen既採用)を最初の実戦投入先とする
- **DUAL DATABASE**: aruaru-db ⇔ PostgreSQL の二重書き込みは §0.8「四重化」の
  ①②として既定義。本ポジショニングでは変更なし。
- **VersionlessAPI / Git管理 / open-raid-z ACID+ZFS互換**: 既存クレート
  (`open-runo-versionless-api`、`open-runo-history`、open-raid-zチェックサム層)を
  そのまま土台とし、重複実装しない。

### 0.9.4 拘束条件(変更なし・再確認)

- §0のゼロデータロス保証(課金アイテム・金融・証券・不動産・クレジットカード)を
  損なう最適化は不可。
- §0.5の同期規則: 共有クレートは open-runo ⇔ poem-cosmo-tauri でミラーする。
  poem-cosmo-tauri のミッション(Poem+ブラウザ内実行機能搭載Tauriの完全互換再現)は
  open-runo より大きく、規模を理由に作業を先送りしない。
- 日英二言語での事前リサーチ規則は、web検索可能なセッションで実施すること
  (本セッションはdev-infraドメインのみ到達可能)。

## 1. Goal (目指すもの)

Build a **hybrid transport + storage stack** across the five projects so that a single logical request (e.g. a GraphQL federation query landing on `open-runo`) can, end-to-end:

1. Arrive over the fastest available transport for the client/network conditions (multi-layer transport negotiation).
2. Be served from `aruaru-db` with full ACID guarantees even when the underlying storage is distributed.
3. Persist to `open-raid-z` with ZFS-style integrity (checksums, copy-on-write, scrub) without giving up POSIX/Windows filesystem semantics.
4. Do all of this with security (mTLS/QUIC-native encryption) and speed (zero-copy where possible) as co-equal goals, not one traded for the other.

The "4層4重" framing means: don't pick one transport. Run a layered fallback/negotiation stack:

| Layer | Transport | Primary use case |
|---|---|---|
| L1 | UDP/IP raw + custom framing | LAN-local, lowest latency, trusted network segments |
| L2 | QUIC (HTTP/3) over UDP | WAN, NAT traversal, connection migration, mobile clients |
| L3 | TCP/IP (HTTP/2 or HTTP/1.1 fallback) | Legacy clients, environments where UDP is firewalled |
| L4 | Application-level multiplexing (GraphQL federation over any of the above) | Uniform API regardless of which lower layer was negotiated |

"4重" (fourfold redundancy) = each layer should degrade gracefully to the one below it, and the system should be able to detect and switch mid-session (e.g. QUIC connection migration, or TCP fallback if UDP is blocked mid-handshake).

## 2. Current State (今出来ているもの)

This section must be kept honest and specific — do not let it become aspirational. Update per project:

- **`open-runo`**: GraphQL federation gateway. Has `poem-parity.md`, `cosmo-parity.md`, `tauri-parity.md` tracking feature parity with Poem/Cosmo/Tauri references — meaning transport-layer work is already being scoped, but (as of last working session) UPSERT/SQL compatibility with `aruaru-db` is an open gap, and some crates were not yet pushed (non-compilable state flagged in earlier sessions).
- **`aruaru-db`**: Hybrid distributed DB with Git-on-SQL. Poem web framework integration confirmed well-optimized, minimal middleware overhead — this is a real, verified data point, not aspirational. **UPSERT gap closed (2026-07-12)**: parser now handles `INSERT ... ON CONFLICT (col) DO UPDATE SET ... / DO NOTHING`, including `EXCLUDED.col` references, wired into a new `upsert()` executor. Verified via an isolated 16-test suite (parser layer, since this sandbox's toolchain cannot compile the full crate — see rule on toolchain limitations below); 4 additional engine-level tests were written but not yet run in a real toolchain/CI — confirm those before treating this as fully closed.
- **`open-raid-z`**: Experimental Windows filesystem with ZFS-style features. `Pool::read_unaligned`/`write_unaligned` implemented. `orzctl migrate` subcommand exists with integration tests. FAT32 test coverage exists. Known constraint: `windows` crate v0.58 is `#![cfg(windows)]`-gated, so Windows-native types are unavailable when developing/testing on Linux — any hybrid-network code touching `open-raid-z` internals must account for this at CI time (feature-gate or mock).
- **`poem-cosmo-tauri`**: Referenced as a Tauri-based desktop counterpart; parity doc exists in `open-runo` (`tauri-parity.md`) but this repo's own transport work is new as of this session per the user.
- **`open-web-server`**: **Audited 2026-07-12.** Far more mature than the earlier "unaudited" placeholder suggested. Already implements:
  - A 4-layer defense-in-depth wire protocol (`open-web-server-wire`): L1 TLS 1.3 (rustls) → L2 mutual auth → L3 ChaCha20-Poly1305 AEAD application-layer payload encryption → L4 replay-guard (monotonic seq + timestamp bound into AEAD associated data, rejecting replayed ciphertext that AEAD alone wouldn't catch).
  - A parallel, orthogonal `udp_channel` module (added 2026-07-11) for **transport-path redundancy** — the same `MutationRequest` is fire-and-forth sent over UDP alongside the authoritative TCP path; UDP failure/timeout never affects the TCP-path commit. This is exactly the §1 L1/L2 relationship, already implemented, not just designed.
  - `open-web-server-core`: `IdempotencyKey`, `MutationRequest`/`MutationReceipt` domain types — every write requires an idempotency key by construction (a `CoreError::DuplicateKey` variant exists specifically to reject re-application).
  - `open-web-server-ledger`: the documented 3-hop commit pipeline — open-web-server (local WAL, fsync) → open-runo (Federation routing + audit log) → aruaru-db (Raft consensus, Git-on-SQL commit id). Client only gets "confirmed" after step 3's commit id returns.
  - `open-web-server-gateway`: hyper/tokio-based HTTP entrypoint (explicitly *not* Poem-dependent as of a 2026-07-10 stack pivot, while keeping API-shape compatibility with the earlier Poem implementation), with `/api/v1/items/grant` and `/api/v1/transactions/charge` handlers and an idempotency-key-required middleware gate.
  - **Conclusion: §0's mission is already substantially implemented here**, ahead of the other four repos. The remaining gap is wiring `aruaru-db`'s new UPSERT support (this session) into the ledger's aruaru-db-facing write path, and connecting `open-raid-z` checksums into the WAL step.

## 3. Where To Start (どこから手を付けるべきか)

Priority order, reasoning included so it can be re-argued later:

1. ~~Fix the `aruaru-db` UPSERT/SQL parser gap.~~ **Done (2026-07-12)** — see §2.
2. ~~Audit `open-web-server`.~~ **Done (2026-07-12)** — turned out to be the most mission-complete of the five repos; see §2.
3. **Confirm the two items above actually compile/pass in a real toolchain.** This session's sandbox could not build the full `aruaru-query` crate (edition2024 transitive-dependency issue, same class as `open-raid-z`'s documented constraint) — the parser fix was verified in isolation, and the engine-level tests are unverified. Do this before building anything on top.
4. **Wire `aruaru-db`'s new UPSERT support into `open-web-server-ledger`'s aruaru-db write path**, replacing whatever placeholder/plain-INSERT logic it currently uses for the final Raft-consensus commit step.
5. **Define the L1–L4 negotiation contract as a shared crate/interface**, consumed by `open-runo`'s gateway and `open-web-server` — `open-web-server-wire`'s 4-layer defense-in-depth stack and `udp_channel` are a strong starting reference implementation; don't reinvent it, extract/generalize it.
6. **Wire `open-raid-z` scrub/checksum hooks into `open-web-server-ledger`'s WAL step and `aruaru-db`'s write path** as an opt-in durability layer.
7. **Only then** build further QUIC/UDP fast-path work — `open-web-server` already has a working UDP redundant path; evaluate whether it needs QUIC specifically or whether the existing design already satisfies §0.

## 4. Technical Rules (ルール)

- **No transport-only "fast path" may bypass ACID guarantees.** If a low-latency path can't preserve durability semantics, it must be explicitly labeled (e.g. `--allow-eventual`) and off by default.
- **Money/asset-bearing writes (game item purchases, brokerage orders, balance changes) always use the zero-loss path from §0** — idempotency key required, ack only after durable commit, no exceptions for "just this once, for latency."
- **Research as part of normal maintenance, not a one-off.** When implementing or reviewing changes in this stack, actively look things up — search the web (e.g. Google) for current protocol specs, CVEs, and library changelogs; check GitHub (issues, releases, source) for the actual current behavior of dependencies (`rustls`, `chacha20poly1305`, `datafusion`, `fjall`, etc.) — rather than relying purely on trained/remembered knowledge, which can be stale or wrong. **Run web searches in both Japanese and English** — search results, advisories, and community discussion differ by language (e.g. Japanese-language blog posts/Qiita/Zenn articles on a crate's quirks, or CVE writeups that only appear in English-language security trackers), so a single-language search risks missing relevant findings. This is especially important for the crypto/replay-guard/consensus code in `open-web-server-wire` and `aruaru-db`, where a wrong assumption about a library's behavior is a security or durability bug, not a cosmetic one. Cite what was actually checked (a URL, an issue number, a changelog entry, and which language it was found in) rather than asserting "current best practice" from memory alone.
- **Toolchain limitations must be recorded, not silently worked around forever.** If a sandbox/CI environment can't build a crate (e.g. the recurring edition2024-transitive-dependency issue affecting both `open-raid-z` and `aruaru-db`'s `datafusion`/`arrow`/`fjall` chains), say so explicitly in the commit and in this doc, and flag any tests as "written but unverified" until a real toolchain confirms them — don't claim something is fixed until it has actually compiled and passed.
- **Every cross-project integration must have a feature flag.** Follow the `open-raid-z` precedent (`gpu`, `winfsp_backend`, `foreign_fs_fat`/`foreign_fs_exfat`) — don't force one project's platform constraints onto another's default build.
- **Bilingual docs (JP/EN minimum) for anything user-facing**, consistent with existing project convention.
- **Large, meaningful commits** over many small ones, consistent with established workflow preference.
- **Performance claims must cite a benchmark run in this repo**, not general industry claims about QUIC/io_uring/etc. "Poem is well-optimized" was earned by actually profiling it — hold new claims to the same bar.
- **Security and speed are both required, not traded off.** Any PR that improves latency by weakening the encryption/auth story should be rejected or gated.

## 5. Open Questions For Next Session

- Has `open-web-server` been audited yet?
- Is there a shared crate for transport negotiation, or is each project doing its own?
- What's the actual current UPSERT-parser fix status in `aruaru-db`?
- Do we have real benchmark numbers for QUIC vs TCP in this stack, or only from general literature (which should not be cited as fact per §1 disclosure)?
- Which project owns the idempotency-key / transaction-ID scheme for §0 zero-loss guarantees — `aruaru-db`, or a new shared crate? **Partially answered**: `open-web-server-core::IdempotencyKey` already exists and propagates through the 3-hop pipeline; `aruaru-db::execute_idempotent` also already exists independently. These two schemes need to be reconciled/unified rather than left as two parallel idempotency systems.
- Has the `aruaru-db` UPSERT fix and its engine-level tests actually been compiled and run in a real toolchain (not this session's sandbox)?
- Should `open-runo`'s federation layer generate the `EXCLUDED.col`-style UPSERT SQL that `aruaru-db` now understands, or does it already, making this fix immediately usable end-to-end?
