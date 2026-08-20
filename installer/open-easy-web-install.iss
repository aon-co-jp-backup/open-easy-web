; open-easy-web-install.exe — Inno Setup script (2026-08-19新設)
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
; ビルド方法: この開発機にInno Setup(ISCC.exe)は導入されていないため
; 未ビルド(正直な開示、CLAUDE.mdのHANDOFF参照)。ビルドする場合は、事前に
; `cargo build --release --features acme,ddns,sftp,upnp`等で
; open-easy-web-server.exe を生成し、本ファイルと同じ installer\ ディレクトリ
; へ配置してから `iscc open-easy-web-install.iss` を実行すること。

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
OutputBaseFilename=open-easy-web-install
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
