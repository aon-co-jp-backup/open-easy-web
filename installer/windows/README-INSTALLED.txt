open-easy-web をインストールしていただきありがとうございます。
Thank you for installing open-easy-web.

============================================================
【重要・必須】固定アカウントのメールアドレス設定
【IMPORTANT / REQUIRED】Fixed account email setup
============================================================

open-easy-web-server は起動時に環境変数 OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL
が設定されていないと、誰もログインできない不完全な状態でサイレントに
動き続けるのではなく、安全のため即座に終了する設計になっています。

open-easy-web-server refuses to start (exits immediately) unless the
OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL environment variable is set — this is a
deliberate safety choice over silently running with no way to log in.

設定例(PowerShell、管理者権限は不要):
Example (PowerShell, no admin rights required):

  [Environment]::SetEnvironmentVariable('OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL', 'you@example.com', 'User')
  [Environment]::SetEnvironmentVariable('OPEN_EASYWEB_SERVER_BIND', '0.0.0.0:8090', 'User')

設定後、いったんログアウト/ログインするか、新しいPowerShellウィンドウを
開いてからアプリを起動してください。
After setting these, either log out/in or open a new PowerShell window
before launching the app.

============================================================
自己アップデート機能について / About the self-update feature
============================================================

open-easy-web-server は起動中、深夜0時(UTC)にGitHub Releases
(https://github.com/aon-co-jp/open-easy-web/releases) の最新版を自動的に
確認します(既定はOFF、環境変数 OPEN_EASYWEB_AUTO_UPDATE=true で有効化、
または管理画面のトグルからも変更可能)。

While running, open-easy-web-server can automatically check GitHub
Releases (https://github.com/aon-co-jp/open-easy-web/releases) for a
newer version at local midnight (UTC). This is OFF by default — enable it
via the OPEN_EASYWEB_AUTO_UPDATE=true environment variable, or the admin
panel toggle.

新しいバージョンが見つかった場合:
When a newer version is found:

- Linux: SO_REUSEPORTを使い、新旧プロセスが同じポートを一時的に共有する
  ことで、実質的にダウンタイム無しで切り替わります。
  Linux uses SO_REUSEPORT so old and new processes briefly share the same
  port, achieving near-zero-downtime switchover.
- Windows: 新バイナリを一時的な別ポート(実ポート+1)でまず起動し、
  /healthz への到達を確認してから、実ポートでの正式な切り替えを行います。
  ヘルスチェックに失敗した場合は、新バイナリを起動せず旧バージョンの
  まま動作を継続します(自動ロールバック)。
  Windows first launches the new binary on a temporary probe port (real
  port + 1) and confirms it responds on /healthz before switching over on
  the real port. If the health check fails, the old process is never
  stopped and continues running (automatic rollback).

正直な開示: このロールバック機構は単体テスト・ローカルでの動作確認まで
検証済みですが、実際に「壊れたビルドをGitHub Releaseとして公開し、稼働中
のサーバーが実際にロールバックする」という一気通貫のE2E検証は、この
インストーラー作成セッションでは実施していません。
Honest disclosure: this rollback mechanism has been verified via unit
tests and local checks, but a full end-to-end scenario — publishing an
intentionally broken build as a GitHub Release and confirming a live
server actually rolls back — has not been performed in this
installer-authoring session.
