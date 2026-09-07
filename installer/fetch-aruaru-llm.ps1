# aruaru-llm(契約不要の独自AIチャットコマース応答サービス、open-cuda
# とSET構成)本体を、open-easy-webインストーラーの「一緒にインストール
# (任意)」タスクから取得するスクリプト(2026-09-07新設、open-english側
# の`fetch-aruaru-llm.ps1`〈2026-08-11新設〉と同一ロジック、open-easy-web
# 向けに文言のみ調整)。
#
# 正直な開示: これはaruaru-llmの実行ファイルのみを取得する。GPT-2/
# DistilGPT-2の実モデル重み(数百MB〜数GB)は含まない——モデル重みの
# 取得はaruaru-llm自身のセットアップ手順に従うこと(aruaru-llm/README.md
# 参照)。取得後の自動起動は行わない(open-easy-webはこのインストーラー
# 経由でaruaru-llmを取得するだけで、テナント登録〈`src/shell.rs`の
# アプリケーションサーバー選択〉から使う場合はaruaru-llm自身を別途起動
# する必要がある)。aruaru-db・PostgreSQLは含まない——それぞれ別
# リポジトリのセットアップ手順に従うこと(README-INSTALLED.txt参照)。
#
# open-cuda について / About open-cuda:
#   aruaru-llm を使うなら open-cuda も必要だが、別途ダウンロードは
#   不要。open-cuda(opencuda-blas/opencuda-bert/open-cuda-llm 等の
#   GEMM・Attention・GPT-2 デコーダ実装)は aruaru-llm.exe へ静的
#   リンクされており、この zip に既に含まれている。Vulkan/DirectX の
#   GPU バックエンドだけは aruaru-llm 側の GPU ビルド
#   (aruaru-llm-installer.exe の installgpu タスク、既定オフ)を使う
#   場合にのみ有効になる。
#   If you use aruaru-llm you also need open-cuda, but there is nothing
#   extra to download: open-cuda (the opencuda-blas/opencuda-bert/
#   open-cuda-llm GEMM, attention and GPT-2 decoder code) is statically
#   linked into aruaru-llm.exe and is already inside this zip. Only the
#   Vulkan/DirectX GPU backends require aruaru-llm's separate GPU build
#   (the installgpu task in aruaru-llm-installer.exe, off by default).
param(
    [Parameter(Mandatory = $true)]
    [string]$DestDir
)

$ErrorActionPreference = "Stop"

try {
    New-Item -ItemType Directory -Force -Path $DestDir | Out-Null

    $apiUrl = "https://api.github.com/repos/aon-co-jp/aruaru-llm/releases/latest"
    $release = Invoke-RestMethod -Uri $apiUrl -Headers @{ "User-Agent" = "open-english-installer" }
    $asset = $release.assets | Where-Object { $_.name -like "*windows*x86_64*.zip" } | Select-Object -First 1

    if (-not $asset) {
        Write-Output "aruaru-llm: no Windows release asset found. Please download it manually from https://github.com/aon-co-jp/aruaru-llm/releases"
        exit 0
    }

    $zipPath = Join-Path $DestDir "aruaru-llm.zip"
    Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $zipPath -UseBasicParsing
    Expand-Archive -Path $zipPath -DestinationPath $DestDir -Force
    Remove-Item $zipPath -Force

    Write-Output "aruaru-llm downloaded to $DestDir. Model weights are NOT included - see aruaru-llm's own README for how to fetch them and start the server."
} catch {
    # ダウンロード失敗はインストーラー全体を止めない(可用性優先、
    # 既存のaruaru-llm自体の「サービスを止めない」設計方針と同じ)。
    Write-Output "aruaru-llm download failed: $_. You can install it manually later from https://github.com/aon-co-jp/aruaru-llm/releases"
    exit 0
}
