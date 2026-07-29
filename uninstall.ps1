# open-easy-webのアンインストールスクリプト(Windows版、install.ps1の
# 対になるもの、2026-07-29新設)。
#
# アンインストール前にデータを退避するかどうかを尋ねる
# (data-portability.ps1参照、ローカル/GitHub/rclone〈Googleドライブ等〉
# から選択可能)。
#
# 使い方(管理者権限のPowerShellで): .\uninstall.ps1

#Requires -RunAsAdministrator

$ErrorActionPreference = "Stop"

$InstallDir = "C:\Program Files\open-easy-web"
$ServiceName = "OpenEasyWeb"

Write-Host "==> アンインストール前にデータを退避しますか?(推奨)"
$backupAnswer = Read-Host "    退避しますか? [Y/n]"
if ($backupAnswer -notmatch '^[nN]') {
    Write-Host "    保存先を選んでください: 1) ローカルのtar.gz  2) GitHubリポジトリ  3) rclone(Googleドライブ等)"
    $kind = Read-Host "    番号を入力 [1-3]"
    $portabilityScript = Join-Path $PSScriptRoot "scripts\data-portability.ps1"
    switch ($kind) {
        "1" {
            $default = "C:\open-easy-web-backup-$(Get-Date -Format yyyyMMddHHmmss).tar.gz"
            $p = Read-Host "    保存先パス [$default]"
            if ([string]::IsNullOrWhiteSpace($p)) { $p = $default }
            & $portabilityScript backup local $p
        }
        "2" {
            $repoUrl = Read-Host "    リポジトリURL(事前にGitHub側で公開/非公開を選んで作成しておいてください)"
            $branch = Read-Host "    ブランチ [main]"
            if ([string]::IsNullOrWhiteSpace($branch)) { $branch = "main" }
            $ghToken = Read-Host "    GitHubトークン"
            & $portabilityScript backup github $repoUrl -Branch $branch -GithubToken $ghToken
        }
        "3" {
            $remote = Read-Host "    rcloneリモート(例: gdrive:backups/open-easy-web.tar.gz)"
            & $portabilityScript backup rclone $remote
        }
        default {
            Write-Host "    選択が無効なため、データ退避をスキップします。"
        }
    }
} else {
    Write-Host "    データ退避をスキップします(この後のアンインストールで失われます)。"
}

Write-Host "==> サービスを停止・削除します"
$existing = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($existing) {
    Stop-Service -Name $ServiceName -ErrorAction SilentlyContinue
    sc.exe delete $ServiceName | Out-Null
}

if (Test-Path $InstallDir) {
    Write-Host "==> インストールディレクトリを削除($InstallDir)"
    Remove-Item -Recurse -Force $InstallDir
}

Write-Host "==> アンインストール完了。"
Write-Host "    データファイル自体(C:\ProgramData\open-easy-web 配下等)は削除していません。"
Write-Host "    完全に削除したい場合は手動で削除してください。"
