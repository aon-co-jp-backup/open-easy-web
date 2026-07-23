#!/bin/sh
# open-easy-web(open-easy-web-server)インストールスクリプト(AlmaLinux/
# Ubuntu/Debian/Fedora/RHEL等、systemdを使う主要Linuxディストリ共通)。
#
# 正直な開示: このバイナリは固定アカウント制の認証を持ち、起動時に
# 環境変数 `OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL` が未設定だと panic して
# 即座に終了する(誰もログインできない状態でサイレントに動き続けるより
# 起動失敗のほうが安全という設計判断、詳細はCLAUDE.md参照)。
# WASMフロントエンド(`pkg/`+`index.html`)は別途 `cargo build --target
# wasm32-unknown-unknown` + `wasm-bindgen` でのビルドが必要で、この
# tar.gzには含まれない(バックエンドAPIサーバーのみを配布対象とする)。
#
# 使い方:
#   curl -fsSL https://github.com/aon-co-jp/open-easy-web/releases/latest/download/open-easy-web-server-linux-x86_64.tar.gz | tar xz
#   sudo ./install.sh

set -eu

BIN_SRC="$(dirname "$0")/open-easy-web-server"
INSTALL_DIR="/usr/local/bin"
DATA_DIR="/etc/open-easy-web"
SERVICE_FILE="/etc/systemd/system/open-easy-web.service"

if [ "$(id -u)" -ne 0 ]; then
    echo "root権限で実行してください(例: sudo ./install.sh)" >&2
    exit 1
fi

if [ ! -f "$BIN_SRC" ]; then
    echo "open-easy-web-server バイナリが見つかりません($BIN_SRC)。同梱のtar.gzを展開したディレクトリで実行してください。" >&2
    exit 1
fi

echo "==> バイナリを ${INSTALL_DIR}/open-easy-web-server へ配置"
install -m 755 "$BIN_SRC" "${INSTALL_DIR}/open-easy-web-server"
mkdir -p "$DATA_DIR"

if [ ! -f "$SERVICE_FILE" ]; then
    echo "==> systemdサービスを作成(${SERVICE_FILE})"
    cat > "$SERVICE_FILE" << EOF
[Unit]
Description=open-easy-web - 第二のKUSANAGI(ドメイン/HTTPS簡単登録+アップロード運用ツール)
After=network.target

[Service]
Type=simple
WorkingDirectory=${DATA_DIR}
Environment=OPEN_EASYWEB_SERVER_BIND=0.0.0.0:8090
# 必須: 固定アカウント制のログイン用メールアドレス(未設定だと起動時にpanicする)。
# Environment=OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL=you@example.com
# 任意: 電話番号かセカンドメールのどちらか一方以上の登録が必要。
# Environment=OPEN_EASYWEB_FIXED_ACCOUNT_PHONE=+81-90-xxxx-xxxx
# Environment=OPEN_EASYWEB_FIXED_ACCOUNT_BACKUP_EMAIL=backup@example.com
# 任意: WASMフロントエンド(pkg/+index.html)を同梱配信する場合の静的ファイル配置先。
# Environment=OPEN_EASYWEB_STATIC_DIR=${DATA_DIR}/static
ExecStart=${INSTALL_DIR}/open-easy-web-server
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF
    systemctl daemon-reload
else
    echo "==> 既存のsystemdサービスが見つかったため上書きしません(${SERVICE_FILE})"
fi

echo "==> 完了。次のコマンドで必須環境変数を設定してから起動してください:"
echo "    sudo systemctl edit open-easy-web  # OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL 等を追記"
echo "    sudo systemctl enable --now open-easy-web"
