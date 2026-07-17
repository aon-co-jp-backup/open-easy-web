#!/usr/bin/env bash
# scripts/gen-vhost.sh が記録した全ドメイン(deploy/generated/domains.txt)の
# TLS証明書有効期限をまとめて監視する。cron / systemdタイマーからの定期実行を想定。
# 使い方: scripts/check-all-tls.sh [WARN_DAYS]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
DOMAINS_FILE="${REPO_ROOT}/deploy/generated/domains.txt"
WARN_DAYS="${1:-14}"

if [ ! -f "${DOMAINS_FILE}" ]; then
  echo "監視対象ドメインがありません(${DOMAINS_FILE} が未生成)。先に scripts/gen-vhost.sh を実行してください。"
  exit 0
fi

status=0
while IFS= read -r domain; do
  [ -z "${domain}" ] && continue
  "${SCRIPT_DIR}/check-tls.sh" "${domain}" "${WARN_DAYS}" || status=1
done < "${DOMAINS_FILE}"
exit "${status}"
