; open-easy-web Windowsインストーラー(Inno Setup)。
;
; 参考実装: open-english/installer/windows/open-english.iss(同じ
; aon-co-jpエコシステムの姉妹プロジェクト、GitHub Releases経由の自動更新
; +Inno Setupインストーラーという同じ設計方針)。open-easy-webは常時稼働
; Webサーバー(バックエンド`open-easy-web-server`+WASMフロントエンド
; `pkg/`+`index.html`)であり、`server/src/auto_update.rs`に既に自己更新
; 機構(深夜0時の自動チェック+`/healthz`ベースのヘルスチェック、Windows
; 版は2026-08-19にプローブポートでのヘルスチェック+失敗時ロールバックを
; 追加済み)が実装されている——このインストーラーはその配布手段。
;
; 正直な開示: このアプリは起動時に`OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL`
; 環境変数が未設定だと即座に終了する設計(CLAUDE.md参照、誰もログイン
; できない状態でサイレントに動き続けるより起動失敗のほうが安全という
; 判断)。インストーラー自体はこの環境変数を対話的に設定する機能を
; 持たない(サイレント/無人インストールとの両立のため)——インストール後に
; 表示するREADME-INSTALLED.txtで設定手順を案内する。
;
; ビルド方法: ルート(`open-easy-web/`)で
;   cargo build --target wasm32-unknown-unknown --release
;   wasm-bindgen --target web --no-typescript --out-dir pkg
;     target/wasm32-unknown-unknown/release/open_easy_web.wasm
; `server/`で
;   cargo build --release
; を実行した後、このディレクトリで`ISCC.exe open-easy-web.iss`を実行する。

#define MyAppName "open-easy-web"
#ifndef MyAppVersion
  #define MyAppVersion "0.0.0-local-build"
#endif
#define MyAppPublisher "aon-co-jp"
#define MyAppURL "https://github.com/aon-co-jp/open-easy-web"
#define MyAppExeName "open-easy-web-server.exe"

[Setup]
; PrivilegesRequired=lowest: このアプリは管理者権限を必要とするシステム
; 領域書き込み・サービス登録等を行わない(Windowsサービスとして登録する
; 場合は別途`install.ps1`の管理者権限手順を使う、このインストーラーは
; 単体プロセスとしての起動のみを対象とする)。
PrivilegesRequired=lowest
AppId={{5A3E7C21-9F14-4C6D-8B02-1D6F3A8E5C77}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}/releases
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
UninstallDisplayIcon={app}\{#MyAppExeName}
Compression=lzma2
SolidCompression=yes
OutputDir=dist
; open-englishと同じ命名規則(バージョン番号なし、常に同じファイル名)に
; 揃える。
OutputBaseFilename=open-easy-web-install
ArchitecturesInstallIn64BitMode=x64compatible
DisableProgramGroupPage=yes

[Languages]
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Files]
Source: "..\..\server\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\index.html"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\pkg\*"; DestDir: "{app}\pkg"; Flags: ignoreversion recursesubdirs
Source: "README-INSTALLED.txt"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{app}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{app}"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Run]
; 実機E2E検証(open-english側の既知の教訓、2026-08-12発覚): `self_update`
; 系の機構が`/VERYSILENT`でこのインストーラーを起動して無人アップデート
; を行う場合、サーバー起動エントリに`skipifsilent`が付いていると更新後に
; サーバーが自動起動しない——ここでは意図的に`skipifsilent`を付けず、
; サイレント時も含め常にサーバーを起動する(open-easy-webは常時稼働
; サーバーであり、更新のたびに手動起動が必要になっては困るため)。
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#MyAppName}}"; Flags: nowait postinstall

[UninstallDelete]
Type: filesandordirs; Name: "{app}"
