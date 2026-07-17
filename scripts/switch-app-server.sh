#!/usr/bin/env bash
# ドメイン単位で「アプリケーションサーバー」(open-runo / poem-cosmo-tauri /
# なし)を後から選択・変更・解除する。Apache+Tomcatの関係と同じく、
# Webサーバー(nginx/apache/open-web-server)は本スクリプト無しでも単体で
# 動作する——このスクリプトは既にデプロイ済みの vhost の転送先
# (proxy_pass/ProxyPass)を書き換えるだけで、Webサーバー自体の有無・
# 稼働状態には影響しない。
#
# 使い方:
#   scripts/switch-app-server.sh <DOMAIN> none
#   scripts/switch-app-server.sh <DOMAIN> open-runo         <HOST:PORT>
#   scripts/switch-app-server.sh <DOMAIN> poem-cosmo-tauri   <HOST:PORT>
#
# 前提: scripts/gen-vhost.sh --stack=proxy で該当ドメインのvhostが
# 既に生成・配置済みであること(scripts/switch-engine.sh で有効化した
# エンジン=nginx/apacheのどちらかを自動検出して書き換える)。
#
# 例:
#   scripts/switch-app-server.sh app.example.com open-runo 127.0.0.1:8080
#   scripts/switch-app-server.sh app.example.com poem-cosmo-tauri 127.0.0.1:8081
#   scripts/switch-app-server.sh app.example.com none                # 解除

set -euo pipefail

if [ $# -lt 2 ]; then
  echo "使い方: $0 <DOMAIN> <none|open-runo|poem-cosmo-tauri> [HOST:PORT]" >&2
  exit 1
fi

DOMAIN="$1"
APP_SERVER="$2"
UPSTREAM="${3:-}"

case "$APP_SERVER" in
  none) ;;
  open-runo|poem-cosmo-tauri)
    if [ -z "$UPSTREAM" ]; then
      echo "'${APP_SERVER}' を選択する場合は接続先 HOST:PORT の指定が必要です。" >&2
      exit 1
    fi
    ;;
  *)
    echo "不明なアプリケーションサーバー: '${APP_SERVER}'" \
         "(none/open-runo/poem-cosmo-tauri のいずれかを指定)" >&2
    exit 1
    ;;
esac

# デプロイ済みvhostの検出(switch-engine.shが配置した先を両方確認)。
CANDIDATES=(
  "/etc/nginx/conf.d/${DOMAIN}.conf"
  "/etc/httpd/conf.d/${DOMAIN}.conf"
  "/etc/apache2/sites-enabled/${DOMAIN}.conf"
)
TARGET_CONF=""
ENGINE=""
for c in "${CANDIDATES[@]}"; do
  if [ -f "$c" ]; then
    TARGET_CONF="$c"
    case "$c" in
      /etc/nginx/*) ENGINE="nginx" ;;
      *) ENGINE="apache" ;;
    esac
    break
  fi
done

if [ -z "$TARGET_CONF" ]; then
  echo "[ERROR] ${DOMAIN} のvhostが見つかりません。先に以下を実行してください:" >&2
  echo "  scripts/gen-vhost.sh --stack=proxy --engine=nginx ${DOMAIN} <BIND_IP> <UPSTREAM>" >&2
  echo "  scripts/switch-engine.sh ${DOMAIN} nginx" >&2
  exit 1
fi

if [ "$APP_SERVER" = "none" ]; then
  echo "'none' への切り替えは、このドメインを静的/別バックエンド配信に戻すことを意味します。"
  echo "既存の転送先設定はそのまま保持されます(vhostの再生成・再配置は"
  echo "scripts/gen-vhost.sh --stack=<static等> で行ってください)。"
  echo "${DOMAIN}: app_server=none として記録のみ行います。"
else
  case "$ENGINE" in
    nginx)
      sed -i -E "s#(proxy_pass[[:space:]]+http://)[^;]+(;)#\1${UPSTREAM}\2#" "$TARGET_CONF"
      nginx -t
      if command -v systemctl >/dev/null 2>&1; then systemctl reload nginx; else nginx -s reload; fi
      ;;
    apache)
      sed -i -E "s#(ProxyPass[[:space:]]+\"/\"[[:space:]]+\"http://)[^\"]+(\")#\1${UPSTREAM}\2#" "$TARGET_CONF"
      sed -i -E "s#(ProxyPassReverse[[:space:]]+\"/\"[[:space:]]+\"http://)[^\"]+(\")#\1${UPSTREAM}\2#" "$TARGET_CONF"
      if command -v apache2ctl >/dev/null 2>&1; then
        apache2ctl configtest && { command -v systemctl >/dev/null 2>&1 && systemctl reload apache2 || apache2ctl graceful; }
      else
        httpd -t && { command -v systemctl >/dev/null 2>&1 && systemctl reload httpd || httpd -k graceful; }
      fi
      ;;
  esac
  echo "${TARGET_CONF} の転送先を ${UPSTREAM}(${APP_SERVER})に変更し、${ENGINE} をリロードしました。"
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
APP_SERVERS_FILE="${REPO_ROOT}/deploy/generated/app-servers.txt"
mkdir -p "$(dirname "${APP_SERVERS_FILE}")"
touch "${APP_SERVERS_FILE}"
grep -vxE "${DOMAIN}:.*" "${APP_SERVERS_FILE}" > "${APP_SERVERS_FILE}.tmp" 2>/dev/null || true
mv "${APP_SERVERS_FILE}.tmp" "${APP_SERVERS_FILE}" 2>/dev/null || true
echo "${DOMAIN}:${APP_SERVER}:${UPSTREAM}" >> "${APP_SERVERS_FILE}"

echo
echo "${DOMAIN} のアプリケーションサーバーを '${APP_SERVER}' に設定しました。"
