#!/bin/sh
# open-easy-web(open-easy-web-server)macOS向けインストールスクリプト。
#
# **正直な開示(2026-08-06追加)**: この開発環境はWindows機であり、実際の
# macOS環境でのビルド・`launchctl load`実行・動作確認は一切行えていない。
# 本スクリプトの検証は以下に限定される:
#   - シェル構文検証(`bash -n install-macos.sh`相当)のみ。
#   - `launchd`のplist書式・`launchctl`コマンドの用法は2026年時点の
#     日英Web検索(macOS Ventura〜Sequoia世代の情報)で確認したが、
#     実機での`launchctl bootstrap`/`launchctl load`実行結果は未確認。
# 実際にmacOS実機で試した際に問題が見つかった場合は、CLAUDE.mdの
# HANDOFFへ追記した上で本スクリプトを修正すること。
#
# macOSのサービス管理はsystemdではなくlaunchdを使う。本スクリプトは
# ユーザーレベルのLaunchAgent(~/Library/LaunchAgents/)へplistを配置する
# 方式を採用した(システム全体のLaunchDaemonにするにはroot権限+
# /Library/LaunchDaemons/への配置が必要になるため、まずより手軽な
# ユーザーレベルを既定にした)。
#
# 使い方:
#   curl -fsSL https://github.com/aon-co-jp/open-easy-web/releases/latest/download/open-easy-web-server-macos-x86_64.tar.gz | tar xz
#   ./install-macos.sh
#
# (Apple Silicon向けのビルドはCI(.github/workflows/release.yml、
#  build-macosジョブ)側でaarch64-apple-darwinターゲットも生成する想定
#  だが、この開発環境ではクロスビルド自体を検証できていない——
#  下記「クロスコンパイルについて」参照)

set -eu

BIN_SRC="$(dirname "$0")/open-easy-web-server"
INSTALL_DIR="${HOME}/.local/bin"
DATA_DIR="${HOME}/Library/Application Support/open-easy-web"
LAUNCH_AGENTS_DIR="${HOME}/Library/LaunchAgents"
PLIST_LABEL="jp.co.aon.open-easy-web"
PLIST_FILE="${LAUNCH_AGENTS_DIR}/${PLIST_LABEL}.plist"
LOG_DIR="${HOME}/Library/Logs/open-easy-web"

if [ "$(uname -s)" != "Darwin" ]; then
    echo "このスクリプトはmacOS専用です(Linuxは install.sh、Windowsは install.ps1 を使ってください)。" >&2
    exit 1
fi

if [ ! -f "$BIN_SRC" ]; then
    echo "open-easy-web-server バイナリが見つかりません($BIN_SRC)。同梱のtar.gzを展開したディレクトリで実行してください。" >&2
    exit 1
fi

echo "==> バイナリを ${INSTALL_DIR}/open-easy-web-server へ配置"
mkdir -p "$INSTALL_DIR"
install -m 755 "$BIN_SRC" "${INSTALL_DIR}/open-easy-web-server"
mkdir -p "$DATA_DIR"
mkdir -p "$LOG_DIR"
mkdir -p "$LAUNCH_AGENTS_DIR"

if [ ! -f "$PLIST_FILE" ]; then
    echo "==> launchd用plistを作成(${PLIST_FILE})"
    cat > "$PLIST_FILE" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>${PLIST_LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>${INSTALL_DIR}/open-easy-web-server</string>
    </array>
    <key>WorkingDirectory</key>
    <string>${DATA_DIR}</string>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>
    <key>StandardOutPath</key>
    <string>${LOG_DIR}/stdout.log</string>
    <key>StandardErrorPath</key>
    <string>${LOG_DIR}/stderr.log</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>OPEN_EASYWEB_SERVER_BIND</key>
        <string>0.0.0.0:8090</string>
        <!-- 必須: 固定アカウント制のログイン用メールアドレス
             (未設定だと起動時にpanicする、Linux版install.shと同じ制約)。
             下記の行を編集してから `launchctl load` すること。 -->
        <!-- <key>OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL</key>
        <string>you@example.com</string> -->
        <!-- 任意: 電話番号かセカンドメールのどちらか一方以上の登録が必要。 -->
        <!-- <key>OPEN_EASYWEB_FIXED_ACCOUNT_PHONE</key>
        <string>+81-90-xxxx-xxxx</string> -->
        <!-- <key>OPEN_EASYWEB_FIXED_ACCOUNT_BACKUP_EMAIL</key>
        <string>backup@example.com</string> -->
        <!-- 任意: WASMフロントエンド(pkg/+index.html)を同梱配信する場合の静的ファイル配置先。 -->
        <!-- <key>OPEN_EASYWEB_STATIC_DIR</key>
        <string>${DATA_DIR}/static</string> -->
    </dict>
</dict>
</plist>
EOF
else
    echo "==> 既存のlaunchd plistが見つかったため上書きしません(${PLIST_FILE})"
fi

echo ""
echo "==> 完了。次の手順で必須環境変数を設定してから起動してください:"
echo "    1. ${PLIST_FILE} を編集し、OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL 等の"
echo "       コメントアウトされた <key>/<string> 行を有効化・値を設定する。"
echo "    2. サービスを読み込んで起動する(macOS Ventura以降推奨のサブコマンド):"
echo "         launchctl bootstrap gui/\$(id -u) ${PLIST_FILE}"
echo "       (古い方式との互換のため、`launchctl load -w ${PLIST_FILE}` でも動作するはずだが"
echo "        `load`/`unload` はAppleにより将来的な非推奨〈deprecated〉の方向性が示唆されている"
echo "        ため、新規導入では `bootstrap`/`bootout` を推奨する——2026年時点の情報、"
echo "        実機での動作は未検証)。"
echo "    3. 状態確認: launchctl list | grep ${PLIST_LABEL}"
echo "    4. ログ確認: tail -f ${LOG_DIR}/stdout.log ${LOG_DIR}/stderr.log"
echo ""
echo "==> 停止・アンインストールする場合は uninstall-macos.sh を使ってください。"
