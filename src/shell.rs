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
        共有バックエンドへの登録(任意、2026-07-16新設・「分身の術」構想):
        既に稼働中のopen-web-server/poem-cosmo-tauri/aruaru-llmインスタンス
        へこのドメインを動的登録し、ドメインごとの個別インストールを
        不要にします。管理APIのURLを入力すると、一覧のカードに
        「🔗 共有バックエンドへ登録」ボタンが表示されます。
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
      <label for="site-shared-db-uri">DB connection string, required for open-web-server only (DB接続文字列)</label>
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
