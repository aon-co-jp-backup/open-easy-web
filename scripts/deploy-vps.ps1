# VPSレンタルサーバーへ open-easy-web (と任意で open-web-server) をアップロードし、
# IPアドレスから起動するための Windows PowerShell スクリプト。
#
# 前提:
#   - Windows 10 (1809以降) / Windows 11 には OpenSSH クライアント
#     (ssh.exe/scp.exe) が標準搭載されている。無ければ
#     「設定 > アプリ > オプション機能 > OpenSSHクライアント」から追加する。
#   - VPS側はUbuntu/Debian系を想定し、root(または sudo 可能なユーザー)で
#     SSH接続できること。
#   - アップロードせずローカルだけで使う場合は、このスクリプトは不要。
#     scripts/serve.sh を直接ローカルで実行すればよい(下記README参照)。
#
# 使い方の例(PowerShellから):
#   .\scripts\deploy-vps.ps1 -VpsHost 203.0.113.10 -VpsUser root
#   .\scripts\deploy-vps.ps1 -VpsHost 203.0.113.10 -VpsUser root `
#       -OpenWebServerPath "F:\open-runo\open-web-server"
#
# パラメータ:
#   -VpsHost             VPSのIPアドレスまたはドメイン(必須)
#   -VpsUser             SSH接続ユーザー名(既定: root)
#   -VpsPort             SSHポート(既定: 22)
#   -RemoteAruaruPath    open-easy-webのアップロード先(既定: /root/RUNO/open-easy-web)
#   -OpenWebServerPath   ローカルのopen-web-serverディレクトリ(指定時のみ
#                        /root/open-web-server へ同時アップロードする)
#   -SkipBuild           指定すると、ローカルでの `cargo build`/`wasm-bindgen`
#                        を省略する(既に pkg/ をビルド済みの場合)
#   -StartServer         指定すると、アップロード後にSSH経由で
#                        scripts/serve.sh 0.0.0.0 8080 をバックグラウンド起動する

param(
    [Parameter(Mandatory = $true)][string]$VpsHost,
    [string]$VpsUser = "root",
    [int]$VpsPort = 22,
    [string]$RemoteAruaruPath = "/root/RUNO/open-easy-web",
    [string]$OpenWebServerPath = "",
    [switch]$SkipBuild,
    [switch]$StartServer
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot

if (-not $SkipBuild) {
    Write-Host "ローカルでビルドします(cargo build + wasm-bindgen)..." -ForegroundColor Cyan
    Push-Location $RepoRoot
    try {
        cargo build --target wasm32-unknown-unknown
        wasm-bindgen --target web --no-typescript --out-dir pkg `
            target/wasm32-unknown-unknown/debug/open_easy_web.wasm
    }
    finally {
        Pop-Location
    }
}

Write-Host "VPS上に $RemoteAruaruPath を作成します..." -ForegroundColor Cyan
ssh -p $VpsPort "$VpsUser@$VpsHost" "mkdir -p $RemoteAruaruPath"

Write-Host "open-easy-web を $VpsUser@${VpsHost}:$RemoteAruaruPath へアップロードします..." -ForegroundColor Cyan
scp -P $VpsPort -r `
    "$RepoRoot\index.html" `
    "$RepoRoot\pkg" `
    "$RepoRoot\scripts" `
    "$RepoRoot\deploy" `
    "$RepoRoot\Cargo.toml" `
    "$RepoRoot\src" `
    "$VpsUser@${VpsHost}:$RemoteAruaruPath/"

if ($OpenWebServerPath -ne "") {
    if (-not (Test-Path $OpenWebServerPath)) {
        Write-Warning "OpenWebServerPath '$OpenWebServerPath' が見つかりません。スキップします。"
    }
    else {
        Write-Host "VPS上に /root/open-web-server を作成します..." -ForegroundColor Cyan
        ssh -p $VpsPort "$VpsUser@$VpsHost" "mkdir -p /root/open-web-server"
        Write-Host "open-web-server を $VpsUser@${VpsHost}:/root/open-web-server へアップロードします..." -ForegroundColor Cyan
        scp -P $VpsPort -r "$OpenWebServerPath\*" "$VpsUser@${VpsHost}:/root/open-web-server/"
    }
}

if ($StartServer) {
    Write-Host "VPS上で scripts/serve.sh を起動します(0.0.0.0:8080)..." -ForegroundColor Cyan
    ssh -p $VpsPort "$VpsUser@$VpsHost" `
        "cd $RemoteAruaruPath && chmod +x scripts/*.sh && nohup bash scripts/serve.sh 0.0.0.0 8080 > serve.log 2>&1 & disown"
    Write-Host "起動しました。ブラウザで http://${VpsHost}:8080/index.html を開いて確認してください。" -ForegroundColor Green
}
else {
    Write-Host "アップロード完了。VPSにSSHして起動する場合:" -ForegroundColor Green
    Write-Host "  ssh $VpsUser@$VpsHost"
    Write-Host "  cd $RemoteAruaruPath && bash scripts/serve.sh 0.0.0.0 8080"
    Write-Host "起動後、ブラウザで http://${VpsHost}:8080/index.html を開いて確認してください。"
}
