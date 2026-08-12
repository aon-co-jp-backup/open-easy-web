# open-easy-web(open-easy-web-server)インストールスクリプト(Windows /
# Windows Server 共通)。
#
# 正直な開示: このバイナリは固定アカウント制の認証を持ち、起動時に
# 環境変数 `OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL` が未設定だと即座に
# 終了する(誰もログインできない状態でサイレントに動き続けるより起動
# 失敗のほうが安全という設計判断、詳細はCLAUDE.md参照)。WASM
# フロントエンド(`pkg/`+`index.html`)は別途ビルドが必要で、この
# zipには含まれない(バックエンドAPIサーバーのみを配布対象とする)。
#
# 使い方(管理者権限のPowerShellで):
#   Invoke-WebRequest -Uri "https://github.com/aon-co-jp/open-easy-web/releases/latest/download/open-easy-web-server-windows-x86_64.zip" -OutFile open-easy-web-server.zip
#   Expand-Archive open-easy-web-server.zip -DestinationPath open-easy-web-server
#   cd open-easy-web-server
#   .\install.ps1

#Requires -RunAsAdministrator

$ErrorActionPreference = "Stop"

$InstallDir = "C:\Program Files\open-easy-web"
$ServiceName = "OpenEasyWeb"

Write-Host "==> インストール先: $InstallDir"
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

$BinSrc = Join-Path $PSScriptRoot "open-easy-web-server.exe"
if (-not (Test-Path $BinSrc)) {
    Write-Error "open-easy-web-server.exe が見つかりません($BinSrc)。zipを展開したディレクトリで実行してください。"
    exit 1
}
Copy-Item $BinSrc -Destination $InstallDir -Force

$existing = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($existing) {
    Write-Host "==> 既存のWindowsサービスが見つかったため、バイナリのみ更新しました(再起動は行いません)"
    Write-Host "    手動で再起動する場合: Restart-Service $ServiceName"
} else {
    Write-Host "==> Windowsサービスとして登録する場合の手順(必須環境変数を先に設定すること):"
    Write-Host "      [Environment]::SetEnvironmentVariable('OPEN_EASYWEB_SERVER_BIND', '0.0.0.0:8090', 'Machine')"
    Write-Host "      [Environment]::SetEnvironmentVariable('OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL', 'you@example.com', 'Machine')"
    Write-Host "      New-Service -Name $ServiceName -BinaryPathName '$InstallDir\open-easy-web-server.exe' -DisplayName 'open-easy-web' -StartupType Automatic"
    Write-Host "      Start-Service $ServiceName"
}

Write-Host ""
Write-Host "==> 既存の関連DATAを取り込みますか?(2026-07-29追記、ユーザー指示)"
$restoreAnswer = Read-Host "    取り込みますか? [y/N]"
if ($restoreAnswer -match '^[yY]') {
    Write-Host "    復元元を選んでください: 1) ローカルのtar.gz  2) GitHubリポジトリ  3) rclone(Googleドライブ等)"
    $kind = Read-Host "    番号を入力 [1-3]"
    $portabilityScript = Join-Path $PSScriptRoot "scripts\data-portability.ps1"
    switch ($kind) {
        "1" {
            $p = Read-Host "    tar.gzのパス"
            & $portabilityScript restore local $p
        }
        "2" {
            $repoUrl = Read-Host "    リポジトリURL"
            $branch = Read-Host "    ブランチ [main]"
            if ([string]::IsNullOrWhiteSpace($branch)) { $branch = "main" }
            $ghToken = Read-Host "    GitHubトークン"
            & $portabilityScript restore github $repoUrl -Branch $branch -GithubToken $ghToken
        }
        "3" {
            $remote = Read-Host "    rcloneリモート(例: gdrive:backups/open-easy-web.tar.gz)"
            & $portabilityScript restore rclone $remote
        }
        default {
            Write-Host "    選択が無効なため、データ復元をスキップします。"
        }
    }
} else {
    Write-Host "    データ復元はスキップしました(新規インストールとして続行)。"
}

Write-Host ""
Write-Host "==> アンインストール時にデータを退避したい場合は uninstall.ps1 を使ってください。"
Write-Host "==> 完了。"
