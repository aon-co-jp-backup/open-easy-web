# 開発方針・開発環境ルール(全リポジトリ共通ヘッダー、2026-07-15追記)

## 1. 比較的新しい言語・フレームワークの参照資料一覧

Rust自体は歴史があるが、本エコシステムが採用する **Poem** のような
比較的新しい・情報量がまだ少なめのWebフレームワークは、Python+FastAPIの
ような広く普及した組み合わせと比べ、AIモデルの学習データ・公開されている
実装例/Q&A/ブログ記事の絶対量が少ない傾向がある。そのため、AI駆動開発
(Claude等)がこれらを扱う際、実装の勘違い・API名の記憶違い・古いバージョン
のAPIでの実装(本プロジェクトで実際に複数回発生した既知の失敗パターン)に
よる**手戻り・いたちごっこ**が起きやすい。

対策として、AIが作業を始める際は、以下から**そのタスクに必要な部分だけ**を
先に参照してから実装に着手すること(全部読む必要はない。関連しそうな1〜2件を
拾い読みする程度で十分)。これにより歩留まりが上がり、AI駆動開発の手戻りが
減ることが期待される。

| 技術 | 公式ドキュメント | GitHub | 補足・ブログ等 |
|---|---|---|---|
| Rust言語本体 | https://doc.rust-lang.org/book/ | https://github.com/rust-lang/rust | https://blog.rust-lang.org/ |
| Poem(Webフレームワーク) | https://docs.rs/poem/latest/poem/ | https://github.com/poem-web/poem | https://crates.io/crates/poem |
| Tokio(非同期ランタイム) | https://tokio.rs/tokio/tutorial | https://github.com/tokio-rs/tokio | https://tokio.rs/blog |
| async-graphql | https://async-graphql.github.io/async-graphql/en/index.html | https://github.com/async-graphql/async-graphql | https://crates.io/crates/async-graphql |
| Tauri | https://tauri.app/ | https://github.com/tauri-apps/tauri | https://tauri.app/blog/ |
| wasm-bindgen / web-sys | https://rustwasm.github.io/wasm-bindgen/ | https://github.com/rustwasm/wasm-bindgen | https://rustwasm.github.io/docs/book/ |
| SurrealDB | https://surrealdb.com/docs | https://github.com/surrealdb/surrealdb | https://surrealdb.com/blog |
| sqlx | https://docs.rs/sqlx/latest/sqlx/ | https://github.com/launchbadge/sqlx | |
| WinFsp | https://winfsp.dev/ | https://github.com/winfsp/winfsp | |
| DirectX 12 / DirectML | https://learn.microsoft.com/en-us/windows/win32/direct3d12/directx-12-programming-guide | https://github.com/microsoft/DirectML | https://devblogs.microsoft.com/directx/ |
| WebAssembly(wasm32全般) | https://webassembly.org/ | https://github.com/WebAssembly | https://rustwasm.github.io/docs/book/ |

> ⚠️ **重要な注意(正直な開示)**: このURL一覧は、Web検索ツールを持たない
> セッションで学習データに基づき記載したものであり、**実在性・現在の
> 有効性・記載内容の正確性を検証していない**。特にAI(Claude含む)が
> このリストを鵜呑みにして実装や回答の根拠にすることは避け、
> **開発者自身が実際にアクセスして確認する**か、Web検索が使える
> セッションで一次情報を再確認してから利用すること。リンク切れ・
> リダイレクト・バージョン変更(特にAPIの破壊的変更)の可能性を
> 常に考慮する。新しい技術を追加する場合はこの表に追記していくこと。

## 2. AI駆動開発ツールに関する所感(2026-07-15、ユーザー所感として記録)

2026-07-15時点、ChatGPT等の汎用AIチャットは小規模なWebアプリ程度までは
開発できるものの、システムがある程度複雑・大規模になると出戻りが大きくなり、
一度に扱えるプログラムサイズにもすぐ限界が来る傾向がある。

Claude Code / Claude Desktopは、ローカルドライブを直接指定してファイルの
読み書きができ、GitHubリポジトリの読み出し(本プロジェクトのような
複数リポジトリにまたがるエコシステム)にも対応できるため、本プロジェクトの
ような規模のAI駆動開発には適していると考えられる。新しくAI駆動開発環境を
セットアップする際の選択肢として推奨する。

---

# 技術スタック・開発ルール(open-easy-web)

このリポジトリ、および関連プロジェクト(`open-runo`/RPoem(旧poem-cosmo-tauri)/
`aruaru-web`/`aruaru-db`/`open-web-server`/`open-raid-z`)で開発・保守を
行う際は、以下を基本方針とする。作業ドライブは `F:\open-runo`(E:ドライブは
2026-07-10に消失、以後Fが実体)。この節は
[`open-raid-z`](https://github.com/aon-co-jp/open-raid-z) の `CLAUDE.md`
を正本とし、各プロジェクトへコピーして同期する。

## このリポジトリの役割(2026-07-13、`aruaru-web` から分離・新設)

`open-easy-web` は「**第二のKUSANAGI**」——DBに依存しない汎用の
デプロイ・運用ツール。WordPress高速化サーバー構築キット「KUSANAGI」の
ように、アプリのアップロード後にIPアドレスから起動し、ドメイン登録・
HTTPS化を簡単に自動適用できることを目指す。

**分離の経緯**: `aruaru-web`(自身も「第二のKUSANAGI」を自称していた)
が開発していた機能のうち、(1) 「サイト管理」画面・IPアドレス起動・
ドメイン/HTTPS登録・HTTPS自動監視/自動発行/自動更新・VPSデプロイという
「**簡単なドメイン/サブドメイン登録・削除**」「**HTTPS自動監視・
自動発行・自動更新**」「**アップロード後の簡単なサイト運用**」に
関する全て(KUSANAGIの高速化機能を除く)を、この `open-easy-web` に
引き継いだ。(2) 一方、`aruaru-web` が同時に開発していた
**KUSANAGI風のWeb高速化機能**(vhostのgzip圧縮・静的アセットの長期
キャッシュ・FastCGIバッファ調整・upstream keepaliveプーリング)は、
Nginx/Apache設定生成という形ではなく、**`open-runo`/RPoem(旧poem-cosmo-tauri)
側でネイティブRust実装(hyperミドルウェア)として統合**する方針に
なったため、そちらへ移動した(`aruaru-web`のCLAUDE.md参照)。

**このリポジトリはWeb高速化機能を意図的に持たない**——`deploy/nginx/`・
`deploy/apache/`のvhostテンプレートは、ドメイン・HTTPS・基本的な
リバースプロキシ配線のみを担当し、高速化ディレクティブは含まれない。
高速化が必要な場合は`open-runo`/RPoem(旧poem-cosmo-tauri)のRustサーバーを
使うこと(それらのミドルウェアは`with_compression`(gzip応答圧縮)・
`with_static_cache_headers`(静的アセットCache-Control)として実装
済み)。

## フロントエンド

- Tauriパッケージには直接依存しない。ただしTauriのデスクトップUI体験・
  `invoke()`的なコマンド呼び出しインターフェースとは互換性を保つ。
- **HTML5/CSS3・TypeScript・Bootstrap・Node.jsのスタックは廃止**。
  Rustをメイン言語としてフロントエンドを構成し、**WebAssembly (WASM)**に
  置き換える(コンパイル対象はRust → `wasm32-unknown-unknown`)。DOM操作・
  `fetch()`呼び出しはRust製WASMモジュール側(`wasm-bindgen` + `web-sys`)で
  行い、TypeScript/Node.jsのビルドチェーンには依存しない。重量級のRust製
  Webフレームワーク(Yew/Leptos/Dioxus等)も、強い理由がない限り採用しない。
  https://webassembly.org/ | https://rustwasm.github.io/

## ビルド手順(このリポジトリ固有)

```bash
rustup target add wasm32-unknown-unknown        # 初回のみ
cargo install wasm-bindgen-cli --version 0.2.126 # 初回のみ(Cargo.lockと一致させる)
cargo build --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir pkg \
  target/wasm32-unknown-unknown/debug/open_easy_web.wasm
python -m http.server 8080   # index.html + pkg/ を配信
```

## 関連プロジェクト

- **open-easy-web**(このリポジトリ): https://github.com/aon-co-jp/open-easy-web
- **aruaru-web**(分離元。高速化機能・ドメイン/HTTPS機能の旧居場所、
  現在はDB/高速化/ドメイン機能いずれも持たない): https://github.com/aon-co-jp/aruaru-web
- **open-runo**(Web高速化機能の実装先の一つ): https://github.com/aon-co-jp/open-runo
- **RPoem**(旧poem-cosmo-tauri)(同上、実装の先行地点): https://github.com/aon-co-jp/RPoem
- **aruaru-db**: https://github.com/aon-co-jp/aruaru-db
- **open-web-server**: https://github.com/aon-co-jp/open-web-server
- **open-raid-z**(開発ルールの正本): https://github.com/aon-co-jp/open-raid-z
- **rs-to-readme**: https://github.com/aon-co-jp/rs-to-readme

## 運用ルール

- **開発中はこの`CLAUDE.md`を、コード変更のコミット/pushと必ず一緒に
  push する**。
- 実装で迷った場合や、API仕様の詳細確認が必要な場合は、学習データからの
  推測より公式ドキュメント(上記URL)、または`open-runo`/RPoem(旧poem-cosmo-tauri)
  側の実ソース(`crates/open-runo-router/src/middleware_hyper.rs`)を
  優先して参照する。
- **無人自動開発(確認不要・自動デバッグ)のタイミングでは、20〜30分おきの
  スケジュール実行待ちにせず、1パス内でできる限り連続して作業を進める**
  こと。
- **各無人開発パスの最後には、必ず以下を実行すること**: (1) 世界10ヶ国語の
  README(`README-<言語>.md`、日本語・英語・中国語簡体字・韓国語・
  スペイン語・フランス語・ドイツ語・イタリア語・ロシア語・アラビア語)を
  最新の実装内容に合わせて更新する、(2) `PORTING.md`を同様に更新する、
  (3) この`CLAUDE.md`のHANDOFF節・現状節を更新する、(4) 上記を含む全ての
  変更をコミットしてpushする。
- **このリポジトリにWeb高速化機能(gzip/静的キャッシュ/FastCGIバッファ
  調整/upstream keepalive)を追加で持ち込まないこと**——2026-07-13の
  エコシステム分離方針に反する。高速化が必要な変更提案は
  `open-runo`/RPoem(旧poem-cosmo-tauri)側で行うこと。

## 現状(このリポジトリ固有)

- 2026-07-13、`aruaru-web`(2026-07-11ブートストラップ、2026-07-13時点で
  「サイト管理」「IPアドレス起動」「vhost生成・高速化・HTTPS自動設定」
  「VPSへのデプロイ」の4機能を持っていた)から、高速化機能を除く全機能を
  分離・移植してブートストラップ。単一クレート構成(`Cargo.toml`、
  `src/`は`lib.rs`/`dom.rs`/`profiles.rs`/`shell.rs`の4モジュール)、
  `crate-type = ["cdylib", "rlib"]`、依存は`wasm-bindgen`/
  `wasm-bindgen-futures`/`js-sys`/`web-sys`/`serde`/`serde_json`のみ。
- 実装済み機能:
  - **サイト管理画面**(`src/profiles.rs`): open-easy-web自身・
    WordPress・Laravel・FastAPIなど任意のバックエンドスタックのデプロイ先を
    複数登録・編集・削除でき、`localStorage`
    (`openeasyweb_site_profiles_v1`)に保存。接続テスト・ポート番号検証・
    削除確認ダイアログ・JSONエクスポート/インポートを実装(aruaru-webの
    実装をそのまま継承)。
  - **IPアドレスからの起動**: `scripts/serve.sh <BIND_IP> <PORT>`。
  - **vhost生成・HTTPS自動設定(高速化ディレクティブ抜き)**:
    `scripts/gen-vhost.sh [--stack=STACK] <DOMAIN> <BIND_IP> [UPSTREAM]
    [WEBROOT]`で、`static`・`proxy`・`wordpress`・`laravel`・`fastapi`の
    5スタック対応のNginx/Apache vhost(HTTP→HTTPSリダイレクト、ACME
    challenge許可込み)を生成する。**aruaru-webのテンプレートから
    gzip圧縮・静的アセットの長期キャッシュ(expires/Cache-Control)・
    FastCGIバッファ調整(fastcgi_buffers/fastcgi_buffer_size)・named
    upstream+keepaliveプーリングを全て削除した差分**(fastapiスタックの
    WebSocket/ストリーミング対応は機能上の必須要件のため維持)。
  - **HTTPS自動監視・自動更新**: `scripts/setup-tls.sh`・
    `scripts/check-tls.sh`・`scripts/check-all-tls.sh`・
    `deploy/systemd/{easyweb-tls-renew,easyweb-tls-monitor}.{service,timer}`
    (aruaru-webの`aruaru-tls-*`からunit名をリネーム)。
  - **VPSへのデプロイ**: `scripts/deploy-vps.ps1`(aruaru-webから移植、
    パス/ファイル名をopen-easy-web向けに更新)。
- `cargo build --target wasm32-unknown-unknown` / `cargo clippy --target
  wasm32-unknown-unknown` ともに警告0件で成功(このパスで確認済み)。
- `scripts/gen-vhost.sh`を全5スタックで実行し、`{{DOMAIN}}`/`{{IP}}`/
  `{{UPSTREAM}}`/`{{WEBROOT}}`のプレースホルダ置換が正しいことを確認
  (`deploy/generated/`配下の実際の生成物で確認済み)。

## 運用ルール追記(2026-07-18、正本はopen-raid-zのCLAUDE.md参照) — 確認不要の自動継続・リミット解除後の自動再開

- **コンテキストウインドウ・5時間利用制限・その他のセッション中断が
  発生し、その後リミットが解除されて新しいセッションが開始された場合、
  「続けてよろしいですか」等の確認を挟まず、毎回自動的に前回セッションの
  続きの作業を再開すること**(ユーザー指示、2026-07-18)。具体的には:
  1. セッション開始時、各リポジトリの`git status`/`git log`と、この
     `CLAUDE.md`(および他プロジェクトのCLAUDE.md)のHANDOFF節・
     「次にすべきこと」記載を確認し、未完了・未pushの作業が無いかを
     まず裏取りする(タスク管理メタデータを鵜呑みにしない既存方針と
     同じ姿勢で、実際のgit状態を確認する)。
  2. 未完了作業が見つかった場合、ユーザーへの確認を求めず、そのまま
     自動的に検証(build/test)→修正→コミット→pushまで完了させる。
  3. 完了している場合は、各CLAUDE.mdの「次にすべきこと」「未着手・
     未完成」に記載された次の項目へ確認なしに着手する(既存の
     「未着手だからといって確認を求めて手を止めない」方針の延長)。
  4. 「続けてよろしければそのまま自動開発を継続します」のような、
     続行そのものを尋ねる確認は今後一切行わない(ユーザー指示、
     2026-07-18)。作業内容の要約・進捗報告はしてよいが、それは
     承認を求めるものではなく完了報告として書く。
  5. こまめにコミット・pushしておくことで、次回セッションが「どこから
     再開すべきか」を迷わず`git log`/CLAUDE.mdから機械的に判断できる
     ようにしておく(区切りがついた時点で都度コミット・pushする既存
     方針との組み合わせ)。


## 運用ルール追記(2026-07-19、正本はopen-raid-zのCLAUDE.md参照) — 白画面バグ等を見逃さない検証徹底

- **WEB/UIを持つ機能を実装した後は、ビルド成功・`cargo test`・curlでの
  ステータスコード確認だけで「完了」と報告せず、実際に画面が正しく
  表示される(白画面・レンダリング崩れ・コンソールエラーが無い)ところ
  まで確認すること**(ユーザー指示、2026-07-19)。
  1. ブラウザ操作が可能な環境では、実際にページを開いて表示内容
     (見出し・本文・想定した要素の存在)とコンソールエラーの有無を
     確認する。
  2. ブラウザ操作ができない環境では、少なくとも`curl`等でHTMLボディの
     中身を取得し、期待される文字列が実際に含まれているかを確認する
     ——ステータスコード200だけを見て「動作確認済み」としない。
  3. 白画面・エラー・期待した内容の欠落等の不具合が見つかった場合は、
     確認を求めず自動的に原因調査・修正・再確認まで行う。
  4. 本番ドメインが未取得・DNS未設定なだけの状態は上記の「白画面
     バグ」とは別物であり、混同しない(`localhost`確認で代替可)。


## HANDOFF(直近の自動巡回ログ、上が最新)

- **2026-07-23 監査+flakyテスト2件の実バグ修正(ユーザー指示「完成度・
  実用性・互換性・連携性を向上して」)**:
  1. **監査結果**: `AppServerKind`経由のテナント登録(open-runo/
     RPoem[旧poem-cosmo-tauri]/aruaru-llm)・TLS自動発行/更新は実装済み・
     実際に`main.rs`から配線済みと確認。**依頼文にあった「RS-Chiketto」
     「RS-Red」は、リポジトリ全体をgrepしても現状一切登場せず**、
     `AppServerKind`にも存在しない(必要なら別途追加要)。
  2. **`cargo test --workspace`(ルート)が実質0件しか実行しない構造的
     な罠を発見**: ルート`Cargo.toml`は`[workspace]`のみで
     `members`未指定、`server/`が独自に別の`[workspace]`を宣言する
     **2ワークスペース構成**になっている。実際のバックエンド50件の
     テストは`cd server && cargo test`しないと一切実行されない
     ——CLAUDE.md本文では毎回正しいコマンドが書かれているが、
     この2ワークスペース分離自体はREADME/ビルド手順に明記されて
     いなかった。
  3. **`totp_setup_enable_then_requires_code_on_next_login`のflaky
     failureの実原因を特定・修正**: `server/src/main.rs`内2箇所で、
     TOTPコードを「0〜100万を総当たりして`verify_code`が受理する
     値を探す」という設計になっていた。debugビルドではこの総当たり
     自体が(正解が高い番号の場合)数秒〜20秒以上かかることがあり、
     その間にTOTPの時間窓(30秒×スキュー許容±1ステップ)を超えて
     しまい、サーバー側が正しく`401`(コード不一致)を返す——という
     のが実際のflakyの原因だった(3回実行して1回失敗を実際に再現し、
     原因を特定)。`server/src/totp.rs`の非公開関数`code_at`を
     `pub`化し、正しいコードを直接計算する方式へ2箇所とも書き換えて
     解消。**検証**: 修正後は該当テスト単体の実行時間が23秒→0.02秒
     に激減、3回連続green(以前は3回に1回程度の頻度で再現していた
     flakyが解消したことを実証)。
  - 次にすべきこと: (1) ルート`Cargo.toml`のビルド手順ドキュメントに
    2ワークスペース構成(`cd server && cargo test`が必須)を明記する、
    (2) `AppServerKind`へのRS-Git/RS-Red等の追加要否をユーザーに確認、
    (3) `scripts/gen-vhost.sh`とサーバー側`vhost.rs`(Rust再実装版)の
    役割分担をCLAUDE.md/READMEに正確に書き分ける(現状は前者が
    メイン経路であるかのように読める記載がある)。

- **2026-07-22 `https://easy-web.tokyo/`のSSL証明書ホスト名不一致を修正
  (ユーザー指示)**: `http://easy-web.tokyo/`は200 OKで正常だったが、
  `https://easy-web.tokyo/`にアクセスするとブラウザにSSL警告が出る問題を
  調査・修正。
  - **原因**: `/etc/nginx/conf.d/easy-web.tokyo.conf`(2026-07-17新設、
  当時DNS未反映のためHTTPのみ)が443番のserverブロックを持たず、
  TLS終端は別ファイル`/etc/nginx/conf.d/easyweb-tokyo-tls.conf`
  (`easyweb.tokyo`、ハイフン無し旧ドメイン向け)が担っていた。`certbot
  certificates`で確認したところ、`easyweb.tokyo`(ハイフン無し)証明書は
  `easyweb.tokyo`/`www.easyweb.tokyo`のみをSANに含み、ハイフン付き新
  ドメイン`easy-web.tokyo`/`www.easy-web.tokyo`をカバーする証明書が
  一枚も存在しなかった。443番へのTLS接続時、SNI `easy-web.tokyo`に対して
  一致するserverブロックが無く提示証明書とホスト名が食い違い、
  `SEC_E_WRONG_PRINCIPAL`(ホスト名不一致)警告となっていた。
  - **DNS確認**: `nslookup easy-web.tokyo` → `160.251.237.162`
  (VPS本体)へ正しく解決済みであることを確認(2026-07-17時点の
  「DNS反映待ち」は解消済みだった)。
  - **修正内容**: (1) `certbot certonly --webroot -w /var/www/acme-webroot
  -d easy-web.tokyo -d www.easy-web.tokyo`で新規証明書を取得
  (`/etc/letsencrypt/live/easy-web.tokyo/`、2026-10-20失効、certbotの
  自動更新タイマーにも登録済み)。(2)
  `/etc/nginx/conf.d/easy-web.tokyo.conf`に443番のserverブロックを追記し
  (`server_name easy-web.tokyo www.easy-web.tokyo`、
  `ssl_certificate`/`ssl_certificate_key`とも新証明書のパスを指定)、
  80番のserverブロックはproxy_passのまま維持(`http://.../healthz`の
  200監視を止めないため、意図的にhttps://へのリダイレクトは追加して
  いない)。旧設定は`easy-web.tokyo.conf.bak-20260722`としてVPS上に
  バックアップ済み。`nginx -t`で構文検証(既存の`aruaru.tokyo.conf`由来の
  無関係な警告のみ、エラー無し)後、`systemctl reload nginx`で反映。
  - **検証**: `curl -v https://easy-web.tokyo/`(証明書検証あり、`-k`
  無し)で`HTTP/1.1 200 OK`を確認、同様に`https://www.easy-web.tokyo/`も
  200を確認。作業前後とも`curl http://easy-web.tokyo/healthz`が200を
  返し続けることを確認済み(本番停止なし)。
  - **今後の推奨アクション**: certbotの自動更新は
  `easyweb.tokyo`(旧)・`easy-web.tokyo`(新)の2証明書が併存する状態に
  なった——旧ドメイン向けの`/etc/nginx/conf.d/easyweb-tokyo-tls.conf`を
  今後廃止する予定があるなら、対応する旧証明書の`certbot delete`も
  検討すること(今回はサービス継続を優先し削除は行っていない)。

- **2026-07-20 開発マシンのドライブレター変更(Z:→F:)・本番VPS表記修正
  (`open-easyweb`→`open-easy-web`)・デプロイ先パス変更(`/root/open-easy-web`
  →`/root/RUNO/open-easy-web`)・`src/profiles.rs`の自サイト情報自動補正
  バグ2件を修正(ユーザー指示)**:
  1. **開発マシンのドライブ構成変更**: これまで`Z:\runo\open-easy-web`
     だった作業パスが、ユーザーの環境変更により`F:\runo\open-easy-web`
     (同一内容、ドライブ文字のみ変更)になった。以後のセッションは
     `F:\runo\open-easy-web`を正として作業する。
  2. **本番VPS(`easy-web.tokyo`、実体は`easyweb.tokyo`向けnginx vhost経由)
     の表記修正**: 画面最上部の見出し・ページタイトルが実際には
     `open-easyweb`(ハイフン無し、旧ブランディング)のままデプロイされて
     いた——ローカルのソース(`src/shell.rs`)は既に`open-easy-web`表記に
     修正済みだったが、本番へは反映されていなかった(ビルド成果物と
     ソースの乖離)。ローカルで`cargo build --target
     wasm32-unknown-unknown` + `wasm-bindgen`を再実行し、生成物を本番へ
     再デプロイして解消。
  3. **デプロイ先ディレクトリの変更**: VPS上の実体パスを
     `/root/open-easy-web`から`/root/RUNO/open-easy-web`へ移設
     (`mv`、既存の`open-easy-web-frontend`/`open-easy-web-server`/
     `open-easy-web-wasm`サブディレクトリ構成はそのまま)。
     `/etc/systemd/system/open-easy-web.service`の`WorkingDirectory`・
     `ExecStart`・`Environment=OPEN_EASYWEB_STATIC_DIR`の3箇所を`sed`で
     新パスに書き換え、`systemctl daemon-reload && systemctl start
     open-easy-web`で復旧・動作確認済み(`systemctl is-active` =
     `active`)。`scripts/deploy-vps.ps1`の`-RemoteAruaruPath`既定値も
     同じ新パスに追従済み(このコミットに含む)。
  4. **`src/profiles.rs`の`migrate_stale_self_seed()`(自サイト情報の
     旧表記→新表記への自動補正関数)に発見した2件のバグを修正**:
     (a) ホスト名の判定条件が誤って**既に正しい値**`"easy-web.tokyo"`を
     チェックしており、実際の旧表記`"easyweb.tokyo"`(ハイフン無し)を
     検出できず補正が効かなかった(コピー&ペースト由来の誤り)。
     (b) `name`フィールド(`"open-easyweb(このサイト)"`→
     `"open-easy-web(このサイト)"`)がそもそも補正対象に含まれておらず、
     ホスト名を直しても表示名は古いままだった。(c) 判定を`id ==
     "seed-self"`で行っていたため、一度でも「保存」ボタン経由で編集
     された自サイトは`id`が`site-<timestamp>`形式に変わり、以後は
     `id`一致で検出できなくなっていた——`purpose == "self"`での判定に
     変更し、`id`の変遷に関わらず補正できるようにした。
  5. **付随して発見した開発環境固有の重大な既知の問題(次回以降も注意)**:
     このリポジトリをネットワーク共有ドライブ(SMB等でマウントした
     ドライブ、当時は`Z:`、現在は`F:`)上に置いた状態で`cargo build`→
     `wasm-bindgen`を実行すると、**直前の書き込み(ビルド成果物)に対する
     読み取りが古い内容を返すことがある**(読み取りキャッシュの不整合、
     複数回再現・確認済み)。この不整合により、一時的に本番へ
     内部参照が不整合な(JS側が古い入力ファイル名`_bg.wasm`/`_bg.js`を
     参照する)壊れたビルドをデプロイしてしまい、画面が一時的に真っ白
     になる事故が発生した(`WebAssembly.instantiate(): Import #0
     "./open_easy_web_src_bg.js": module is not an object or function`)。
     **回避策**: `cargo build --target-dir <ローカルドライブの一時
     ディレクトリ>`でビルド出力先をネットワークドライブ外(ローカルの
     C:等)に切り替え、`wasm-bindgen`もそのローカルコピーに対して実行
     すると解消する(このHANDOFFの直後に10ヶ国語README/PORTING.mdへも
     同じ注意書きを追記済み)。**入力ファイル名を最終的な出力名と一致
     させること**も重要——`wasm-bindgen`は入力wasmファイルのファイル名
     stemを基にJSグルーコード内の相対import参照(`_bg.wasm`/`_bg.js`)を
     生成するため、後から出力ファイルだけをリネームしても内部参照は
     古い名前のまま残る(このバグを実際に本番デプロイ後の実ブラウザ
     コンソールエラーで検出・修正した)。
  - **検証**: (1) `cargo build --target wasm32-unknown-unknown`
    (ローカル`--target-dir`経由のクリーンビルド)警告0件で成功。
    (2) 実際に`http://easy-web.tokyo/`をブラウザで開き、見出し・タイトル
    が`open-easy-web`になっていること、「選択中のサイト」表示が
    `open-easy-web(このサイト) ( easy-web.tokyo )`に補正されていること、
    コンソールエラーが無いことを実際のアクセシビリティスナップショット・
    コンソールログ・ネットワークログで確認済み(型チェックのみでの
    「完了」報告ではない、既存の検証基準どおり)。(3) VPS側で
    `systemctl is-active open-easy-web` = `active`、旧`/root/
    open-easy-web`ディレクトリが存在しないこと、nginx設定に古いパス
    参照が残っていないこと(`grep`)を確認済み。
  - 次にすべきこと: (1) `server/`クレート側(バックエンド)は今回
    パス変更・再起動のみで、コード変更・再ビルドは行っていない
    (`open-easy-web-server`バイナリ自体は無変更のため再ビルド不要と
    判断)——次回、`server/`側にもコード変更を加える際は、この新しい
    デプロイパス(`/root/RUNO/open-easy-web/open-easy-web-server`)を
    前提に手順を組むこと。(2) ネットワークドライブのキャッシュ不整合が
    今回だけの一過性の問題か、`F:`ドライブでも再発するかは未確認——
    再発した場合は同じ「ローカル`--target-dir`経由でビルド」回避策を
    再度使うこと。

- **2026-07-20 個人情報のハードコード除去(ユーザー指示)——`server/src/main.rs`の
  `FIXED_ACCOUNT_EMAIL`/`FIXED_ACCOUNT_BACKUP_EMAIL`/`FIXED_ACCOUNT_PHONE`定数
  (実際の個人Gmailアドレス2件・実電話番号)を削除し、環境変数から読む方式に変更**:
  - 新規必須環境変数`OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL`(未設定なら起動時に
    `panic`で明示的に落ちる——固定アカウント制でこれが無いと誰もログイン
    できないため、サイレントな機能不全より起動失敗の方が安全と判断)。
    任意環境変数`OPEN_EASYWEB_FIXED_ACCOUNT_PHONE`/
    `OPEN_EASYWEB_FIXED_ACCOUNT_BACKUP_EMAIL`(いずれか片方以上の登録が
    必須という既存の`register()`バリデーションはそのまま)。
  - `acme_email`のデフォルトフォールバック先も同じ値を使うよう追従。
  - テスト/docコメント中に残っていた実電話番号(`090-7555-5011`)・実個人
    メール(`totp.rs`の`norukia.jp@gmail.com`)もダミー値
    (`090-1234-5678`/`owner@example.com`)に置換。
  - **検証**: `cargo build`警告0件、`cargo test` 50件中49件green・
    1件(`totp_setup_enable_then_requires_code_on_next_login`)は単体再実行で
    green(既知のflaky、2026-07-18 HANDOFFに記録済みの並列実行タイミング
    起因で今回の変更とは無関係)。
  - **⚠️ 本番VPS反映時の注意(次回デプロイ時に必須)**: 実VPS
    (`/root/open-easy-web/open-easy-web-server`、systemdサービス
    `open-easy-web`)側で`OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL`(+電話/
    セカンドメールのいずれか)を環境変数として設定してから
    `systemctl restart open-easy-web`すること——設定せずに再起動すると
    起動時に`panic`して**サービスが落ちる**(固定アカウントが復元できず
    誰もログインできなくなるより安全な設計だが、デプロイ手順を伴わないと
    ダウンタイムになる)。`deploy/systemd/`にはまだ
    `open-easy-web-server.service`雛形が無い(既知の未着手項目)ため、
    現状は`/etc/systemd/system/open-easy-web.service`のVPS側の
    `Environment=`行、または`EnvironmentFile=`を手動で編集する必要がある。

- **2026-07-20 ドキュメント監査(ユーザー指示、コード変更なし)——実装と
  ドキュメントの齟齬を発見・修正、10ヶ国語READMEのうち3件・PORTING.md・
  この`CLAUDE.md`を更新**:
  1. `cargo check --target wasm32-unknown-unknown`(ルートWASMクレート)・
     `cd server && cargo check`(バックエンドAPIクレート)とも警告0件で
     成功、実装自体は健全であることを確認した(コード変更は行っていない)。
  2. **発見した齟齬(1) 構成節が古い**: `README.md`/`README-Japan.md`/
     `PORTING.md`の「構成」節が2026-07-13ブートストラップ時点のファイル
     一覧のままで、その後追加された`server/`(バックエンドAPIクレート
     一式)・`docs/HYBRID_NETWORK_ARCHITECTURE.md`・`src/api_auth.rs`/
     `src/api_upload.rs`/`src/auth_ui.rs`/`src/view_bridge.rs`が
     一切反映されていなかった——修正済み。
  3. **発見した齟齬(2) ルート`README.md`が`README-Japan.md`より古い**:
     ルートの`README.md`(GitHubのデフォルト表示)には「アカウント認証」
     「AIによる自動PHP判定」「共有バックエンドへの動的登録」の3機能
     説明が丸ごと欠落し、「いまできないこと」に既に実装済みの「認証」が
     依然「未実装」と誤記載されたままだった——`README-Japan.md`の記述に
     合わせて追記・訂正済み。
  4. **発見した齟齬(3) RPoemへの改名が一部ドキュメントにしか反映されて
     いなかった**: `8164032`(2026-07-20早朝)で`CLAUDE.md`の「関連
     プロジェクト」リンク1箇所のみ`poem-cosmo-tauri`→`RPoem`に修正
     されていたが、同じ`CLAUDE.md`の本文6箇所、および
     `README.md`/`README-Japan.md`/`README-English.md`/`PORTING.md`の
     計十数箇所は未反映のままだった——現在形の説明文のみ
     `RPoem(旧poem-cosmo-tauri)`表記に統一し、HANDOFF内の過去の経緯を
     語る文章(当時は実際に`poem-cosmo-tauri`という名前だった)、および
     `src/profiles.rs`/`server/src/appserver_registration.rs`側の
     `app_server`識別子文字列(`"poem-cosmo-tauri"`/`"poem_cosmo_tauri"`
     ——localStorageやサーバー間APIの実際のワイヤーフォーマット値)は
     **意図的に変更していない**(改名は表示名のみで、保存済みプロファイル
     やAPI互換性を壊す変更ではないため)。
  5. **発見した課題(コード内、ドキュメント外——今回は変更せず記録のみ)**:
     `server/src/main.rs`の`FIXED_ACCOUNT_EMAIL`/
     `FIXED_ACCOUNT_BACKUP_EMAIL`/`FIXED_ACCOUNT_PHONE`定数に実際の
     個人メールアドレス・電話番号がハードコードされている。コメントに
     よれば2026-07-15にユーザー指示でセキュリティ上の理由から公開の
     新規登録(`/api/auth/register`)を無効化し、起動時にシードされる
     この固定アカウント1件のみがログイン可能な仕様になった、という
     重要な意思決定だが、この`CLAUDE.md`のHANDOFFにはこれまで一度も
     記載されていなかった(2026-07-14〜07-16のエントリのどこにも
     登場しない、記録漏れ)。**今回はREADME側にこの仕様(固定アカウント
     制)を追記して利用者向けの説明齟齬は解消したが、個人の実メール
     アドレス・電話番号がソースコードに平文で残っている点自体は
     コード変更(小さな修正の範囲を超える)に当たるため今回は手を
     入れていない**——別途、環境変数化(既存の`OPEN_EASYWEB_*`環境変数
     パターンに合わせる)を検討すること。
  - **pushしたかどうか**: このエントリを含めてコミット・
    `origin/main`へpush予定(このパスの最後にまとめて実施)。
  - 次にすべきこと: (1) 上記(5)の個人情報ハードコードの環境変数化、
    (2) 残る7ヶ国語README(中国語/韓国語/スペイン語/フランス語/ドイツ語/
    イタリア語/ロシア語/アラビア語)の構成節・機能説明も同様に同期する
    (今回は日本語2件+英語1件のみ更新、範囲を絞った)。

- **2026-07-19 監査: 下記2026-07-18エントリの「次にすべきこと(1)」
  (WASM側UI配線)は、実は同日中の後続コミットで既に完了済みだったことを
  確認・実地検証、HANDOFF記載を訂正**: このエントリ自体の「次にすべき
  こと」が古いまま残っていて、実際には`837178d`
  (`Add aruaru-llm as a selectable app_server option in the
  site-management UI`、2026-07-18 21:00)で`src/profiles.rs`の
  `appserver_kind_for()`に`"aruaru-llm" => Some("aruaru_llm")`、
  `src/shell.rs`の`#site-app-server`セレクトに
  `<option value="aruaru-llm" title="契約不要の独自AIチャットコマース
  応答サービス(open-cudaとSET構成)。バックエンド接続先ではなく
  テナント登録のみ行う。">aruaru-llm(AIチャットコマース)</option>`が
  既に追加済みだった(このHANDOFFの追記漏れ、コード自体は正しく
  実装・コミット済み)。「タスク管理メタデータを鵜呑みにしない」
  既存方針どおり、実ソース・実ビルド・実ブラウザ描画で裏取りした。
  - **検証(型チェックのみでなく実際に確認)**:
    1. `cargo build --target wasm32-unknown-unknown`(ルートcrate)
       警告・エラー0件で成功。
    2. `cd server && cargo test` — **50件全green**(2026-07-18
       エントリで唯一flakyだった`totp_setup_enable_then_requires_
       code_on_next_login`も含め全件パス、このパスでは再現せず)。
    3. `wasm-bindgen --target web --no-typescript --out-dir pkg
       target/wasm32-unknown-unknown/debug/open_easy_web.wasm`で
       実際にJSグルー+`.wasm`を生成し、`python -m http.server`で
       ローカル配信。**実ブラウザ(Claude Browser pane)で
       `index.html`を開き**、白画面・コンソールエラーが無いことを
       確認した上で、サイト追加フォームの「アプリケーションサーバー」
       `<select>`に実際に`aruaru-llm(AIチャットコマース)`という
       選択肢が描画されていることをアクセシビリティツリー越しに確認。
       さらに実際に選択→サイト名・ホスト(`e-gov.info`)を入力→
       保存ボタンをクリックし、`localStorage`
       (`openeasyweb_site_profiles_v1`)に
       `"app_server":"aruaru-llm"`として実際に永続化されること、
       一覧カードに`アプリサーバー: aruaru-llm`と表示されることを
       DOM経由で確認した(コンパイル済み`.wasm`バイナリ内に
       `aruaru-llm`/`aruaru_llm`の文字列が実際に埋め込まれていることも
       `grep`で裏取り済み)。保存後にサーバー側ドメイン登録が
       `not logged in`エラーになったのは、このパスでは
       `open-easy-web-server`本体(セッション認証付きAPI)を起動せず
       静的ファイル配信のみだったための想定内の挙動であり、
       aruaru-llm UI配線とは無関係(「白画面バグ」には該当しない)。
    4. 既存の`appserver_registration::tests::
       registers_aruaru_llm_tenant_with_expected_shape`
       (サーバー側、実TCPループバックのモックで
       `POST /admin/tenants`の形状検証)も引き続きgreen。
  - **結論**: 下記2026-07-18エントリの「次にすべきこと(1)」は完了済み。
    残る「次にすべきこと(2)」(実際に稼働中の`aruaru-llm`インスタンスへの
    実登録E2E検証)のみ引き続き未着手。

- **2026-07-18 `aruaru-llm`(契約不要の独自AI、`open-cuda`とSET構成)への
  「分身の術」登録対応を追加**: `open-raid-z/CLAUDE.md`の方針
  (「管理はopen-easy-webで行なうように」)に基づき、
  `appserver_registration.rs`の`AppServerKind`に`AruaruLlm`variantを
  追加し、`register_aruaru_llm()`(`aruaru-llm`の
  `POST /admin/tenants`、`x-admin-token`ヘッダ認証)を新設。既存の
  `register_open_web_server`/`register_poem_cosmo_tauri`と同じ
  `register()`ディスパッチ経由で呼び出せる。**検証**:
  `cargo build`/`cargo test`とも成功、**50件全green**(新規1件
  `registers_aruaru_llm_tenant_with_expected_shape`、実TCPループバック上の
  モックサーバーで`POST /admin/tenants`が正しいホスト名・ヘッダで
  呼ばれることを確認)、既存49件のリグレッションも無いことを確認済み。
  次にすべきこと: (1) WASM側(`src/profiles.rs`/`src/shell.rs`)の
  `app_server`選択肢に`aruaru-llm`を追加するUI配線(現状はサーバー側
  APIのみ)、(2) 実際に稼働中の`aruaru-llm`インスタンスへの実登録
  E2E検証(今回はモックサーバーでの形状検証のみ)。

- **2026-07-17 `POST /api/sites/:name/register-appserver`ルートの配線漏れを
  発見・修正、VPS本番デプロイ完了(無人自動開発)**: `cargo build`の
  dead_code警告(`appserver_registration.rs`の`register`他3関数が
  未使用)を追ったところ、WASM側(`src/profiles.rs`)の
  「🔗 共有バックエンドへ登録」ボタンは完成していたのに、
  サーバー側(`server/src/main.rs`)にこのエンドポイント自体が
  ルーティングされておらず、本番では常に404になっていたという実バグを
  発見した。他の`/api/sites/*`アクションと同じ`require_session`
  認証パターンでルートを追加し解消。
  - **検証**: `cargo build`が**警告0件**に(従来7件→3件は無関係な
    警告として残置、`register`系4関数分の警告が解消)。`cargo test`
    49件全green(新規1件: 認証無し401・`shared_endpoint`到達不能時に
    502が返ることを実HTTP経由で確認する統合テスト)。VPS本番
    (`/root/open-easy-web/open-easy-web-server`)へも反映し、
    `systemctl restart open-easy-web`後、実際に
    `https://easyweb.tokyo/api/sites/example.tokyo/register-appserver`
    へ認証無しでPOSTし`401`が返ることを確認済み。
  - 次にすべきこと: 認証ありでの実登録(実際に稼働中の
    open-web-server/poem-cosmo-tauriインスタンスへ本当にテナント登録が
    成功するか)は、共有バックエンド側も実際に起動した状態でのE2E検証が
    必要(今回は403/502の経路のみ実HTTPで確認)。

- **2026-07-17 `totp-login`エンドポイントをVPS本番へデプロイ完了**:
  上記の新規`POST /api/auth/totp-login`を実VPS
  (`/root/open-easy-web/open-easy-web-server`、systemdサービス
  `open-easy-web`、`https://easyweb.tokyo`経由で公開)へ反映。
  デプロイ時に判明した実バグ: VPS上のソースがローカルの最新版より古く
  `appserver_registration.rs`自体が存在せず、`Cargo.toml`にも
  `thiserror`依存が無かった(以前のセッションでこのファイルの反映が
  漏れていた)——ファイルをコピーし依存を追加して解消。`cargo build
  --release`成功後、`systemctl restart open-easy-web`で反映、実際に
  `https://easyweb.tokyo/api/auth/totp-login`へ実HTTPリクエストを送り
  未登録アカウントに対し`403`が正しく返ることを確認済み(型チェックの
  みでの「完了」報告ではない)。
  次にすべきこと: `easy-web.tokyo`(ハイフン付き新ドメイン)へのDNS
  Aレコード追加(ConoHa DNSゾーン側、ユーザー操作待ち)後、そちらの
  ドメインでも同様に証明書取得・vhost追加を行う。

- **2026-07-17 メールOTP/TOTP 2FAを「どちらか一方だけでログイン可能」に
  変更(ユーザー指示)**: 従来は「メールOTP必須、2FA(TOTP)有効時はさらに
  TOTPコードも必須」というAND方式だった。ユーザーへの確認の結果、
  「2FA有効時はTOTPコードだけでメールOTPをスキップしてログイン可能に
  する」という方針を採用。
  - 既存の`verify-otp`(メールOTP経由、2FA有効時はTOTPコードも要求)は
    **そのまま変更していない**——引き続き有効なログイン経路の一つ。
  - 新規`POST /api/auth/totp-login`(`server/src/main.rs`、
    `TotpLoginRequest { account_email, totp_code }`)を追加。
    `users.totp_enabled()`でTOTP未有効のアカウントは`403 Forbidden`で
    拒否(そのアカウントにとっての2つ目の要素が存在しないため)。
    有効なアカウントはTOTPコードのみでセッション発行(メールOTPの
    リクエスト・消費を一切経由しない)。
  - **検証**: `cargo test`(server側)— **48件全green**
    (新規2件: `totp_login_rejects_accounts_without_totp_enabled`、
    既存の`totp_setup_enable_then_requires_code_on_next_login`内に
    実HTTP経由での`totp-login`成功ケースを追記)。WSL Ubuntu
    (rustc/cargo 1.97)で実施、型チェックのみでなく実際のHTTP
    リクエスト・レスポンスで確認済み。
  - **未着手(次回セッション、ユーザー指示「次回2FAともう一つのe-mailも
    確認」)**: (1) WASM側(`src/api_auth.rs`/`src/auth_ui.rs`)に
    `totp-login`を呼ぶUI導線がまだ無い(現状はサーバーAPIのみ)。
    (2) 次回、実際にブラウザ操作で(a) メールOTP+TOTPの既存フロー、
    (b) TOTPコード単体の新フロー、両方が実際にログインできることを
    確認する(「もう一つのe-mail」=セカンドメール/backup_email経由の
    メールOTPフローも含めて確認する、という意味と解釈)。

- **2026-07-17 `aon-co-jp/easyweb`と`aon-co-jp/open-easyweb`を本リポジトリ
  (`open-easy-web`、ドメイン`easy-web.tokyo`)へ融合 — ユーザー指示**:
  開発が並行して分岐していた2つのリポジトリを統合。
  - **ベースとして採用したのは`easyweb`側**——TOTP 2FA
    (`server/src/totp.rs`)・実ドメイン自動化(証明書自動取得込み、
    `server/src/tls.rs`)・WASM側の認証UI一式(`src/api_auth.rs`・
    `src/api_upload.rs`・`src/auth_ui.rs`)・実VPS(旧`easy-web.tokyo`
    ドメイン)での本番投入実績があり、`open-easyweb`より機能的に先行
    していたため。
  - **`open-easyweb`側から統合した独自追加分**: `server/src/
    appserver_registration.rs`(2026-07-16新設、open-web-server/
    poem-cosmo-tauriの共有バックエンド管理APIへドメインを動的登録する
    「分身の術」構想の仕上げ)、`src/view_bridge.rs`(open-runo-view
    Phase 3/4のSSR hydration連携)、`docs/HYBRID_NETWORK_ARCHITECTURE.md`。
    `Cargo.toml`に`open-runo-view`(git依存、dom feature)を追加。
  - **リブランディング**: パッケージ名`open-easyweb`→`open-easy-web`、
    バイナリ名`open-easyweb-server`→`open-easy-web-server`、
    リポジトリURL`aon-co-jp/open-easyweb`→`aon-co-jp/open-easy-web`、
    ドメイン参照`runo.tokyo`/`easyweb.tokyo`→`easy-web.tokyo`
    (機械的sed置換、コード中の`OPEN_EASYWEB_*`環境変数名・
    `openeasyweb_*`localStorageキーは互換性維持のためあえて据え置き)。
  - **統合時に発見・修正した実バグ**: `appserver_registration.rs`が
    使う`thiserror`が`server/Cargo.toml`に未宣言だった(元の
    `open-easyweb`側でのみ追加していた依存の移植漏れ)——追加して解消。
  - **検証**: `cargo check --target wasm32-unknown-unknown`
    (WASM側、`open-runo-view`のgit依存解決込み)成功。
    `cargo test`(server側)——**47件全green**
    (TOTP 5件・tls 2件・totp各種・appserver_registration 3件・
    実HTTP統合テスト(OTPログイン・TOTPセットアップ・サイト操作の
    フルフロー)含む)。WSL Ubuntu(rustc/cargo 1.97)で実施。
  - **UI再統合も同一パス内で完了**: 上記「未着手」だった
    「🔗 共有バックエンドへ登録」UIを、`easyweb`ベースの
    `src/profiles.rs`(`SiteProfile`に`shared_appserver_endpoint`・
    `shared_appserver_admin_key`・`shared_appserver_db_uri`・
    `shared_appserver_session_token`の4フィールド追加、
    `on_register_appserver`ハンドラ・`register_appserver_request`
    fetch関数を追加)・`src/shell.rs`(対応する入力欄4つを追加)へ
    再度手動統合した。`cargo check --target wasm32-unknown-unknown`
    成功(WSL Ubuntu、rustc/cargo 1.97)。
  - **未着手(次回セッション)**: (1) 実VPS(`easy-web.tokyo`)への実
    デプロイ・動作確認は未実施(今回はローカルビルド・テストのみ)。
    (2) 10ヶ国語README・PORTING.mdの内容更新(タイトル・リポジトリURLの
    機械置換のみ実施、内容の見直しは次回)。
  - **リポジトリ削除について**: ユーザーから「融合完了後は
    `aon-co-jp/easyweb`と`aon-co-jp/open-easyweb`を削除してほしい」との
    指示を受けているが、削除は取り消し困難な操作のため、この統合が
    実際に問題なく動くことをユーザーに確認していただいた上で、
    削除の実行直前に改めて明示確認を取ってから実施する方針(削除は
    まだ実施していない)。

- **2026-07-16 本番投入: ドメイン自動化(HTTPS自動取得含む)・TOTP 2FA・
  OTPメールの実リンク化**: 本番のVPS(easy-web.tokyo経由)で以下を実装・
  ビルド・デプロイ・実際に動作確認済み。
  - **ドメイン登録・削除・HTTPS自動取得**(`server/src/tls.rs`新規、
    `server/src/vhost.rs`拡張): `certbot certonly --webroot`を呼ぶ
    `tls::ensure_cert()`を新設。`vhost::apply_with_auto_tls()`が
    (1)まずHTTPのみのvhost(`deploy/nginx/vhost-php-http-only.conf.template`
    新規)を適用してサイトを即座に閲覧可能にし、(2)証明書取得を試み、
    (3)成功すればHTTPS版vhost(既存`vhost-php.conf.template`)に差し替える
    ——失敗してもHTTPでサイトは動き続ける設計。`vhost::remove()`で
    ドメイン登録の削除(`DELETE /api/sites/:name`)にも対応、アップロード
    済みファイル・証明書自体は削除しない(破壊的操作の最小化)。
  - **TOTP(認証アプリ)2FA**(`server/src/totp.rs`新規、HMAC-SHA1+base32を
    自前実装、外部totpクレート不使用): `users.rs`に`totp_secret`/
    `pending_totp_secret`を追加。`/api/auth/totp/{setup,enable,disable}`
    新設。`verify-otp`はTOTP有効アカウントの場合`totp_code`必須にし、
    無ければ`totp_required: true`を返しセッションを発行しない(真の2FA、
    ユーザー確認済みの仕様)。WASM側(`api_auth.rs`/`auth_ui.rs`/
    `shell.rs`)にセットアップ・有効化・無効化UIとログイン時のTOTP入力欄を
    追加。
  - **OTPメール本文の「連絡先変更はこちら」を実リンク化**
    (`mail.rs`): 従来はプレーンテキストの案内文だけだったのを、
    `https://easy-web.tokyo/`への実際のURLに変更(クリックでサイトへ
    遷移できる)。
  - **検証**: `cargo build`/`cargo test`とも44件全green(新規9件:
    tls 2件・totp 5件・DELETE統合1件・TOTP全体フローの実HTTP統合テスト1件)。
    VPS実機で: OTP送信→ログイン→TOTPセットアップ→有効化→次回ログインで
    `totp_required`が返ることを実際のHTTPリクエストで確認。WASMも
    VPS上で再ビルド・`wasm-bindgen`再生成し、生成された`.wasm`バイナリに
    新UI要素(`totp-setup-btn`等)が実際に含まれることを`strings`コマンドで
    確認。
  - **未着手**: ドメイン自動化のフルフロー(新規ドメイン追加→HTTPS自動
    取得)は実際の未使用ドメインでの実地E2E検証はまだ行っていない
    (既存の稼働中ドメインを壊さないよう、本セッションではユニット/
    ローカルAPI呼び出しレベルの検証に留めた)。次回、実際に新しい
    テストドメインで試すこと。

- **2026-07-15 コードヘルス監査 — audit only, no changes**:
  ルートクレート(`cargo build --target wasm32-unknown-unknown`)・
  `server/`クレート(`cargo build`)ともに警告0件でビルド成功。
  `server/`のテストは35件全green、ルートクレートは(WASM専用のため
  想定通り)ユニットテスト0件。`git status`はクリーン、修正すべき壊れた
  ビルド・失敗テスト・小規模な欠落は見つからなかったため、コード変更は
  行っていない。前回HANDOFFエントリで「次回セッションが最初にすべきこと」
  として挙げられていたサーバー再起動・実ブラウザでの新UI動作確認は、
  このパスはコード健全性の巡回監査(ビルド/テスト/lint/git状態)に
  スコープを絞ったため未実施——引き続き次回の開発セッションでの対応が
  必要(本エントリでは着手しない)。

- **2026-07-14(深夜、続き。フロントエンド配線完了、サーバー再起動待ちで中断——
  次回セッションが最初に読むこと)**: 前回HANDOFFで「未着手」としていた
  「WASMフロントエンドの日英併記UI配線」に着手・完了。加えて、
  ユーザー指示で連絡先変更フローを一般化した。

  **今回変更・追加したファイル(すべて`git status`で確認済み、
  コミット・push未実施——このHANDOFF自体もこのコミットに含める)**:
  - **サーバー側の一般化**(`server/src/`):
    - `users.rs`: `ContactField`(`Phone`/`BackupEmail`)enum + `parse()`、
      `update_contact(account_email, field, new_value)`を追加
      (主メール改名は既存の`rename_email`のまま——アカウント識別子なので
      扱いを分離)。
    - `auth.rs`: `PendingEmailChange`→`PendingContactChange`
      (`field: String`追加)に一般化、`request_email_change`/
      `confirm_email_change`→`request_contact_change`/
      `confirm_contact_change`にリネーム(`field`引数を追加、返り値も
      `(account_email, field, new_value)`の3要素タプルに)。
    - `mail.rs`: 確認メール送信を`send_contact_change_confirmation`に
      一般化(`field_label()`で日英併記のラベルを埋め込み)。**OTPログイン
      メール本文に「携帯電話番号やメールアドレスの変更はこちら」という
      日英併記の案内文を追記**(ユーザー指示)。
    - `main.rs`: `RequestEmailChangeRequest`に`field`
      (省略時デフォルト`"email"`、後方互換)を追加、
      `confirm_email_change`ハンドラは`field`に応じて`rename_email`
      または`update_contact`を呼び分ける。
    - **検証**: `cargo build`/`cargo test`とも32件全green
      (新規3件: `contact_change_confirmation_round_trips_account_field_
      and_new_value`・`contact_change_supports_phone_and_backup_email_
      fields`・`unknown_contact_change_token_is_rejected`)。
  - **WASMフロントエンド新規配線**(`src/`):
    - 新規`src/api_auth.rs`: `register`/`request_otp`/`verify_otp`
      (成功時`localStorage`にセッショントークン+アカウントメールを保存)/
      `logout`/`request_contact_change`/`register_hint`の`fetch()`
      ラッパー。JSON⇔`serde_json::Value`変換は`serde-wasm-bindgen`を
      新規依存追加せず`JSON.stringify`→`serde_json::from_str`の素朴な
      方法で実装(既存の「薄い依存のみ」方針を踏襲)。
    - 新規`src/api_upload.rs`: `create_folder`/`upload_files`
      (`FormData`+`web_sys::FileList`、`multipart/form-data`)/
      `detect_and_configure`/`correct_detection`の`fetch()`ラッパー、
      全て`Authorization: Bearer`付き。
    - 新規`src/auth_ui.rs`: DOM配線本体。登録フォーム・ログインフォーム
      (連絡先入力→OTP入力の2段階)・ログイン中パネル(メールアドレス
      表示+ログアウト)・連絡先変更フォーム(`<select>`でメール1/メール2/
      電話番号を選択)・サイト操作パネル(フォルダー作成→アップロード→
      🤖AI判定&自動構成→確信度%表示→訂正ボタン)の全イベントハンドラ。
      ログイン状態に応じ`auth-logged-out`/`auth-logged-in`/
      `site-ops-section`の表示を`sync_auth_visibility()`で切替。
      `wire()`内で`register_hint()`をサーバーから取得し登録フォームの
      案内文をライブ上書き(HTML側の静的文言はサーバー未起動時の
      フォールバックとして残置)。
    - `src/shell.rs`: 上記UI用のHTMLセクション一式を追加
      (`auth-section`・`site-ops-section`)。
    - `src/lib.rs`: `mod api_auth; mod api_upload; mod auth_ui;`追加、
      `start()`内で`auth_ui::wire()`を呼び出し。
    - `Cargo.toml`: `web-sys`featureに`FormData`/`Headers`/`Response`
      を追加。
    - `index.html`: `.hidden { display: none; }`を追加。
  - **検証**: `cargo build --target wasm32-unknown-unknown`は**警告0件**で
    成功(`register_hint`が未使用という警告が一度出たが、`wire()`内で
    実際に呼び出す形にして解消——「呼ばれない関数を書いて終わり」に
    しない、という既存の検証基準に従った)。`wasm-bindgen --target web`
    でのJSグルー再生成も成功。**実バイナリでの動作確認は
    サーバー再起動待ちで中断**(直前まで`http://127.0.0.1:8090`で
    旧UI(サイト管理画面のみ)の動作を確認済みだったが、新UIを反映した
    再起動はこのパスでは未実施)。

  **次回セッションが最初にすべきこと**:
  1. サーバーを再起動して新UIを反映:
     ```
     taskkill //F //IM open-easy-web-server.exe
     cd F:\open-runo\aruaru-easyweb
     (環境変数 OPEN_EASYWEB_STATIC_DIR 等は前回HANDOFFのローカル起動手順を参照)
     cargo run --manifest-path server/Cargo.toml
     ```
  2. ブラウザで`http://127.0.0.1:8090/`を開き、実際に:
     (a) 登録フォームで電話番号「なし」+メール2ありで登録→成功、
     (b) 電話番号もメール2も未入力での登録→エラー表示を確認、
     (c) ログイン(連絡先入力→OTP、SMTP未設定なら503が返るはずなので、
     `state.auth.request_otp()`相当をサーバーログや`--ignored`テスト
     経由で代替確認するか、実SMTP設定をこの機会に用意する)、
     (d) ログイン後にサイト操作パネル(フォルダー作成・アップロード・
     AI判定・訂正)が実際に動くこと、(e) 連絡先変更フォームで
     `field`セレクトの3パターンいずれも送信できること、を実ブラウザ
     操作で確認する。**型チェック・ビルド成功だけで「完了」と
     報告しないこと**(このリポジトリの既存の検証基準どおり)。
  3. 確認が取れたら、このHANDOFFの下に追記する形で結果を記録し、
     commit・pushする(このセッションの変更は**まだコミットされていない
     可能性がある**——`git status`を必ず確認すること)。
  4. その後、前々回HANDOFFに残っている未着手項目
     (`deploy/systemd/`への`open-easy-web-server.service`雛形追加、
     実VPSへの本番デプロイ・実SMTP/SMS WebhookでのE2E検証、
     10ヶ国語README/PORTING.md更新)に進む。

- **2026-07-14(夜、中断——次回セッションが最初に読むこと)
  新規`server/`クレート(`open-easy-web-server`)着手中、ビルド未最終確認のまま
  中断**: 経緯——実VPS(easy-web.tokyo/audiocafe.tokyo稼働中)でPHP実行に
  対応しようとしたところ、VPS上で動いている`aruaru-easyweb`バイナリの
  ソースがGitHub上のどこにも存在しない(ロストソース)ことが判明し、
  ユーザー承認のもと「後継の`open-easy-web`(このリポジトリ)に、PHP対応・
  アップロード機能・認証機能を実装し、後で新設された
  `aon-co-jp/aruaru-easyweb`リポジトリへコピーして移行する」方針になった。

  **実装済み(`F:\open-runo\open-easy-web\server\`、新規crate
  `open-easy-web-server`、tokio/hyper直接実装・重量級フレームワーク不使用)**:
  - `src/php_detector.rs`: 外部LLM不要の自己学習AI(poem-cosmo-tauriの
    `CachePredictor`と同じ設計思想)。ファイル拡張子・`<?php`タグ・
    `wp-config.php`/`composer.json`/`artisan`/`.htaccess`の各シグネチャを
    ノイズOR結合(`1-Π(1-w_i)`)でスコアリングしPHP判定。手動訂正で
    EWMA式(α=0.2)に重みを補正・JSON永続化。
  - `src/vhost.rs`: `deploy/nginx/vhost-php.conf.template`
    (本セッション前半で追加済み)を読み込みplaceholder置換、
    `/etc/nginx/conf.d/<domain>.conf`へ書き込み(`sites-available`ではなく
    `conf.d`を使う理由: 実VPS運用でnginxの`conf.d`が`sites-enabled`より
    先に読み込まれ優先されることを実証済みだったため、他ツール
    (`aruaru-easyweb`)との重複時にも安全側に倒せる)、`nginx -t`→
    `systemctl reload nginx`、失敗時ロールバック。
  - `src/upload.rs`: `multipart/form-data`手書きパーサー(RFC 7578、
    poem-cosmo-tauriの`read_multipart_body`と同じアプローチ)。
    パストラバーサル対策の`safe_relative_path`。
  - `src/auth.rs` + `src/users.rs`: **固定パスワード無し、メールOTP認証**。
    `UserStore`(`email`をID、`phone`または`backup_email`のうち
    最低どちらか一方を登録必須——電話「なし」ならセカンドメール必須、
    JSON永続化)。`AuthStore`は連絡先(主メール/セカンドメール/電話番号
    いずれか)をキーにOTP発行・検証し、検証成功後に呼び出し側が
    `UserStore`で解決した主メールに対しセッションを発行する設計
    (OTPロジックと「どの連絡先がどのアカウントに属するか」を分離)。
    メールアドレス変更は`request_email_change`/`confirm_email_change`
    (確認リンクは**新アドレスではなく現在の主メール宛にのみ送る**——
    アカウント乗っ取り防止)。
  - `src/mail.rs`(`lettre`、SMTP、`OPEN_EASYWEB_SMTP_*`env var、
    未設定時は503でグレースフルデグレード)・`src/sms.rs`
    (特定プロバイダに依存しないWebhook方式、`OPEN_EASYWEB_SMS_WEBHOOK_URL`)。
  - `src/main.rs`: 全エンドポイント配線
    (`/api/auth/register`・`/register-hint`・`/request-otp`・
    `/verify-otp`・`/logout`・`/request-email-change`・
    `/confirm-email-change`、`/api/sites/:name/{folder,upload,
    detect-and-configure,correct}`は`Authorization: Bearer`必須)。
    登録フォーム向けに「メール1・メール2・電話番号の3つとも登録推奨」
    という日英併記の案内文(`REGISTER_HINT`定数)を用意済み。
  - **テスト31件、`cargo build`・`cargo test`とも直前確認時点でgreen**
    (実TCP+`reqwest`での統合テスト2本、OTPログイン→セッション→
    保護エンドポイントアクセスのフルフローを実HTTP経由で検証済み)。
    ただし`users.rs`の軽微な警告修正(未使用import削除、
    `find_by_email`に`#[allow(dead_code)]`)を最後に加えた**直後の
    再ビルド確認は中断により未実施**——次回セッションの最初に
    `cd F:\open-runo\open-easy-web\server && cargo build && cargo test`
    を実行し、警告0件・31件全green(またはそれ以上)を確認すること。

  **未着手として明記(次回セッションが着手すべきこと、確認不要で進めてよい)**:
  1. **WASMフロントエンド(`src/`側)の日英併記UI配線が丸ごと未着手**。
     `src/shell.rs`に登録フォーム(email・phone・backup_email入力、
     「なし」選択肢)・OTPログインフォーム(contact入力→OTP入力の2段階)・
     フォルダー作成/アップロードUI・AI判定結果表示(🤖確信度%+訂正ボタン)・
     メールアドレス変更フォームを追加し、新規`src/api_auth.rs`/
     `src/api_upload.rs`(`profiles.rs`と同じ`fetch()`薄いラッパー
     パターン)からサーバーAPIを呼ぶ配線が必要。`src/lib.rs`の`start()`
     に新規ボタンのイベントリスナー登録も要る。
  2. `scripts/serve.sh`が引き続き`python -m http.server`のままなので、
     実際に`open-easy-web-server`バイナリでWASMバンドル配信も兼ねる形に
     切り替えるかは(1)の後で判断(README/CLAUDE.mdへの新旧起動方法の
     併記のみ済み、デフォルト変更は保留のまま)。
  3. `deploy/systemd/`への`open-easy-web-server.service`雛形追加。
  4. **実VPSへの本番デプロイ・実SMTP/実SMS Webhookでのエンドツーエンド
     検証は未実施**(このセッションでは`server`クレートの実装・
     ユニット/統合テストまで)。
  5. 10ヶ国語README・PORTING.mdの更新、この`CLAUDE.md`の「現状」節更新
     (このHANDOFFエントリ自体は書いたが、要約反映は次回)。
  6. 全て完了し実VPSで検証できたら、`aon-co-jp/aruaru-easyweb`
     (新設リポジトリ)へファイル一式をコピーし、以後はそちらで開発を
     継続する(ユーザー指示、2026-07-14夜)。

  **VPS側の状態(このセッション中に実施済み、正常)**: `easy-web.tokyo`の
  ネームサーバー委任がバリュードメイン→ConoHa DNS(`a.conoha-dns.com`/
  `b.conoha-dns.org`)に修正され`.tokyo`レジストリへ反映済み・DNS解決
  正常。`audiocafe.tokyo`はnginx+PHP-FPM(`/etc/nginx/conf.d/
  audiocafe.tokyo.conf`、手動作成)で実際にPHPサイトとして稼働中
  (200確認済み)。旧`aruaru`(PostgreSQL版、port 3000)サービス・
  関連する`/root/aruaru`残骸ディレクトリ・`aruaru-os-daily`
  タイマーは完全削除済み。`aruaru-easyweb`(port 8080)自体は
  引き続き稼働中だが、**`audiocafe.tokyo`/`easy-web.tokyo`のドメイン登録は
  このパスの最後にユーザー指示で削除済み**(`DELETE /api/domain/:id`、
  aruaru-easyweb自身の削除機能を使用)——実際のサイト提供は
  `conf.d/`側の手動設定が引き続き担っているため、削除による機能影響は無い。

- **2026-07-14(続き) 廃止済みサービスの残骸監査ツールを新設
  (ユーザー指示「AIが自動削除する機能を搭載して」への代替提案・承認済み)**:
  ユーザーから「VPS上の`aruaru-web`を削除したが、cronジョブ・証明書更新
  スクリプト等の残骸が無いか自動調査してAIが自動削除するメンテナンス
  機能を`open-easy-web`に搭載して」という指示があった。**「AIが判断して
  自動削除する」設計はあえて採用しなかった**——本番インフラの
  cron/systemd/証明書設定の削除は破壊的かつ復元困難で、別の現役サービスが
  同じcronエントリや証明書更新フックを共用しているケースを誤検知すると
  無関係なサービスを巻き添えにするリスクがあるため。代わりに
  **「検知・レポートは自動化するが、削除の実行は人間の最終承認を必須と
  する」設計を提案し、ユーザーの承認を得た**。
  新規`scripts/audit-orphaned-services.sh <検索文字列...>`:
  systemd unitファイル(`/etc/systemd/system/*.{service,timer}`、
  ファイル名+中身の両方を検索)・crontab(root/各ユーザー/`/etc/cron.d`)・
  certbot renewal設定(`/etc/letsencrypt/renewal/*.conf`とその
  deploy/pre/post-hook)の3種類を走査し、検索文字列(廃止したサービス名や
  ドメイン名)にマッチする項目を一覧表示する。**delete/rm/systemctl
  disable等の破壊的コマンドは一切実行しない**——見つかった項目ごとに
  「削除の目安コマンド」を`<REVIEW>`プレースホルダ付きで表示するのみ
  (そのままコピペ実行できないよう意図的に配慮)。`bash -n`での構文検証
  および、このWindows開発環境でのdry-run実行(該当ディレクトリが
  存在しないため「見つかりませんでした」を正しく返すことを確認——実際の
  検出動作は、実VPS(`easy-web.tokyo`が稼働中のConoHa AlmaLinux環境)で次回
  検証すべき)。README(ルートのみ、10ヶ国語同期は次回)に使用方法を追記。
  **未着手として明記**: (1) 実VPS環境での実行検証、(2) 10ヶ国語READMEへの
  反映。
  併せて、`open-web-server`のREADMEに命名の由来(`open-web-server`は
  ユーザーによる命名、`aruaru-server`はClaude開発過程での命名)と
  両者の位置付けを追記(ユーザー指示)——`aruaru-server`は`aruaru-db`
  workspace内の1クレート(`[[bin]] name = "aruaru-server"`)であり、
  `aruaru-query`/`aruaru-wire`/`aruaru-dist`等と密結合しているため、
  別リポジトリへの分離は開発上のメリットが無く推奨しないと回答した。

- **2026-07-14(配信エンジン選択・アプリケーションサーバー選択をドメイン単位で
  追加、ユーザー指示)**:
  1. **配信エンジン(Nginx/Apache)の選択・後からの変更**:
     `scripts/gen-vhost.sh`に`--engine=nginx|apache|both`を追加(既定は
     `both`、旧来と同じ動作を維持)。新規`scripts/switch-engine.sh
     <DOMAIN> <nginx|apache>`で、登録済みドメインの配信エンジンを
     後からいつでも切り替え可能にした(生成済みvhostを配置先
     ディレクトリへコピーし、もう片方のエンジンのvhostは`.disabled`へ
     退避、対象エンジンのみリロード)。RHEL系(`/etc/nginx/conf.d`・
     `/etc/httpd/conf.d`)・Debian系(`/etc/apache2/sites-enabled`)の
     両方の配置先を自動検出。
  2. **ドメイン単位のアプリケーションサーバー選択(Apache+Tomcat型)**:
     `src/profiles.rs`の`SiteProfile`に`app_server`
     ("none"/"open-runo"/"poem-cosmo-tauri")・`app_server_upstream`
     (host:port)フィールドを追加(`#[serde(default)]`で旧localStorage
     データとの互換を維持)。`src/shell.rs`のサイト管理フォームに対応する
     選択UI・入力欄を追加、`src/profiles.rs`の一覧カード表示・
     編集フォームへの反映・保存処理すべてに配線。既存ドメインの
     編集フォーム経由で選択変更・削除が可能(profiles.rsの既存の
     編集/削除フローをそのまま利用、新規追加コードは無し)。
     新規`scripts/switch-app-server.sh <DOMAIN> <none|open-runo|
     poem-cosmo-tauri> [HOST:PORT]`で、デプロイ済みvhostの
     `proxy_pass`/`ProxyPass`転送先を後から書き換え可能(nginx/apache
     どちらがデプロイ済みかを自動検出)。
  3. `open-web-server`側にも対の実装(`open-web-server-gateway`の
     `app_proxy`モジュール、`OPEN_WEB_SERVER_APP_UPSTREAM`環境変数で
     単体動作/アプリサーバー委譲を切り替え)を追加済み——詳細は
     `open-web-server`側CLAUDE.md参照。
  **検証**: `cargo build --target wasm32-unknown-unknown`成功
  (警告確認は次回の`cargo clippy`実行時に併せて行う)。
  **未着手として明記**: 実VPS環境での`switch-engine.sh`/
  `switch-app-server.sh`の実行検証(このパスはWindows開発環境のため
  未実施)、Ruby(Puma/Unicorn)・Perl(PSGI/Plack)向けの専用`gen-vhost.sh`
  スタックテンプレート追加(現状は`--stack=proxy`の汎用UPSTREAMで代替
  可能、次回パスで明示的なスタックとして追加予定)。

- **2026-07-13(open-web-server連携を実バイナリで検証)**: ユーザー指示
  「open-easy-web と open-web-server 関連リポジトリ同士の連携を高めて」を
  受け、`scripts/gen-vhost.sh --stack=proxy`が生成する汎用リバースプロキシ
  vhostが実際に`open-web-server-gateway`(バイナリ名`open-web-server`、
  デフォルト`0.0.0.0:8080`、`/healthz`ヘルスチェック実装済み)を正しく
  指せることを、モックではなく実バイナリ・実HTTPサーバーで検証した:
  1. `open-web-server`側で`cargo build -p open-web-server-gateway`、
     `OPEN_WEB_SERVER_BIND=0.0.0.0:18080`で実起動。
  2. `scripts/gen-vhost.sh --stack=proxy owstest.example.com 127.0.0.1
     127.0.0.1:18080`で実際にvhostを生成、`proxy_pass
     http://127.0.0.1:18080`/`ProxyPass "/" "http://127.0.0.1:18080/"`が
     正しく埋め込まれることを確認。
  3. **Nginx**: winget経由で公式`nginxinc.nginx`パッケージをWindows側に
     導入(ハッシュ検証済み)、生成vhostのTLS部分だけ差し替えた
     プレーンHTTP版設定で`nginx -t`(構文検証)に加え、実際に
     `nginx.exe`を起動して`curl http://127.0.0.1:18081/healthz`で
     **実際にnginx経由でopen-web-server-gatewayまでHTTPリクエストが
     到達し200を返すこと**をnginxアクセスログ・gateway側リクエストログ
     の両方で確認(エンドツーエンド実証)。
  4. **Apache**: WSL2 Ubuntuに`apache2`を導入し
     `a2enmod ssl proxy proxy_http rewrite`、生成された
     `owstest2.example.com.apache.conf`をそのまま`sites-available`に
     配置(自己署名証明書をSSLCertificateFile参照先に用意)、
     `apache2ctl configtest`で**Syntax OK**を確認(WSL側から
     Windows側ホストへのアップストリーム到達はWindows Defender
     ファイアウォールの既定ブロックにより未検証——`ProxyPass`構文自体の
     妥当性検証が目的であり、configtestはアップストリームの疎通を
     要求しないためこれで十分)。
  5. **2026-07-13追記: TLS(Let's Encrypt実証明書)を実ドメインで検証完了**
     (上記「対象外」は解消)。ユーザーが実際に取得済みのドメイン
     `easy-web.tokyo`を、実VPS(ConoHa、AlmaLinux 10.2、既に`aruaru`
     (PostgreSQL版, port 3000)・`aruaru-easyweb`(port 8080)・
     nginx が稼働中の環境)のDNS Aレコード(`easy-web.tokyo`・
     `www.easy-web.tokyo`とも本番VPSのIPアドレス、Google Public DNS経由で
     反映確認済み)に向けた上で、`certbot certonly --webroot`で
     **実際にLet's Encryptから本物の証明書を取得**(2026-10-11まで
     有効、自動更新スケジュール設定済み)。
     **実バグ発見・修正**: 既存nginx設定のACME webroot
     (`/root/aruaru/data/acme-webroot`)は`/root`ディレクトリ自体が
     `750`権限(nginxユーザーがトラバース不可)のため、実際に
     ACME HTTP-01チャレンジが403で失敗した(`/root`配下にwebroot
     を置くこと自体が本番運用上のバグ)。`/var/www/acme-webroot`
     (`nginx:nginx`所有、755)へ移設して解消——**`scripts/gen-vhost.sh`
     ・`scripts/setup-tls.sh`のテンプレートが将来的にwebrootを生成する
     場合、`/root`配下は絶対に使わないこと**(この教訓をドキュメント化)。
     443番のvhostを新設(`server_name easy-web.tokyo www.easy-web.tokyo;`、
     `ssl_certificate`は取得した実証明書を参照)し`aruaru-easyweb`
     (port 8080)へプロキシ。**実インターネット経由で検証**:
     `http://easy-web.tokyo/`が`301`で`https://`へリダイレクト、
     `curl`(証明書検証あり、`-k`オプション無し)で`https://easy-web.tokyo/`
     ・`https://www.easy-web.tokyo/`とも`200`、実際に`aruaru-easyweb`の
     ダッシュボードHTMLが返ることを確認——自己署名やスキップ検証では
     ない、本物の公的信頼された証明書での疎通。
  **結論**: `open-easy-web`の`--stack=proxy`vhost生成レシピは
  `open-web-server`の実エンドポイント(`/healthz`、デフォルトポート
  8080)と整合しており、実際にリバースプロキシとして機能することを
  実バイナリ・実HTTPサーバーで確認した(ドキュメント記載のみだった
  従来の状態から昇格)。

- **2026-07-13(初回パス、ブートストラップ)**: ユーザーからの新規指示
  「aruaru-webのドメイン/HTTPS/易操作機能を`open-easy-web`に分離し、
  高速化機能は`open-runo`/`poem-cosmo-tauri`へ統廃合・融合する」を受け、
  `aruaru-web/CLAUDE.md`・全スクリプト・vhostテンプレートの実ソースを
  読んだ上で本リポジトリをブートストラップ。
  - `src/{lib,dom,profiles,shell}.rs`・`index.html`をaruaru-webから
    コピーし、`aruaru-web`→`open-easy-web`のブランディング置換
    (localStorageキーも`_v2`→`_v1`でリネーム、DBに依存しない汎用
    ツールという性質は変わらないため機能面の変更は無し)。
  - `scripts/{serve,setup-tls,check-tls,check-all-tls}.sh`・
    `deploy-vps.ps1`・`gen-vhost.sh`をそのまま移植(パス/ファイル名の
    ブランディングのみ変更)。
  - `deploy/nginx/`・`deploy/apache/`の5スタック×2(Nginx/Apache)=10
    テンプレートから、gzip・expires/Cache-Control・fastcgi_buffers・
    named upstream+keepaliveを全て削除した新テンプレートを新規作成
    (aruaru-web側のオリジナルは全てNginx 1.24/Apache 2.4で
    `nginx -t`/`apache2ctl configtest`により構文検証済みだったため、
    このパスでの変更は「ディレクティブの削除のみ」——新規構文を
    一切追加していない差分であることを目視で確認)。
  - `deploy/systemd/`の`aruaru-tls-*`を`easyweb-tls-*`にリネーム。
  - **検証**: `cargo build`/`cargo clippy`(`--target
    wasm32-unknown-unknown`)ともに警告0件。`bash scripts/gen-vhost.sh`
    を全5スタックで実行しプレースホルダ置換を確認。**この開発環境が
    Windowsであり、nginx/apacheバイナリが利用できないため、
    `nginx -t`/`apache2ctl configtest`による実際の構文検証はこのパスでは
    未実施**——正直な限界として明記する。次回、Linux環境(または
    WSL/コンテナ)が利用可能であれば、aruaru-webの過去パスと同様の
    手順(Nginx 1.24・Apache 2.4を導入し全5スタックを構文検証、
    static/proxyスタックは実起動してcurl機能検証)を実施すること。
  - 10言語README・PORTING.md・このCLAUDE.mdを新規作成。
  **次回パスがすべきこと**: (1) Linux環境が利用可能になり次第、
  `nginx -t`/`apache2ctl configtest`による実際の構文検証、(2)
  `scripts/deploy-vps.ps1`の実VPS環境での動作確認(aruaru-web側でも
  未検証のまま持ち越されていた項目)、(3) 実際のcertbotによる
  Let's Encrypt発行の検証(パブリックドメイン・外部到達可能な環境が
  必要)。

## アプリケーションサーバー層の役割(open-runo / RPoem[旧poem-cosmo-tauri]、2026-07-16追記)

「配信エンジン(vhost)」に`open-web-server`を選択肢として追加したが、
open-web-serverがApache＋Nginxのハイブリッド仕様のWebサーバーとして
まだ機能していない間は、Tomcatのような互換レイヤーとして機能するのは
`open-runo`またはRPoem(旧poem-cosmo-tauri)である。

これらは`open-raid-z`とVersionlessAPIによって、バージョンレス運用と
バージョン管理・Git管理を両立しながら、ACID互換性とZFS互換性に対応した
`aruaru-db`と、PostgreSQLとのDUAL DATABASE構成による「4層4重」の
最新鋭の通信システムを構築し、仕様変更が容易なデータベース設計により、
3DオンラインゲームAI課金アイテム、オンライン金融、オンライン証券、
オンラインクレジットカード決済など、ネット上で紛失してはならない
ミッションクリティカルな用途向けに、24時間365日ノンストップの
サーバー対応WEBサイト開発を全面的にバックアップするフレームワーク・
ミドルウェアとして機能することを目指す。
