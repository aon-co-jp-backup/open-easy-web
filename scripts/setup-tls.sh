#!/usr/bin/env bash
# Let's Encrypt (certbot) でドメインのTLS証明書を取得し、自動更新を有効化する。
# scripts/gen-vhost.sh が生成する vhost 設定(443番、
# /etc/letsencrypt/live/<DOMAIN>/ を参照)と対になっている。
#
# 使い方: scripts/setup-tls.sh <DOMAIN> [EMAIL] [WEBROOT]
# 例:    scripts/setup-tls.sh easyweb.example.com admin@example.com /var/www/easyweb.example.com
#
# 前提:
#   - certbot がインストール済みであること(例: apt install certbot)。
#   - DOMAIN のAレコードがこのホストのIPを指しており、80番ポートが
#     外部からHTTP到達可能であること(ACME HTTP-01チャレンジのため)。
#   - vhost はすでに scripts/gen-vhost.sh + 手動配置で有効化済みであること。

set -euo pipefail

if [ $# -lt 1 ]; then
  echo "使い方: $0 <DOMAIN> [EMAIL] [WEBROOT]" >&2
  exit 1
fi

DOMAIN="$1"
EMAIL="${2:-admin@${DOMAIN}}"
WEBROOT="${3:-/var/www/${DOMAIN}}"

if ! command -v certbot >/dev/null 2>&1; then
  echo "[ERROR] certbot が見つかりません。先にインストールしてください" \
       "(例: apt install certbot、または dnf install certbot)。" >&2
  exit 1
fi

mkdir -p "${WEBROOT}/.well-known/acme-challenge"

echo "証明書を取得/更新します: ${DOMAIN}"
certbot certonly --webroot -w "${WEBROOT}" -d "${DOMAIN}" \
  --non-interactive --agree-tos -m "${EMAIL}" --keep-until-expiring

echo
echo "証明書: /etc/letsencrypt/live/${DOMAIN}/fullchain.pem"
echo "秘密鍵: /etc/letsencrypt/live/${DOMAIN}/privkey.pem"
echo "(生成済み vhost ファイルのSSL証明書パスと一致しているか確認してください)"
echo

# 自動更新: certbotパッケージ同梱のtimerがあればそれを使い、なければ
# このリポジトリ同梱の deploy/systemd/easyweb-tls-renew.timer を案内する。
if command -v systemctl >/dev/null 2>&1 && systemctl list-unit-files 2>/dev/null | grep -q '^certbot.timer'; then
  systemctl enable --now certbot.timer
  echo "certbot.timer を有効化しました(1日2回の自動更新)。"
else
  echo "certbot.timer が見つかりません。このリポジトリ同梱のunitで自動更新を有効化してください:"
  echo "  deploy/systemd/install-systemd-units.sh"
fi
