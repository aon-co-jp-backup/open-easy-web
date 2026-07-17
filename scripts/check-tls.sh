#!/usr/bin/env bash
# 1ドメインのTLS証明書の残り有効期限を確認する(監視用)。
# 使い方: scripts/check-tls.sh <DOMAIN> [WARN_DAYS]
# 終了コード: 0=正常、1=WARN_DAYS以内に失効、2=証明書を取得できなかった。
set -euo pipefail

if [ $# -lt 1 ]; then
  echo "使い方: $0 <DOMAIN> [WARN_DAYS]" >&2
  exit 2
fi

DOMAIN="$1"
WARN_DAYS="${2:-14}"

EXPIRY_DATE="$(echo | openssl s_client -servername "${DOMAIN}" -connect "${DOMAIN}:443" 2>/dev/null \
  | openssl x509 -noout -enddate 2>/dev/null | cut -d= -f2 || true)"

if [ -z "${EXPIRY_DATE}" ]; then
  echo "[ERROR] ${DOMAIN} の証明書を取得できませんでした(未設定、443番未到達などの可能性)。" >&2
  exit 2
fi

EXPIRY_EPOCH="$(date -d "${EXPIRY_DATE}" +%s)"
NOW_EPOCH="$(date +%s)"
DAYS_LEFT=$(( (EXPIRY_EPOCH - NOW_EPOCH) / 86400 ))

echo "${DOMAIN}: 残り ${DAYS_LEFT} 日で失効(期限: ${EXPIRY_DATE})"

if [ "${DAYS_LEFT}" -le "${WARN_DAYS}" ]; then
  echo "[WARN] ${DOMAIN} の証明書が ${WARN_DAYS} 日以内に失効します。certbot renew の動作を確認してください。" >&2
  exit 1
fi
exit 0
