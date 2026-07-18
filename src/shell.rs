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

<div id="site-mgmt-section" class="hidden">
<section>
  <h2>ドメイン名・サブドメイン名の登録・編集・削除・選択切替</h2>
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
    <button id="site-export" class="secondary">エクスポート(JSON)</button>
    <button id="site-import-trigger" class="secondary">インポート(JSON)</button>
    <input id="site-import-file" type="file" accept="application/json" style="display:none" />
  </div>
</section>

<section>
  <h2>ドメイン・サブドメインを追加・編集</h2>
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
      <label for="site-name">サイト名</label>
      <input id="site-name" type="text" placeholder="例: 本番WordPress" />
    </div>
    <div>
      <label for="site-purpose">用途</label>
      <select id="site-purpose">
        <option value="self">このサイト(open-easy-web自身)</option>
        <option value="other">他のサイト</option>
      </select>
    </div>
    <div>
      <label for="site-protocol">プロトコル</label>
      <select id="site-protocol">
        <option value="https">https</option>
        <option value="http">http</option>
      </select>
    </div>
    <div>
      <label for="site-host">ホスト(IPアドレス / ドメイン / サブドメイン)</label>
      <input id="site-host" type="text" placeholder="例: 203.0.113.10 または example.com" />
    </div>
    <div>
      <label for="site-port">ポート</label>
      <input id="site-port" type="text" placeholder="443" value="443" />
    </div>
    <div>
      <label for="site-path">パス</label>
      <input id="site-path" type="text" placeholder="/" value="/" />
    </div>
    <div class="form-grid-full">
      <label for="site-stack">バックエンドスタック(自由記述・任意)</label>
      <input id="site-stack" type="text" placeholder="例: WordPress / PHP + Laravel / Python + FastAPI" />
    </div>
    <div>
      <label for="site-engine">配信エンジン(vhost)</label>
      <select id="site-engine">
        <option value="nginx">Nginx</option>
        <option value="apache">Apache</option>
        <option value="both">両方生成(未選択)</option>
        <option value="open-web-server" title="open-web-serverがApache＋Nginxのハイブリッド仕様のWebサーバーとしてまだ機能していない間は、配信エンジンではなくアプリケーションサーバー(Tomcat型)として扱ってください。">open-web-server</option>
      </select>
    </div>
    <div>
      <label for="site-app-server">アプリケーションサーバー(動的処理、Apache+Tomcat型)</label>
      <select id="site-app-server">
        <option value="none">なし(Webサーバー単体で動作)</option>
        <option value="open-runo">open-runo</option>
        <option value="poem-cosmo-tauri">poem-cosmo-tauri</option>
        <option value="aruaru-llm" title="契約不要の独自AIチャットコマース応答サービス(open-cudaとSET構成)。バックエンド接続先ではなくテナント登録のみ行う。">aruaru-llm(AIチャットコマース)</option>
      </select>
    </div>
    <div>
      <label for="site-app-server-upstream">アプリケーションサーバー接続先(host:port、任意)</label>
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
      <label for="site-shared-endpoint">共有バックエンド管理APIのURL(任意)</label>
      <input id="site-shared-endpoint" type="text" placeholder="例: http://127.0.0.1:8080" />
    </div>
    <div>
      <label for="site-shared-admin-key">共有バックエンドの管理キー(任意)</label>
      <input id="site-shared-admin-key" type="password" placeholder="x-admin-token / x-api-key" />
    </div>
    <div>
      <label for="site-shared-db-uri">DB接続文字列(open-web-server向けのみ必須)</label>
      <input id="site-shared-db-uri" type="text" placeholder="例: postgres://localhost/shop" />
    </div>
    <div>
      <label for="site-shared-session-token">open-easy-web-serverセッショントークン(任意)</label>
      <input id="site-shared-session-token" type="password" placeholder="Authorization: Bearer ..." />
    </div>
  </div>
  <div class="buttons">
    <button id="save-site">保存</button>
    <button id="clear-site-form" class="secondary">クリア</button>
  </div>
</section>
</div>

<section id="auth-section">
  <h2>アカウント / Account</h2>

  <div id="auth-logged-out">
    <p class="muted">
      セキュリティ上の理由により、新規登録は行っていません。あらかじめ
      登録済みの連絡先(メール1・メール2・携帯電話番号のいずれか)でのみ
      ログインできます。 / For security reasons, public registration is
      disabled. You can only log in with a pre-registered contact
      (Email 1, Email 2, or phone number).
    </p>

    <details open>
      <summary>ログイン(ワンタイムパスワード) / Login (one-time password)</summary>
      <div class="form-grid">
        <div>
          <label for="login-contact">メール1・メール2・電話番号のいずれか / Email 1, Email 2, or phone</label>
          <input id="login-contact" type="text" placeholder="you@example.com" />
        </div>
      </div>
      <div class="buttons">
        <button id="login-request-otp">コードを送信 / Send code</button>
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
        <button id="login-verify-otp">ログイン / Log in</button>
      </div>
      <p id="login-result" class="muted" aria-live="polite"></p>
    </details>

    <details>
      <summary>認証アプリのコードだけでログイン / Log in with just an authenticator app code</summary>
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
        <button id="totp-login-submit">認証アプリのコードでログイン / Log in with authenticator code</button>
      </div>
      <p id="totp-login-result" class="muted" aria-live="polite"></p>
    </details>
  </div>

  <div id="auth-logged-in" class="hidden">
    <p>ログイン中 / Logged in as: <strong id="account-email-label"></strong></p>
    <div class="buttons">
      <button id="logout-btn" class="secondary">ログアウト / Log out</button>
    </div>

    <details>
      <summary>連絡先の変更(メール1・メール2・電話番号) / Change contact info (Email 1, Email 2, phone)</summary>
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
        <button id="change-email-submit">確認メールを送信 / Send confirmation email</button>
      </div>
      <p id="change-email-result" class="muted" aria-live="polite"></p>
    </details>

    <details>
      <summary>認証アプリによる2段階認証(2FA) / Authenticator app 2FA</summary>
      <p class="muted">
        Google Authenticator・Authy等の認証アプリを使った第二要素を追加できます。
        有効化すると、次回ログインからメール/SMSのワンタイムパスワードに加えて
        認証アプリの6桁コードも必要になります。 /
        Add a second factor using an authenticator app such as Google Authenticator
        or Authy. Once enabled, logins require both the email/SMS one-time password
        and the 6-digit authenticator app code.
      </p>
      <div class="buttons">
        <button id="totp-setup-btn">セットアップを開始 / Start setup</button>
        <button id="totp-disable-btn" class="secondary">2FAを無効化 / Disable 2FA</button>
      </div>
      <p class="muted">シークレット / Secret: <code id="totp-secret"></code></p>
      <p class="muted">URI: <code id="totp-uri"></code></p>
      <div id="totp-enable-row" class="form-grid hidden">
        <div>
          <label for="totp-confirm-code">認証アプリに表示された6桁コード / 6-digit code from your authenticator app</label>
          <input id="totp-confirm-code" type="text" inputmode="numeric" placeholder="123456" />
        </div>
      </div>
      <div class="buttons">
        <button id="totp-enable-btn">2FAを有効化 / Enable 2FA</button>
      </div>
      <p id="totp-result" class="muted" aria-live="polite"></p>
    </details>
  </div>
</section>

<section id="site-ops-section" class="hidden">
  <h2>フォルダー作成 / アップロード — Create Folder / Upload Files</h2>
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
    <button id="site-ops-create-folder">① フォルダー作成 / Create folder</button>
  </div>
  <div class="form-grid">
    <div>
      <label for="site-ops-files">② ファイル選択 / Select files</label>
      <input id="site-ops-files" type="file" multiple />
    </div>
  </div>
  <div class="buttons">
    <button id="site-ops-upload">アップロード / Upload</button>
    <button id="site-ops-detect">③ 🤖 AI判定&自動構成 / AI detect & auto-configure</button>
  </div>
  <p id="site-ops-result" class="muted" aria-live="polite"></p>
  <div id="site-ops-correction" class="hidden">
    <p>この判定は正しいですか? / Was this detection correct?</p>
    <div class="buttons">
      <button id="site-ops-correct-yes" class="secondary">正しいです(PHP) / Correct (PHP)</button>
      <button id="site-ops-correct-no" class="secondary">違います(PHPではない) / Not PHP</button>
    </div>
  </div>
</section>

<p id="status" class="muted" aria-live="polite"></p>
"#;
