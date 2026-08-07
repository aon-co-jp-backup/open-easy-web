#!/bin/sh
# open-easy-web-server macOS向けアンインストールスクリプト。
#
# **正直な開示**: install-macos.sh と同様、この開発環境(Windows機)では
# 実際のmacOS実機での動作確認は行っていない(構文検証のみ)。

set -eu

INSTALL_DIR="${HOME}/.local/bin"
PLIST_LABEL="jp.co.aon.open-easy-web"
PLIST_FILE="${HOME}/Library/LaunchAgents/${PLIST_LABEL}.plist"

if [ "$(uname -s)" != "Darwin" ]; then
    echo "このスクリプトはmacOS専用です。" >&2
    exit 1
fi

if [ -f "$PLIST_FILE" ]; then
    echo "==> サービスを停止・登録解除(${PLIST_FILE})"
    launchctl bootout "gui/$(id -u)/${PLIST_LABEL}" 2>/dev/null || \
        launchctl unload "$PLIST_FILE" 2>/dev/null || true
    rm -f "$PLIST_FILE"
else
    echo "==> plistが見つからないため登録解除をスキップ(${PLIST_FILE})"
fi

if [ -f "${INSTALL_DIR}/open-easy-web-server" ]; then
    echo "==> バイナリを削除(${INSTALL_DIR}/open-easy-web-server)"
    rm -f "${INSTALL_DIR}/open-easy-web-server"
fi

echo "==> 完了。ユーザーデータ(~/Library/Application Support/open-easy-web)は"
echo "    削除していません。必要であれば手動で削除してください。"
