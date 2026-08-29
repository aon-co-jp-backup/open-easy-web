; open-easy-web-install.exe — Inno Setup script (2026-08-19新設、2026-08-20正式採用)
;
; ユーザー指示「{リポジトリ名}-install.exe用のInno Setup .issファイルを
; 作成する」への対応。このリポジトリの実際の配布形態は
; install.ps1(Windowsサービス`OpenEasyWeb`として登録、C:\Program
; Files\open-easy-web へ配置)であり、Inno Setup自体は正式なアンインストーラー
; (unins000.exe)を持たない設計だった(self_update.rs等のコメント参照)。
; 本スクリプトは、既存のinstall.ps1/uninstall.ps1をそのまま呼び出す
; 薄いラッパーとしてインストーラーGUIを提供する(install.ps1のロジック自体は
; 変更しない、二重実装を避けるため)。
;
; 【2026-08-20 正式採用の経緯】かつて併存していた
; `installer/windows/open-easy-web.iss`(PrivilegesRequired=lowestの
; 単体プロセス起動方式)は削除された(ユーザー指示: 「open-easy-webの
; インストーラー方式は、単体プロセスというのはありえないです。様々な
; リポジトリを統括するサービスです。」)。open-easy-webは
; open-english・aruaru-llm等、複数の関連リポジトリを統括する中央サービス・
; 管理ハブという役割を持つため、ユーザーが手動起動する一時的なプロセス
; ではなく、常時稼働するWindowsサービスとして登録される本スクリプトの
; 設計が正しい。CLAUDE.mdのHANDOFF(2026-08-20)参照。
;
; ビルド方法: 事前に`cargo build --release --features
; acme,ddns,sftp,upnp`等で open-easy-web-server.exe を生成し、本ファイルと
; 同じ installer\ ディレクトリへ配置してから `iscc open-easy-web-install.iss`
; を実行すること。README-INSTALLED.txt(固定アカウントのメールアドレス
; 設定・自己アップデート機能の説明、日英併記)も同ディレクトリに同梱する。

#define MyAppName "open-easy-web"
#define MyAppVersion "0.1.0"
#define MyAppPublisher "aon-co-jp"
#define MyAppURL "https://github.com/aon-co-jp/open-easy-web"
#define MyServiceName "OpenEasyWeb"

[Setup]
AppId={{B6E2B6B1-6B7B-4C2E-9C77-OPENEASYWEB1}}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
DefaultDirName={autopf}\open-easy-web
DefaultGroupName=open-easy-web
DisableProgramGroupPage=yes
OutputDir=.
OutputBaseFilename=open-easy-web-installer
Compression=lzma2
SolidCompression=yes
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=admin
UninstallDisplayIcon={app}\open-easy-web-server.exe

[Languages]
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Files]
; ビルド成果物(release build)をこのディレクトリへ事前に配置してから
; iscc を実行すること。
Source: "open-easy-web-server.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\install.ps1"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\uninstall.ps1"; DestDir: "{app}"; Flags: ignoreversion
; 自己アップデート機能(auto_update.rs)が起動ファイル隣の version.json を
; 参照するため同梱する(無ければ「未インストール扱い」として自己
; アップデートが無効化される、既存の安全設計)。
Source: "version.json"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
; 固定アカウントのメールアドレス設定(必須)・自己アップデート機能の
; 説明(日英併記)。インストール後に表示する。
Source: "README-INSTALLED.txt"; DestDir: "{app}"; Flags: ignoreversion isreadme

[Run]
; install.ps1自体がサービス登録(New-Service)ロジックを持つため、
; 二重実装を避けてそのまま呼び出す。
Filename: "powershell.exe"; \
    Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\install.ps1"""; \
    WorkingDir: "{app}"; StatusMsg: "open-easy-web サービスをセットアップしています..."; \
    Flags: runhidden waituntilterminated

[UninstallRun]
Filename: "powershell.exe"; \
    Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\uninstall.ps1"""; \
    WorkingDir: "{app}"; RunOnceId: "UninstallOpenEasyWeb"; Flags: runhidden waituntilterminated
