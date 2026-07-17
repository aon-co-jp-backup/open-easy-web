#!/usr/bin/env bash
# HTTPS証明書の自動更新(easyweb-tls-renew)と自動監視(easyweb-tls-monitor)の
# systemdタイマーを、実際のサーバー上にインストールする。root権限が必要。
#
# 使い方: sudo deploy/systemd/install-systemd-units.sh
set -euo pipefail

if [ "$(id -u)" -ne 0 ]; then
  echo "root権限で実行してください(sudo deploy/systemd/install-systemd-units.sh)。" >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
REPO_ROOT="$(cd "${REPO_ROOT}/.." && pwd)"
UNIT_DIR="/etc/systemd/system"

install -m 0644 "${SCRIPT_DIR}/easyweb-tls-renew.service" "${UNIT_DIR}/easyweb-tls-renew.service"
install -m 0644 "${SCRIPT_DIR}/easyweb-tls-renew.timer"   "${UNIT_DIR}/easyweb-tls-renew.timer"

sed -e "s#{{REPO_ROOT}}#${REPO_ROOT}#g" \
  "${SCRIPT_DIR}/easyweb-tls-monitor.service.template" > "${UNIT_DIR}/easyweb-tls-monitor.service"
install -m 0644 "${SCRIPT_DIR}/easyweb-tls-monitor.timer" "${UNIT_DIR}/easyweb-tls-monitor.timer"

chmod +x "${REPO_ROOT}/scripts/check-tls.sh" "${REPO_ROOT}/scripts/check-all-tls.sh" \
  "${REPO_ROOT}/scripts/setup-tls.sh" 2>/dev/null || true

systemctl daemon-reload
systemctl enable --now easyweb-tls-renew.timer
systemctl enable --now easyweb-tls-monitor.timer

echo "有効化しました:"
echo "  - easyweb-tls-renew.timer   : certbot renew を1日2回自動実行(自動更新)"
echo "  - easyweb-tls-monitor.timer : 登録済み全ドメインの有効期限を1日1回チェック(自動監視)"
echo
echo "状態確認: systemctl list-timers | grep easyweb-tls"
echo "手動実行: systemctl start easyweb-tls-monitor.service && journalctl -u easyweb-tls-monitor -n 50"
