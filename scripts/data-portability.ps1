# open-easy-webのサーバー側データのバックアップ/リストア(Windows版、
# scripts/data-portability.sh のPowerShell移植)。
#
# 対象データファイル(既定パス、環境変数で変更されていればそちらを使う):
#   - $env:OPEN_EASYWEB_USERS_STATE          (既定 C:\ProgramData\open-easy-web\.open-easy-web-users.json)
#   - $env:OPEN_EASYWEB_AI_STATE             (既定 C:\ProgramData\open-easy-web\.open-easy-web-ai-state.json)
#   - $env:OPEN_EASYWEB_AUTO_UPDATE_SETTINGS_FILE (既定 C:\ProgramData\open-easy-web\.open-easy-web-auto-update.json)
#
# 保存先: local(tar.gz) / github(リポジトリへpush/pull) / rclone
# (Googleドライブ等、事前に `rclone config` 済みであることが前提——
# このスクリプトからOAuth認証を代行しない、既存方針通り)。
#
# 使い方:
#   .\data-portability.ps1 backup local  C:\backup\open-easy-web.tar.gz
#   .\data-portability.ps1 backup github <repo-url> <branch> <github-token>
#   .\data-portability.ps1 backup rclone gdrive:backups/open-easy-web.tar.gz
#   .\data-portability.ps1 restore local  C:\backup\open-easy-web.tar.gz
#   .\data-portability.ps1 restore github <repo-url> <branch> <github-token>
#   .\data-portability.ps1 restore rclone gdrive:backups/open-easy-web.tar.gz

param(
    [Parameter(Mandatory=$true)][ValidateSet("backup","restore")][string]$Command,
    [Parameter(Mandatory=$true)][ValidateSet("local","github","rclone")][string]$Mode,
    [Parameter(Mandatory=$true)][string]$Target,
    [string]$Branch = "main",
    [string]$GithubToken = ""
)

$ErrorActionPreference = "Stop"

function Get-DataPaths {
    $paths = @()
    $users = if ($env:OPEN_EASYWEB_USERS_STATE) { $env:OPEN_EASYWEB_USERS_STATE } else { "C:\ProgramData\open-easy-web\.open-easy-web-users.json" }
    $ai = if ($env:OPEN_EASYWEB_AI_STATE) { $env:OPEN_EASYWEB_AI_STATE } else { "C:\ProgramData\open-easy-web\.open-easy-web-ai-state.json" }
    $auto = if ($env:OPEN_EASYWEB_AUTO_UPDATE_SETTINGS_FILE) { $env:OPEN_EASYWEB_AUTO_UPDATE_SETTINGS_FILE } else { "C:\ProgramData\open-easy-web\.open-easy-web-auto-update.json" }
    foreach ($p in @($users, $ai, $auto)) {
        if (Test-Path $p) { $paths += $p }
    }
    return $paths
}

function Build-Archive {
    param([string]$DestTgz)
    $paths = Get-DataPaths
    if ($paths.Count -eq 0) {
        Write-Warning "バックアップ対象のデータファイルが見つかりませんでした(新規インストール直後などは正常です)。"
    }
    $tmpRoot = Join-Path $env:TEMP ("oew-backup-" + [guid]::NewGuid())
    New-Item -ItemType Directory -Force -Path $tmpRoot | Out-Null
    foreach ($p in $paths) {
        Copy-Item $p -Destination $tmpRoot -Force
    }
    tar czf $DestTgz -C $tmpRoot .
    Remove-Item -Recurse -Force $tmpRoot
    Write-Host "==> バックアップ作成完了: $DestTgz"
}

function Extract-Archive {
    param([string]$SrcTgz)
    Write-Host "==> $SrcTgz を展開し、既定のデータディレクトリへ復元します"
    $destDir = "C:\ProgramData\open-easy-web"
    New-Item -ItemType Directory -Force -Path $destDir | Out-Null
    tar xzf $SrcTgz -C $destDir
    Write-Host "==> 復元完了($destDir 配下)"
}

switch ($Command) {
    "backup" {
        switch ($Mode) {
            "local" { Build-Archive -DestTgz $Target }
            "github" {
                $archiveTmp = Join-Path $env:TEMP "oew-backup.tar.gz"
                Build-Archive -DestTgz $archiveTmp
                $repoDir = Join-Path $env:TEMP ("oew-repo-" + [guid]::NewGuid())
                $authUrl = $Target -replace "https://", "https://$($GithubToken):@"
                git clone --branch $Branch $authUrl $repoDir 2>$null
                if (-not (Test-Path $repoDir)) { git clone $authUrl $repoDir }
                Copy-Item $archiveTmp -Destination (Join-Path $repoDir "open-easy-web-backup.tar.gz") -Force
                Push-Location $repoDir
                git add open-easy-web-backup.tar.gz
                git -c user.email="open-easy-web@localhost" -c user.name="open-easy-web" commit -m "backup $(Get-Date -AsUTC -Format o)" --allow-empty
                git push origin $Branch
                Pop-Location
                Remove-Item -Recurse -Force $repoDir
                Write-Host "==> GitHub($Target, branch=$Branch)へバックアップをpushしました"
            }
            "rclone" {
                if (-not (Get-Command rclone -ErrorAction SilentlyContinue)) {
                    Write-Error "rcloneがインストールされていません。事前に 'rclone config' でリモート(Googleドライブ等)を設定してください。"
                }
                $archiveTmp = Join-Path $env:TEMP "oew-backup.tar.gz"
                Build-Archive -DestTgz $archiveTmp
                rclone copyto $archiveTmp $Target
                Write-Host "==> rclone経由で $Target へバックアップを保存しました"
            }
        }
    }
    "restore" {
        switch ($Mode) {
            "local" { Extract-Archive -SrcTgz $Target }
            "github" {
                $repoDir = Join-Path $env:TEMP ("oew-repo-" + [guid]::NewGuid())
                $authUrl = $Target -replace "https://", "https://$($GithubToken):@"
                git clone --branch $Branch --depth 1 $authUrl $repoDir
                Extract-Archive -SrcTgz (Join-Path $repoDir "open-easy-web-backup.tar.gz")
                Remove-Item -Recurse -Force $repoDir
            }
            "rclone" {
                if (-not (Get-Command rclone -ErrorAction SilentlyContinue)) {
                    Write-Error "rcloneがインストールされていません。"
                }
                $archiveTmp = Join-Path $env:TEMP "oew-backup.tar.gz"
                rclone copyto $Target $archiveTmp
                Extract-Archive -SrcTgz $archiveTmp
            }
        }
    }
}
