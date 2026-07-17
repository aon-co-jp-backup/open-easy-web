#!/usr/bin/env bash
# 登録済みドメインの配信エンジン(Nginx/Apache)を後から切り替える。
# scripts/gen-vhost.sh --engine=nginx|apache|both で生成済みの
# deploy/generated/<DOMAIN>.{nginx,apache}.conf を前提とする。
#
# 使い方: scripts/switch-engine.sh <DOMAIN> <nginx|apache> [--webroot=/var/www/<DOMAIN>]
#
# 動作:
#   1. 対象エンジンの生成済みvhostが無ければ、gen-vhost.sh --engine=<ENGINE>
#      を再実行するよう案内して終了する(UPSTREAM/WEBROOTはこのスクリプトからは
#      分からないため、自動生成はしない)。
#   2. 対象エンジンのvhostを配置し、もう一方のエンジンのvhostが配置済みなら
#      無効化する(ファイルを削除するのではなく .disabled にリネームして退避)。
#   3. 対象エンジンのみリロードする。
#   4. deploy/generated/engines.txt に現在の有効エンジンを記録する。
#
# 注: open-web-server はフロントのエンジン(このスクリプトの対象)ではない。
# --stack=proxy のUPSTREAMとして指定するバックエンドであり、エンジンには
# 含まれない(gen-vhost.sh冒頭のコメント参照)。

set -euo pipefail

if [ $# -lt 2 ]; then
  echo "使い方: $0 <DOMAIN> <nginx|apache> [--webroot=/var/www/<DOMAIN>]" >&2
  exit 1
fi

DOMAIN="$1"
ENGINE="$2"
shift 2
WEBROOT=""
for arg in "$@"; do
  case "$arg" in
    --webroot=*) WEBROOT="${arg#--webroot=}" ;;
  esac
done
WEBROOT="${WEBROOT:-/var/www/${DOMAIN}}"

case "$ENGINE" in
  nginx|apache) ;;
  *)
    echo "不明なエンジン: '${ENGINE}'(nginx/apache のいずれかを指定。open-web-server は" \
         "フロントのエンジンではなくUPSTREAM側なのでここでは選べません)" >&2
    exit 1
    ;;
esac

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
GEN_DIR="${REPO_ROOT}/deploy/generated"
SRC_CONF="${GEN_DIR}/${DOMAIN}.${ENGINE}.conf"

if [ ! -f "${SRC_CONF}" ]; then
  echo "[ERROR] ${SRC_CONF} が見つかりません。先に生成してください:" >&2
  echo "  scripts/gen-vhost.sh --engine=${ENGINE} ${DOMAIN} <BIND_IP> [UPSTREAM] ${WEBROOT}" >&2
  exit 1
fi

# 配置先ディレクトリの検出(RHEL系/Debian系の両方に対応)。
detect_nginx_dir() {
  if [ -d /etc/nginx/conf.d ]; then echo /etc/nginx/conf.d; return; fi
  echo "[ERROR] /etc/nginx/conf.d が見つかりません(nginx未導入?)" >&2
  exit 1
}
detect_apache_dir() {
  if [ -d /etc/httpd/conf.d ]; then echo /etc/httpd/conf.d; return; fi         # RHEL/AlmaLinux
  if [ -d /etc/apache2/sites-enabled ]; then echo /etc/apache2/sites-enabled; return; fi # Debian/Ubuntu
  echo "[ERROR] Apacheの設定ディレクトリが見つかりません(未導入?)" >&2
  exit 1
}

NGINX_DIR="$(detect_nginx_dir 2>/dev/null || true)"
APACHE_DIR="$(detect_apache_dir 2>/dev/null || true)"

disable_if_present() {
  local dir="$1" file="$2"
  if [ -n "$dir" ] && [ -f "${dir}/${file}" ]; then
    mv "${dir}/${file}" "${dir}/${file}.disabled"
    echo "無効化: ${dir}/${file} -> ${file}.disabled"
  fi
}

reload_nginx() {
  nginx -t
  if command -v systemctl >/dev/null 2>&1; then systemctl reload nginx; else nginx -s reload; fi
  echo "nginx をリロードしました。"
}
reload_apache() {
  if command -v apache2ctl >/dev/null 2>&1; then
    apache2ctl configtest && { command -v systemctl >/dev/null 2>&1 && systemctl reload apache2 || apache2ctl graceful; }
  else
    httpd -t && { command -v systemctl >/dev/null 2>&1 && systemctl reload httpd || httpd -k graceful; }
  fi
  echo "Apache をリロードしました。"
}

if [ "$ENGINE" = "nginx" ]; then
  [ -z "${NGINX_DIR}" ] && { echo "[ERROR] nginxの配置先が見つかりません。" >&2; exit 1; }
  disable_if_present "${APACHE_DIR}" "${DOMAIN}.conf"
  cp "${SRC_CONF}" "${NGINX_DIR}/${DOMAIN}.conf"
  echo "配置: ${NGINX_DIR}/${DOMAIN}.conf"
  reload_nginx
else
  [ -z "${APACHE_DIR}" ] && { echo "[ERROR] Apacheの配置先が見つかりません。" >&2; exit 1; }
  disable_if_present "${NGINX_DIR}" "${DOMAIN}.conf"
  cp "${SRC_CONF}" "${APACHE_DIR}/${DOMAIN}.conf"
  echo "配置: ${APACHE_DIR}/${DOMAIN}.conf"
  echo "(Apache版を使う場合は事前に a2enmod ssl proxy proxy_http rewrite 等を有効化しておくこと)"
  reload_apache
fi

# ドメインごとの現在の有効エンジンを記録(gen-vhost.shと同じファイルを共有)。
ENGINES_FILE="${GEN_DIR}/engines.txt"
touch "${ENGINES_FILE}"
grep -vxE "${DOMAIN}:.*" "${ENGINES_FILE}" > "${ENGINES_FILE}.tmp" 2>/dev/null || true
mv "${ENGINES_FILE}.tmp" "${ENGINES_FILE}" 2>/dev/null || true
echo "${DOMAIN}:${ENGINE}" >> "${ENGINES_FILE}"

echo
echo "${DOMAIN} のエンジンを ${ENGINE} に切り替えました。"
