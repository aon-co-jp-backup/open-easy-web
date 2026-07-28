//! アプリのHTMLシェル(サイト管理画面)。

pub const SHELL_HTML: &str = r#"
<header class="app-header">
  <h1>open-easy-web</h1>
  <p class="muted">
    「第二のKUSANAGI」— アプリのアップロード後にIPアドレスから起動し、
    ドメイン登録・HTTPS化を簡単に自動適用できる運用ツール(Rust &rarr;
    WebAssembly、フレームワーク不使用)。
  </p>
  <p class="muted">選択中のサイト: <strong id="active-site-name">(未設定)</strong></p>
  <p class="muted">
    これは管理者向け本番環境です。試しに使ってみたい方はデモ環境をどうぞ:
    <a href="/demo">https://easy-web.tokyo/demo</a><br>
    This is the admin/production environment. If you just want to try it out, use the demo instead:
    <a href="/demo">https://easy-web.tokyo/demo</a>
  </p>
</header>

<section id="setup-wizard-section">
  <h2>First-time Setup Guide (初回セットアップガイド)</h2>
  <p class="muted">
    VPSを借りたら最初にこの画面を確認してください。SFTPソフトで
    open-easy-webフォルダをアップロードし、Apache互換/Nginx互換の
    どちらでopen-web-serverを動かすか選び、必要ならインストール
    コマンドをコピーします。 / Check this screen first after renting a
    VPS. Upload the open-easy-web folder with an SFTP client, choose
    whether open-web-server should behave Apache-compatible or
    Nginx-compatible, and copy the install command if needed.
  </p>

  <h3>Step 1: Check the IP address you are accessing (① 現在アクセスしているIPアドレス)</h3>
  <p class="muted">
    このIPアドレス(またはホスト名)を、次のステップのSFTP接続先として使います。 /
    Use this IP address (or hostname) as the SFTP destination in the next step.
  </p>
  <p><strong id="setup-wizard-current-host">(取得中… / detecting…)</strong></p>

  <h3>Step 2: Upload via SFTP (② SFTPでopen-easy-webフォルダを作成・アップロード)</h3>
  <p class="muted">
    FileZilla・WinSCP等、お好みのSFTPクライアントでVPSへ接続し
    (ホスト: 上記IPアドレス、ポート: 通常22、ユーザー名/認証情報はVPS提供元の
    案内に従ってください)、サーバー上に <code>open-easy-web</code> という
    名前のフォルダを作り、ローカルの open-easy-web 一式(このアプリ本体)を
    その中へアップロードしてください。 <strong>このアップロード操作自体は
    SFTPクライアント上で手動で行う必要があります(このアプリからは自動化
    しません)。</strong> / Connect to the VPS with your preferred SFTP client
    (FileZilla, WinSCP, etc. — host: the IP address above, port: usually 22,
    username/credentials per your VPS provider's instructions), create a
    folder named <code>open-easy-web</code> on the server, and upload the
    local open-easy-web files into it. <strong>This upload step itself must
    be performed manually in your SFTP client (not automated by this
    app).</strong>
  </p>

  <h3>Step 3: Choose Apache-compatible or Nginx-compatible mode (③ Apache互換モード / Nginx互換モードを選択)</h3>
  <p class="muted">
    アップロードが完了したら、このサイトをopen-web-server上でどちらの
    互換モードで配信するかを選んでください。ファイルが見つからない場合の
    挙動が変わります: Apache互換は`.htaccess`のFallbackResource相当で
    index.htmlへフォールバック、Nginx互換はtry_files相当でフォールバック
    せず404を返します。 / After uploading, choose which compatibility mode
    open-web-server should use to serve this site. This changes what
    happens when a requested file is missing: Apache-compatible falls back
    to index.html (like `.htaccess` FallbackResource), Nginx-compatible
    returns a plain 404 (like `try_files`) without falling back.
  </p>
  <div class="buttons">
    <button id="setup-wizard-apache-btn">Start in Apache-compatible mode (Apache互換モードで起動)</button>
    <button id="setup-wizard-nginx-btn">Start in Nginx-compatible mode (Nginx互換モードで起動)</button>
  </div>
  <p id="setup-wizard-mode-result" class="muted" aria-live="polite"></p>

  <h3>Step 4: Install / register open-web-server (④ open-web-serverのインストール、または追加登録)</h3>
  <p class="muted">
    <strong>open-web-serverは1台のVPSにつき1回だけインストールしてください。</strong>
    tenant_router(マルチテナント振り分け機構)が1プロセス内で複数ドメイン・
    複数アプリ(open-easy-webを含む)をホスト名・パスで振り分けるため、
    2つ目以降のドメイン/アプリでは再インストールは不要です——上のサイト管理
    画面(「共有バックエンドへ登録」)や、下の「簡単ドメイン設定」ウィザードから
    既存のopen-web-serverインスタンスへ追加登録するだけで済みます。 /
    <strong>Install open-web-server only once per VPS.</strong> Its
    tenant_router (multi-tenant dispatcher) routes multiple domains/apps
    (including open-easy-web) within a single process by hostname/path, so
    a second or later domain/app does not need reinstalling — just register
    it against the existing open-web-server instance using the site
    manager's "register with shared backend" option above, or the "Easy
    Free-Domain Setup" wizard below.
  </p>
  <p class="muted">
    まだこのVPSにopen-web-serverをインストールしていない場合は、以下の
    コマンドをコピーしてVPS上のターミナル(SSH)へ貼り付け、手動で実行して
    ください。<strong>このアプリがVPS上で自動的にコマンドを実行することは
    ありません</strong>(安全設計上の意図的な制約)。 / If you have not yet
    installed open-web-server on this VPS, copy the command below and paste
    it into a terminal (SSH) on the VPS yourself. <strong>This app never
    executes commands on the VPS automatically</strong> (an intentional
    safety design constraint).
  </p>
  <pre id="setup-wizard-install-command" class="code-block">curl -fsSL https://github.com/aon-co-jp/open-web-server/releases/latest/download/open-web-server-linux-x86_64.tar.gz | tar xz &amp;&amp; cd open-web-server-linux-x86_64 &amp;&amp; sudo ./install.sh</pre>
  <p class="muted">
    (Windows VPSの場合は代わりに <code>install.ps1</code> を使用してください。
    詳細は open-web-server の README を参照。 / On a Windows VPS, use
    <code>install.ps1</code> instead — see the open-web-server README for
    details.)
  </p>

  <h3>Step 5: Distributed sync &amp; disaster recovery (⑤ このファイルサーバーの分散同期・ディザスタリカバリ設定、任意)</h3>
  <p class="muted">
    このファイルサーバー(上記でセットアップしたopen-easy-webインスタンス)が
    抱えるサイトデータを、他のVPSへ継続的に複製する「分散同期クローンDB」・
    ネット切断や非常時にメール/Googleドライブへ自動退避する「ディザスタ
    リカバリ」を、ここでまとめて設定できます。<strong>設定は任意です——
    スキップしてもこのファイルサーバーは通常どおり使用できます。</strong> /
    You can configure "distributed sync clone DB" (continuously replicating
    this file server's site data to other VPS instances) and "disaster
    recovery" (automatic fallback to email/Google Drive on disconnection or
    emergency) together here. <strong>This step is optional — skipping it
    does not block normal use of this file server.</strong>
  </p>
  <p class="muted">
    管理トークン(サーバー起動時の <code>OPEN_EASYWEB_DIST_SYNC_ADMIN_TOKEN</code>
    環境変数と同じ値)を入力してから利用してください。未設定のサーバーでは
    この管理APIは無効化されています。 / Enter the admin token (same value as
    the <code>OPEN_EASYWEB_DIST_SYNC_ADMIN_TOKEN</code> environment variable
    set on the server) before using this. This admin API is disabled on
    servers where that variable is not set.
  </p>
  <div class="form-grid">
    <label>Admin token (管理トークン)<input type="password" id="dist-sync-admin-token" placeholder="OPEN_EASYWEB_DIST_SYNC_ADMIN_TOKEN"></label>
  </div>

  <h4>5a. Register a VPS clone target (他VPSへの分散同期先を登録)</h4>
  <div class="form-grid">
    <label>Host (ホスト)<input type="text" id="dist-sync-host" placeholder="vps2.example.tokyo"></label>
    <label>Port (ポート)<input type="number" id="dist-sync-port" value="22"></label>
    <label>Username (ユーザー名)<input type="text" id="dist-sync-username" placeholder="sync-user"></label>
    <label>Password env var (パスワード環境変数名)<input type="text" id="dist-sync-password-env" placeholder="EASYWEB_VPS2_SFTP_PASSWORD"></label>
    <label>Remote backup dir (退避先ディレクトリ)<input type="text" id="dist-sync-remote-dir" placeholder="/home/sync-user/easyweb-sync"></label>
    <label>Label, optional (任意のラベル)<input type="text" id="dist-sync-label" placeholder="東京VPS #2"></label>
  </div>
  <div class="buttons">
    <button id="dist-sync-register-btn">Register VPS sync target (VPS同期先を登録)</button>
    <button id="dist-sync-refresh-btn">Refresh list (一覧を更新)</button>
  </div>
  <p id="dist-sync-result" class="muted" aria-live="polite"></p>
  <div id="dist-sync-target-list"></div>

  <h4>5b. Disaster fallback destination, optional (ディザスタ用退避先、任意)</h4>
  <p class="muted">
    ネット切断・非常時に自動でメールまたはGoogleドライブへ退避します。
    どちらも設定せず「スキップ」してもかまいません。<strong>これは5aとは
    独立した機能です——上の「VPS同期先」を1件も登録していなくても、
    メールアドレスだけでこの退避先を設定できます。</strong> / Automatically
    falls back to email or Google Drive on disconnection/emergency. You may
    skip this and configure neither. <strong>This is independent from 5a —
    you can configure this with just an email address even if you have not
    registered any VPS sync target above.</strong>
  </p>
  <div class="form-grid">
    <label>SMTP host<input type="text" id="dist-sync-smtp-host" placeholder="smtp.example.com"></label>
    <label>SMTP port<input type="number" id="dist-sync-smtp-port" value="587"></label>
    <label>SMTP username<input type="text" id="dist-sync-smtp-username" placeholder="backup@example.com"></label>
    <label>SMTP password env var<input type="text" id="dist-sync-smtp-password-env" placeholder="EASYWEB_SMTP_PASSWORD"></label>
    <label>From address<input type="text" id="dist-sync-smtp-from" placeholder="backup@example.com"></label>
    <label>To address<input type="text" id="dist-sync-smtp-to" placeholder="admin@example.com"></label>
  </div>
  <div class="buttons">
    <button id="dist-sync-set-email-fallback-btn">Set email fallback (メール退避先を設定)</button>
  </div>
  <p class="muted">
    Googleドライブへ退避する場合は、事前にご自身でOAuth2クライアント登録・
    同意画面を済ませ、発行済みのリフレッシュトークンを環境変数として
    サーバーへ渡してください(このアプリがOAuth2認証を代行することは
    ありません)。 / For Google Drive, complete the OAuth2 client
    registration/consent screen yourself beforehand and pass the already
    issued refresh token to the server as an environment variable (this app
    never performs the OAuth2 flow on your behalf).
  </p>
  <div class="form-grid">
    <label>Backup folder name<input type="text" id="dist-sync-gdrive-folder" placeholder="open-easy-web-backup"></label>
    <label>Client ID env var<input type="text" id="dist-sync-gdrive-client-id-env" placeholder="EASYWEB_GDRIVE_CLIENT_ID"></label>
    <label>Client secret env var<input type="text" id="dist-sync-gdrive-client-secret-env" placeholder="EASYWEB_GDRIVE_CLIENT_SECRET"></label>
    <label>Refresh token env var<input type="text" id="dist-sync-gdrive-refresh-token-env" placeholder="EASYWEB_GDRIVE_REFRESH_TOKEN"></label>
  </div>
  <div class="buttons">
    <button id="dist-sync-set-gdrive-fallback-btn">Set Google Drive fallback (Googleドライブ退避先を設定)</button>
    <button id="dist-sync-verify-btn">Verify all targets now (今すぐ全同期先を疎通確認)</button>
    <button id="dist-sync-skip-btn">Skip for now (今はスキップ)</button>
  </div>
  <p id="dist-sync-fallback-result" class="muted" aria-live="polite"></p>
</section>

<section>
  <h3>Step 6: Nightly auto-update (⑥ 深夜バックグラウンド自動アップデート、既定OFF)</h3>
  <p class="muted">
    GitHub Releasesの最新バージョンを毎日ローカル深夜0時に確認し、新しい
    バージョンがあれば自動的にダウンロード・検証・切り替えます(Linuxでは
    ほぼゼロダウンタイム、Windowsは数百ミリ秒程度の切り替え猶予)。
    既定は無効(OFF)です。
  </p>
  <label>Admin token (管理トークン)<input type="password" id="auto-update-admin-token" placeholder="OPEN_EASYWEB_DIST_SYNC_ADMIN_TOKEN"></label>
  <label class="toggle-row">
    <input type="checkbox" id="auto-update-enabled-toggle">
    Enable nightly auto-update (深夜自動アップデートを有効にする)
  </label>
  <div class="buttons">
    <button id="auto-update-refresh-status-btn">Refresh status (現在の設定を取得)</button>
  </div>
  <p id="auto-update-status" class="muted" aria-live="polite"></p>
</section>

<div id="site-mgmt-section" class="hidden">
<section>
  <h2>Register / Edit / Delete / Switch Domains &amp; Subdomains (ドメイン名・サブドメイン名の登録・編集・削除・選択切替)</h2>
  <p class="muted">
    aruaru-easyweb自身のドメインと、それ以外の任意のドメイン・サブドメイン
    (WordPress・Laravel・FastAPIなど任意のバックエンドスタックのデプロイ先)を
    ここで一覧管理します。各カードの「選択」で切替、「編集」で内容変更、
    「削除」で登録取り消しができます。「他のサイト」用途のものは、保存・削除
    のたびに実際のサーバー側ドメイン登録(webroot作成・PHP自動判定・
    nginx+HTTPS自動構成)と連動します。DNS登録(レジストラでのAレコード設定)
    自体はここでは行いません。
  </p>
  <div id="site-list"></div>
  <div class="buttons">
    <button id="site-export" class="secondary">Export JSON (エクスポート)</button>
    <button id="site-import-trigger" class="secondary">Import JSON (インポート)</button>
    <input id="site-import-file" type="file" accept="application/json" style="display:none" />
  </div>
</section>

<section>
  <h2>Add / Edit Domain (ドメイン・サブドメインを追加・編集)</h2>
  <p class="muted">
    用途が「他のサイト」の場合、保存すると実際にサーバー側へドメイン
    (ホスト欄の値)を登録し、webrootの作成・PHP自動判定・nginx+HTTPSの
    自動構成まで行います。削除ボタンも同様に、実際のドメイン登録を
    取り消します(アップロード済みファイル・取得済み証明書は保持されます)。
    「このサイト(open-easy-web自身)」はここでの登録対象にはなりません。 /
    When purpose is "他のサイト" (other site), saving actually registers
    the domain on the server: creates the webroot, runs PHP
    auto-detection, and auto-configures nginx+HTTPS. Deleting likewise
    removes the actual domain registration (uploaded files and
    certificates are preserved).
  </p>
  <input id="site-form-id" type="hidden" value="" />
  <div class="form-grid">
    <div>
      <label for="site-name">Site name (サイト名)</label>
      <input id="site-name" type="text" placeholder="例: 本番WordPress" />
    </div>
    <div>
      <label for="site-purpose">Purpose (用途)</label>
      <select id="site-purpose">
        <option value="self">This site itself, open-easy-web (このサイト(open-easy-web自身))</option>
        <option value="other">Other site (他のサイト)</option>
      </select>
    </div>
    <div>
      <label for="site-protocol">Protocol (プロトコル)</label>
      <select id="site-protocol">
        <option value="https">https</option>
        <option value="http">http</option>
      </select>
    </div>
    <div>
      <label for="site-host">Host: IP / domain / subdomain (ホスト)</label>
      <input id="site-host" type="text" placeholder="例: 203.0.113.10 または example.com" />
    </div>
    <div>
      <label for="site-port">Port (ポート)</label>
      <input id="site-port" type="text" placeholder="443" value="443" />
    </div>
    <div>
      <label for="site-path">Path (パス)</label>
      <input id="site-path" type="text" placeholder="/" value="/" />
    </div>
    <div class="form-grid-full">
      <label for="site-stack">Backend stack, free text, optional (バックエンドスタック)</label>
      <input id="site-stack" type="text" placeholder="例: WordPress / PHP + Laravel / Python + FastAPI" />
    </div>
    <div>
      <label for="site-engine">Serving engine, vhost (配信エンジン)</label>
      <select id="site-engine">
        <option value="nginx">Nginx</option>
        <option value="apache">Apache</option>
        <option value="both">両方生成(未選択)</option>
        <option value="open-web-server" title="open-web-serverがApache＋Nginxのハイブリッド仕様のWebサーバーとしてまだ機能していない間は、配信エンジンではなくアプリケーションサーバー(Tomcat型)として扱ってください。">open-web-server</option>
      </select>
    </div>
    <div>
      <label for="site-app-server">Application server (アプリケーションサーバー)</label>
      <select id="site-app-server">
        <option value="none">None, web server only (なし)</option>
        <option value="open-runo">open-runo</option>
        <option value="poem-cosmo-tauri">poem-cosmo-tauri</option>
        <option value="aruaru-llm" title="契約不要の独自AIチャットコマース応答サービス(open-cudaとSET構成)。バックエンド接続先ではなくテナント登録のみ行う。">aruaru-llm (AIチャットコマース)</option>
      </select>
    </div>
    <div>
      <label for="site-app-server-upstream">App server upstream, host:port, optional (アプリケーションサーバー接続先)</label>
      <input id="site-app-server-upstream" type="text" placeholder="例: 127.0.0.1:8080" />
    </div>
    <div class="form-grid-full">
      <p class="muted">
        共有バックエンドへの登録(任意、2026-07-16新設・「忍者の分身の術」
        構想 — 忍術のように、実体(インスタンス)は1つだけ稼働させておき、
        各ドメインからは「分身」として同じ実体へ動的に登録するだけで
        済ませる): 既に稼働中のopen-web-server/poem-cosmo-tauri/
        aruaru-llmインスタンスへこのドメインを動的登録し、ドメインごとの
        個別インストールを不要にします。管理APIのURLを入力すると、一覧の
        カードに「🔗 共有バックエンドへ登録」ボタンが表示されます。
      </p>
      <p class="muted">
        <strong>2026-07-27追記(aruaru-db・open-raid-zの扱いについて、
        正直な開示)</strong>: <code>aruaru-db</code>は、下の「DB接続文字列」
        欄に稼働中インスタンスの接続文字列を入力するだけで、複数ドメインが
        同じDBインスタンスを共有できます——これも「忍者の分身の術」と
        同じ発想(実体は1つ、各ドメインは接続文字列を指すだけの「分身」)
        であり、専用のUI(ドロップダウンやボタン)を新設せずとも既に
        この欄で実現できています。一方、<code>open-raid-z</code>は
        ドメイン単位でテナント登録する対象の「アプリケーションサーバー」
        ではなく、VPS1台につき1回インストールしてマウントする
        ストレージ基盤(ファイルシステム層)のため、この「分身の術」欄の
        対象には含めていません(ドメインごとに使い分ける性質のものでは
        ないため)。 /
        <strong>Added 2026-07-27 (honest disclosure on how aruaru-db and
        open-raid-z fit in)</strong>: for <code>aruaru-db</code>, entering the
        connection string of an already-running instance into the "DB
        connection string" field below is enough for multiple domains to
        share that same DB instance — this is the same "ninja clone" idea
        (one real body/instance, each domain is just a "clone" pointing at
        the same connection string), and it is already achievable through
        this existing field without adding a dedicated dropdown or button.
        <code>open-raid-z</code>, on the other hand, is not a per-domain
        "application server" to register as a tenant here — it is a storage
        layer (filesystem-level) that you install and mount once per VPS, so
        it is intentionally left out of this "ninja clone" section (it isn't
        the kind of thing you switch per domain).
      </p>
    </div>
    <div>
      <label for="site-shared-endpoint">Shared backend admin API URL, optional (共有バックエンド管理APIのURL)</label>
      <input id="site-shared-endpoint" type="text" placeholder="例: http://127.0.0.1:8080" />
    </div>
    <div>
      <label for="site-shared-admin-key">Shared backend admin key, optional (共有バックエンドの管理キー)</label>
      <input id="site-shared-admin-key" type="password" placeholder="x-admin-token / x-api-key" />
    </div>
    <div>
      <label for="site-shared-db-uri">DB connection string — shared aruaru-db instance via "ninja clone", required for open-web-server (DB接続文字列 — 「忍者の分身の術」で共有するaruaru-db等のインスタンス、open-web-server利用時は必須)</label>
      <input id="site-shared-db-uri" type="text" placeholder="例: postgres://localhost/shop" />
    </div>
    <div>
      <label for="site-shared-session-token">open-easy-web-server session token, optional (セッショントークン)</label>
      <input id="site-shared-session-token" type="password" placeholder="Authorization: Bearer ..." />
    </div>
  </div>
  <div class="buttons">
    <button id="save-site">Save (保存)</button>
    <button id="clear-site-form" class="secondary">Clear (クリア)</button>
  </div>
</section>
</div>

<section id="auth-section">
  <h2>Account (アカウント)</h2>

  <div id="auth-logged-out">
    <p class="muted">
      セキュリティ上の理由により、新規登録は行っていません。あらかじめ
      登録済みの連絡先(メール1・メール2・携帯電話番号のいずれか)でのみ
      ログインできます。 / For security reasons, public registration is
      disabled. You can only log in with a pre-registered contact
      (Email 1, Email 2, or phone number).
    </p>

    <details open>
      <summary>Login, one-time password (ログイン)</summary>
      <div class="form-grid">
        <div>
          <label for="login-contact">メール1・メール2・電話番号のいずれか / Email 1, Email 2, or phone</label>
          <input id="login-contact" type="text" placeholder="you@example.com" />
        </div>
      </div>
      <div class="buttons">
        <button id="login-request-otp">Send code (コードを送信)</button>
      </div>
      <div class="form-grid">
        <div>
          <label for="login-code">受信したコード(6桁) / Received code (6 digits)</label>
          <input id="login-code" type="text" inputmode="numeric" placeholder="123456" />
        </div>
      </div>
      <div id="login-totp-row" class="form-grid hidden">
        <div>
          <label for="login-totp-code">認証アプリのコード(2FA有効時のみ) / Authenticator code (only if 2FA is enabled)</label>
          <input id="login-totp-code" type="text" inputmode="numeric" placeholder="123456" />
        </div>
      </div>
      <div class="buttons">
        <button id="login-verify-otp">Log in (ログイン)</button>
      </div>
      <p id="login-result" class="muted" aria-live="polite"></p>
    </details>

    <details>
      <summary>Log in with just an authenticator app code (認証アプリのコードだけでログイン)</summary>
      <p class="muted">
        2FA(認証アプリ)が有効なアカウントは、メールのワンタイムパスワードを
        経由せず、認証アプリの6桁コードだけでログインできます。 / If your
        account has authenticator-app 2FA enabled, you can log in with just
        its 6-digit code, skipping the email one-time password entirely.
      </p>
      <div class="form-grid">
        <div>
          <label for="totp-login-email">アカウントの主メールアドレス / Account primary email</label>
          <input id="totp-login-email" type="text" placeholder="you@example.com" />
        </div>
        <div>
          <label for="totp-login-code">認証アプリのコード(6桁) / Authenticator code (6 digits)</label>
          <input id="totp-login-code" type="text" inputmode="numeric" placeholder="123456" />
        </div>
      </div>
      <div class="buttons">
        <button id="totp-login-submit">Log in with authenticator code (認証アプリのコードでログイン)</button>
      </div>
      <p id="totp-login-result" class="muted" aria-live="polite"></p>
    </details>
  </div>

  <div id="auth-logged-in" class="hidden">
    <p>Logged in as (ログイン中): <strong id="account-email-label"></strong></p>
    <div class="buttons">
      <button id="logout-btn" class="secondary">Log out (ログアウト)</button>
    </div>

    <details>
      <summary>Change contact info: Email 1, Email 2, phone (連絡先の変更)</summary>
      <p class="muted">確認リンクは現在の主メールアドレス(メール1)宛にのみ送信されます。 /
        The confirmation link is sent only to your current primary email (Email 1).</p>
      <div class="form-grid">
        <div>
          <label for="change-email-field">変更する項目 / Field to change</label>
          <select id="change-email-field">
            <option value="email">メール1(主) / Email 1 (primary)</option>
            <option value="backup_email">メール2(セカンド) / Email 2 (backup)</option>
            <option value="phone">携帯電話番号 / Phone number</option>
          </select>
        </div>
        <div>
          <label for="change-email-new">新しい値 / New value</label>
          <input id="change-email-new" type="text" />
        </div>
      </div>
      <div class="buttons">
        <button id="change-email-submit">Send confirmation email (確認メールを送信)</button>
      </div>
      <p id="change-email-result" class="muted" aria-live="polite"></p>
    </details>

    <details>
      <summary>Authenticator app 2FA (認証アプリによる2段階認証)</summary>
      <p class="muted">
        Google Authenticator・Authy等の認証アプリを使った第二要素を追加できます。
        有効化すると、次回ログインからメール/SMSのワンタイムパスワードに加えて
        認証アプリの6桁コードも必要になります。 /
        Add a second factor using an authenticator app such as Google Authenticator
        or Authy. Once enabled, logins require both the email/SMS one-time password
        and the 6-digit authenticator app code.
      </p>
      <div class="buttons">
        <button id="totp-setup-btn">Start setup (セットアップを開始)</button>
        <button id="totp-disable-btn" class="secondary">Disable 2FA (2FAを無効化)</button>
      </div>
      <p class="muted">Secret (シークレット): <code id="totp-secret"></code></p>
      <p class="muted">URI: <code id="totp-uri"></code></p>
      <div id="totp-enable-row" class="form-grid hidden">
        <div>
          <label for="totp-confirm-code">認証アプリに表示された6桁コード / 6-digit code from your authenticator app</label>
          <input id="totp-confirm-code" type="text" inputmode="numeric" placeholder="123456" />
        </div>
      </div>
      <div class="buttons">
        <button id="totp-enable-btn">Enable 2FA (2FAを有効化)</button>
      </div>
      <p id="totp-result" class="muted" aria-live="polite"></p>
    </details>
  </div>
</section>

<section id="freedomain-section">
  <h2>Easy Free-Domain Setup, DuckDNS (簡単ドメイン設定)</h2>
  <p class="muted">
    固定IPではないDDNS環境向けに、無料サブドメイン(DuckDNS)の取得〜自動更新を
    open-web-server側で一気通貫にセットアップします。 / For non-static-IP DDNS
    environments, set up a free DuckDNS subdomain with automatic renewal on the
    open-web-server side, end to end.
  </p>
  <p class="muted">
    ① まずDuckDNS(<a href="https://www.duckdns.org/" target="_blank" rel="noopener noreferrer">duckdns.org</a>)
    でアカウント作成(GitHub/Google/Reddit等のOAuthログイン)し、トークンを取得してください——
    このアカウント作成自体はこのソフトウェアから自動化できません(他社サービスの認証情報を
    代行取得しない方針のため)。 / ① First create an account at
    <a href="https://www.duckdns.org/" target="_blank" rel="noopener noreferrer">duckdns.org</a>
    (via GitHub/Google/Reddit OAuth login) and obtain your token — account creation itself
    cannot be automated by this software (we do not acquire third-party credentials on your behalf).
  </p>
  <div class="form-grid">
    <div>
      <label for="freedomain-server-url">open-web-serverのURL / open-web-server URL</label>
      <input id="freedomain-server-url" type="text" placeholder="例: http://127.0.0.1:8080" />
    </div>
    <div>
      <label for="freedomain-admin-token">open-web-serverの管理トークン / open-web-server admin token</label>
      <input id="freedomain-admin-token" type="password" placeholder="x-admin-token" />
    </div>
  </div>

  <h3>Registered domains (登録済みドメイン一覧)</h3>
  <p class="muted">
    1インスタンスにつき最大20ドメインまで登録・自動更新できます。 / Up to 20 domains
    can be registered and auto-renewed per instance.
  </p>
  <div class="buttons">
    <button id="freedomain-list-fetch-btn" class="secondary">Refresh list (一覧を更新)</button>
  </div>
  <div id="freedomain-domain-list"></div>
  <p id="freedomain-list-result" class="muted" aria-live="polite"></p>

  <h3>Add a domain (ドメインを追加)</h3>
  <div class="form-grid">
    <div>
      <label for="freedomain-duckdns-domain">② 希望サブドメイン名 / Desired subdomain name</label>
      <input id="freedomain-duckdns-domain" type="text" placeholder="例: myhost (→ myhost.duckdns.org)" />
    </div>
    <div>
      <label for="freedomain-duckdns-token">DuckDNSトークン / DuckDNS token</label>
      <input id="freedomain-duckdns-token" type="password" placeholder="duckdns.orgのアカウントページから取得" />
    </div>
  </div>
  <div class="buttons">
    <button id="freedomain-setup-btn">③ Add &amp; verify (追加&疎通確認)</button>
  </div>
  <p id="freedomain-result" class="muted" aria-live="polite" style="white-space: pre-line"></p>

  <div id="freedomain-sftp-step" class="hidden">
    <h3>④ Example SFTP connection command (SFTP接続コマンド例)</h3>
    <div class="form-grid">
      <div>
        <label for="freedomain-sftp-host-select">SFTP接続に使うドメイン(任意) / Domain to use for SFTP (optional)</label>
        <select id="freedomain-sftp-host-select">
          <option value="">(自動選択 / auto-select)</option>
        </select>
      </div>
    </div>
    <div class="buttons">
      <button id="freedomain-sftp-fetch-btn" class="secondary">Fetch SFTP connection info (SFTP接続情報を取得)</button>
    </div>
    <p id="freedomain-sftp-result" class="muted" aria-live="polite"></p>
  </div>
</section>

<section id="external-tools-section">
  <h2>External Tools (外部ツール)</h2>
  <p class="muted">
    open-easy-webの管理対象外(別バイナリ・別ポートで動作)だが、日常運用で
    よく使う社内向けWEBアプリをここに登録し、ワンクリックで呼び出せるように
    します。 / Web apps that open-easy-web does not manage itself (they run as
    separate binaries on separate ports) but are used often during day-to-day
    operations, registered here for one-click access.
  </p>
  <div class="form-grid">
    <div>
      <label for="ext-tool-rs-sync-url">RS-Sync URL (GitHub複数アカウント・複数プロバイダのリポジトリ同期ツール)</label>
      <input id="ext-tool-rs-sync-url" type="text" value="https://easy-web.tokyo/rs-sync/" placeholder="例: https://easy-web.tokyo/rs-sync/" />
    </div>
  </div>
  <div class="buttons">
    <button
      id="ext-tool-rs-sync-launch"
      onclick="window.open((document.getElementById('ext-tool-rs-sync-url').value || 'https://easy-web.tokyo/rs-sync/'), '_blank', 'noopener,noreferrer')"
    >🔗 RS-Syncを起動 / Launch RS-Sync</button>
  </div>
  <p class="muted">
    RS-Sync(<a href="https://github.com/aon-co-jp/rs-sync" target="_blank" rel="noopener noreferrer">aon-co-jp/rs-sync</a>)は、
    GitHub・open-gitea・Gitea・Gitbucketなど複数アカウント/複数プロバイダに
    またがるリポジトリの一方向/双方向ミラー同期を行うRust+Poem製Webアプリ。
    既定では<code>https://easy-web.tokyo/rs-sync/</code>(VPS上でopen-web-server
    自身の「分身の術」テナントルーティング〈`domains.toml`、
    <code>POST /admin/tenants</code>で登録〉経由で`127.0.0.1:8096`へ
    転送)で稼働中のインスタンスを指す(2026-07-28、runo.tokyo上のデモ
    インスタンスは廃止しeasy-web.tokyoへ移設した)。別の場所で動かしている
    場合は上記URLを書き換えてください。デモ環境は
    <a href="https://easy-web.tokyo/rs-sync/demo" target="_blank" rel="noopener noreferrer">https://easy-web.tokyo/rs-sync/demo</a>
    (現状は本番と同一バックエンドを指すエイリアスで、独立したデモ専用
    データはまだ無いと正直に開示)。 /
    RS-Sync is a Rust+Poem web app that mirrors repositories one-way or
    two-way across multiple GitHub/open-gitea/Gitea/Gitbucket accounts. By
    default this points at the instance running at
    <code>https://easy-web.tokyo/rs-sync/</code> (routed via open-web-server's
    own "ninja clone" tenant routing, registered in <code>domains.toml</code>
    via <code>POST /admin/tenants</code>, forwarding to
    <code>127.0.0.1:8096</code> on the VPS; the old runo.tokyo demo instance
    was decommissioned 2026-07-28 and moved here). Adjust the URL above if
    you run RS-Sync elsewhere. A demo link is available at
    <a href="https://easy-web.tokyo/rs-sync/demo" target="_blank" rel="noopener noreferrer">https://easy-web.tokyo/rs-sync/demo</a>
    (honest disclosure: currently just an alias pointing at the same
    backend as production, not an isolated demo dataset).
  </p>

  <div class="form-grid">
    <div>
      <label for="ext-tool-open-redmine-url">open-redmine URL (Redmine互換のRust製チケット管理ツール)</label>
      <input id="ext-tool-open-redmine-url" type="text" value="https://easy-web.tokyo/open-redmine/" placeholder="例: https://easy-web.tokyo/open-redmine/" />
    </div>
  </div>
  <div class="buttons">
    <button
      id="ext-tool-open-redmine-launch"
      onclick="window.open((document.getElementById('ext-tool-open-redmine-url').value || 'https://easy-web.tokyo/open-redmine/'), '_blank', 'noopener,noreferrer')"
    >🔗 open-redmineを起動 / Launch open-redmine</button>
  </div>
  <p class="muted">
    open-redmine(<a href="https://github.com/aon-co-jp/open-redmine" target="_blank" rel="noopener noreferrer">aon-co-jp/open-redmine</a>)は、
    Redmine互換のRust+Poem製チケット/プロジェクト管理ツール。既定では
    <code>https://easy-web.tokyo/open-redmine/</code>(open-web-serverの
    「分身の術」テナントルーティング経由で`127.0.0.1:8100`へ転送)で
    稼働中のインスタンスを指す(2026-07-28、runo.tokyo側のテナント登録は
    削除しeasy-web.tokyoに一本化)。別の場所で動かしている場合は上記URLを
    書き換えてください。デモ環境は
    <a href="https://easy-web.tokyo/open-redmine/demo" target="_blank" rel="noopener noreferrer">https://easy-web.tokyo/open-redmine/demo</a>
    (現状は本番と同一バックエンドを指すエイリアスで、独立したデモ専用
    データはまだ無いと正直に開示)。 /
    open-redmine (<a href="https://github.com/aon-co-jp/open-redmine" target="_blank" rel="noopener noreferrer">aon-co-jp/open-redmine</a>)
    is a Redmine-compatible Rust+Poem ticket/project-tracking tool. By
    default this points at the instance running at
    <code>https://easy-web.tokyo/open-redmine/</code> (routed via
    open-web-server's "ninja clone" tenant routing, forwarding to
    <code>127.0.0.1:8100</code> on the VPS; the runo.tokyo tenant entry was
    removed 2026-07-28, consolidating on easy-web.tokyo). Adjust the URL
    above if you run open-redmine elsewhere. A demo link is available at
    <a href="https://easy-web.tokyo/open-redmine/demo" target="_blank" rel="noopener noreferrer">https://easy-web.tokyo/open-redmine/demo</a>
    (honest disclosure: currently just an alias pointing at the same
    backend as production, not an isolated demo dataset).
  </p>

  <div class="form-grid">
    <div>
      <label for="ext-tool-rs-git-url">open-gitea URL (自前運用Gitフォージ、旧RS-Git)</label>
      <input id="ext-tool-rs-git-url" type="text" value="https://easy-web.tokyo/open-gitea/ui/" placeholder="例: https://easy-web.tokyo/open-gitea/ui/" />
    </div>
  </div>
  <div class="buttons">
    <button
      id="ext-tool-rs-git-launch"
      onclick="window.open((document.getElementById('ext-tool-rs-git-url').value || 'https://easy-web.tokyo/open-gitea/ui/'), '_blank', 'noopener,noreferrer')"
    >🔗 open-giteaを起動 / Launch open-gitea</button>
  </div>
  <p class="muted">
    open-gitea(<a href="https://github.com/aon-co-jp/open-gitea" target="_blank" rel="noopener noreferrer">aon-co-jp/open-gitea</a>、
    2026-07-27にRS-Gitから改名)は、Rust+Poem製の自前運用Gitフォージ
    (セルフホスト版GitHub相当、Gitea(Go製)のRust版を目指す)。既定では
    <code>https://easy-web.tokyo/open-gitea/ui/</code>(open-web-serverの
    「分身の術」テナントルーティング経由で`127.0.0.1:8090`へ転送。UIは
    アプリ側の設計で`/ui/`配下にあるため末尾に`ui/`が必要)で稼働中の
    インスタンスを指す。別の場所で動かしている場合は上記URLを
    書き換えてください。 /
    open-gitea (<a href="https://github.com/aon-co-jp/open-gitea" target="_blank" rel="noopener noreferrer">aon-co-jp/open-gitea</a>,
    renamed from RS-Git on 2026-07-27) is a self-hosted Rust+Poem Git forge
    (a self-hosted GitHub equivalent, aiming for a Rust take on Gitea).
    By default this points at
    <code>https://easy-web.tokyo/open-gitea/ui/</code> (routed via
    open-web-server's "ninja clone" tenant routing, forwarding to
    <code>127.0.0.1:8090</code> on the VPS; the trailing <code>ui/</code> is
    required because the app itself mounts its UI under that path). Adjust
    the URL above if you run open-gitea elsewhere.
  </p>
</section>

<section id="site-ops-section" class="hidden">
  <h2>Create Folder / Upload Files (フォルダー作成 / アップロード)</h2>
  <p class="muted">
    ① まずフォルダーを作成します。 Create a folder for your site first。<br />
    ② ファイルを選択してアップロードします。 Then select and upload your files。<br />
    ③ 🤖 AIがPHPサイトかどうかを自動判定し、PHPと判定されればnginx+PHP-FPMを
    自動構成します。 AI automatically detects whether it's a PHP site and configures
    nginx+PHP-FPM if so。
  </p>
  <div class="form-grid">
    <div>
      <label for="site-ops-name">サイト名(ドメイン等) / Site name (e.g. domain)</label>
      <input id="site-ops-name" type="text" placeholder="example.tokyo" />
    </div>
  </div>
  <div class="buttons">
    <button id="site-ops-create-folder">① Create folder (フォルダー作成)</button>
  </div>
  <div class="form-grid">
    <div>
      <label for="site-ops-files">② Select files (ファイル選択)</label>
      <input id="site-ops-files" type="file" multiple />
    </div>
  </div>
  <div class="buttons">
    <button id="site-ops-upload">Upload (アップロード)</button>
    <button id="site-ops-detect">③ 🤖 AI detect &amp; auto-configure (AI判定&自動構成)</button>
  </div>
  <p id="site-ops-result" class="muted" aria-live="polite"></p>
  <div id="site-ops-correction" class="hidden">
    <p>Was this detection correct? (この判定は正しいですか?)</p>
    <div class="buttons">
      <button id="site-ops-correct-yes" class="secondary">Correct, PHP (正しいです)</button>
      <button id="site-ops-correct-no" class="secondary">Not PHP (違います)</button>
    </div>
  </div>
</section>

<p id="status" class="muted" aria-live="polite"></p>
"#;

#[cfg(test)]
mod tests {
    use super::SHELL_HTML;

    /// 2026-07-29追記: 本番ページ自体(open-easy-webのapp-header)に
    /// `/demo`へのデモ案内リンクを日英併記で追加したことの回帰確認。
    #[test]
    fn shell_html_links_to_demo_environment_bilingually() {
        assert!(SHELL_HTML.contains(r#"<a href="/demo">https://easy-web.tokyo/demo</a>"#));
        assert!(SHELL_HTML.contains("これは管理者向け本番環境です"));
        assert!(SHELL_HTML.contains("This is the admin/production environment"));
    }

    /// 2026-07-27追記: RS-Sync(GitHub複数アカウント/複数プロバイダ同期
    /// ツール)を「外部ツール」セクションへワンクリック起動リンクとして
    /// 登録したことの回帰確認。
    #[test]
    fn shell_html_registers_rs_sync_as_a_launchable_external_tool() {
        assert!(SHELL_HTML.contains(r#"id="external-tools-section""#));
        assert!(SHELL_HTML.contains(r#"id="ext-tool-rs-sync-url""#));
        assert!(SHELL_HTML.contains(r#"id="ext-tool-rs-sync-launch""#));
        assert!(SHELL_HTML.contains("https://easy-web.tokyo/rs-sync/"));
        assert!(SHELL_HTML.contains("https://easy-web.tokyo/rs-sync/demo"));
        assert!(SHELL_HTML.contains("aon-co-jp/rs-sync"));
    }

    /// 2026-07-27追記(続き): open-redmineを「外部ツール」セクションへ
    /// ワンクリック起動リンクとして登録したことの回帰確認
    /// (ユーザー指示: 「easy-web.tokyo/open-redmineとして登録して
    /// easy-web.tokyoからクリックで使える様にして」)。
    #[test]
    fn shell_html_registers_open_redmine_as_a_launchable_external_tool() {
        assert!(SHELL_HTML.contains(r#"id="ext-tool-open-redmine-url""#));
        assert!(SHELL_HTML.contains(r#"id="ext-tool-open-redmine-launch""#));
        assert!(SHELL_HTML.contains("https://easy-web.tokyo/open-redmine/"));
        assert!(SHELL_HTML.contains("https://easy-web.tokyo/open-redmine/demo"));
        assert!(SHELL_HTML.contains("aon-co-jp/open-redmine"));
    }

    /// 2026-07-27追記(続き2): RS-Gitを「外部ツール」セクションへ
    /// ワンクリック起動リンクとして登録したことの回帰確認(ユーザー指示:
    /// 「easy-web.tokyoのTOPページにリンク集で、open-redmineとrs-syncと
    /// RGitなどへのリンクと実装をお願いします」)。
    #[test]
    fn shell_html_registers_rs_git_as_a_launchable_external_tool() {
        assert!(SHELL_HTML.contains(r#"id="ext-tool-rs-git-url""#));
        assert!(SHELL_HTML.contains(r#"id="ext-tool-rs-git-launch""#));
        assert!(SHELL_HTML.contains("https://easy-web.tokyo/open-gitea/ui/"));
        assert!(SHELL_HTML.contains("aon-co-jp/open-gitea"));
    }

    /// 2026-07-27追記: aruaru-db・open-raid-zが「忍者の分身の術」パターンに
    /// どう当てはまる/当てはまらないかの説明文(日英併記)が「分身の術」
    /// フォーム欄の近くにあることの回帰確認。
    #[test]
    fn shell_html_explains_ninja_clone_pattern_for_aruaru_db_and_open_raid_z_bilingually() {
        assert!(SHELL_HTML.contains("忍者の分身の術"));
        assert!(SHELL_HTML.contains("ninja clone"));
        assert!(SHELL_HTML.contains("aruaru-db"));
        assert!(SHELL_HTML.contains("open-raid-z"));
    }
}
