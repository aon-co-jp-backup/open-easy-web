#!/usr/bin/env bash
# 廃止済みサービス(既定: aruaru-web)の残骸を検出するAudit専用スクリプト。
#
# **このスクリプトは既定では何も削除しません(dry-run)。** systemdサービス・
# crontab・証明書更新フック(certbot renewal hooks)を走査し、指定した
# 文字列(サービス名・ドメイン等)を参照している項目を「削除候補」として
# 一覧表示するだけです。実際の削除は、一覧を人間が確認した上で、
# 各項目に対応する `--execute` 付きの個別コマンドを別途手動で実行して
# 初めて行われます(このスクリプト自身が `rm`/`systemctl disable`/
# `crontab -r` 等の破壊的操作を実行することは一切ありません)。
#
# **なぜ自動削除にしないか**: 同じcronエントリ・証明書更新フックを
# 別の現役サービスが共用しているケースを誤検知すると、無関係なサービスを
# 巻き添えで壊すリスクがあるため。「検知・レポートは自動化するが、実行は
# 人間の最終承認を必須とする」という設計方針(ユーザー承認済み、
# 2026-07-14)。
#
# 使い方:
#   scripts/audit-orphaned-services.sh <検索文字列> [<検索文字列2> ...]
#
# 例:
#   scripts/audit-orphaned-services.sh aruaru-web
#   scripts/audit-orphaned-services.sh aruaru-web aruaru_web
#
# 検索対象:
#   1. systemd unit ファイル(/etc/systemd/system/*.service, *.timer)の
#      中身(ExecStart等)に検索文字列を含むもの、および unit ファイル名
#      自体に検索文字列を含むもの。
#   2. crontab(root および全ユーザーの /var/spool/cron/crontabs/*、
#      利用可能なら /etc/cron.d/* も)の各行。
#   3. certbot の renewal 設定(/etc/letsencrypt/renewal/*.conf)と、
#      その中の deploy-hook / pre-hook / post-hook が指すスクリプト。
#
# 出力は「見つかった箇所」のみで、削除コマンドの"雛形"を添えて表示する
# ——コピペしてすぐ実行できてしまうと事故が起きやすいため、意図的に
# プレースホルダ(<REVIEW>)を混ぜてあり、そのままでは実行できない。

set -euo pipefail

if [ $# -lt 1 ]; then
  echo "使い方: $0 <検索文字列> [<検索文字列2> ...]" >&2
  echo "例:     $0 aruaru-web aruaru_web" >&2
  exit 1
fi

PATTERNS=("$@")
FOUND_ANY=0

echo "=========================================================="
echo " open-easy-web: 廃止済みサービス残骸監査(dry-run、削除は行いません)"
echo " 検索文字列: ${PATTERNS[*]}"
echo "=========================================================="
echo

grep_any_pattern() {
  # $1 = 対象ファイル/文字列。 検索文字列のいずれかにマッチすれば0を返す。
  local target="$1"
  for p in "${PATTERNS[@]}"; do
    if printf '%s' "$target" | grep -qi -- "$p"; then
      return 0
    fi
  done
  return 1
}

# ── 1. systemd unit ファイル ──────────────────────────────────────
echo "--- [1/3] systemd unit ファイル ---"
SYSTEMD_HIT=0
if [ -d /etc/systemd/system ]; then
  while IFS= read -r -d '' unit; do
    base="$(basename "$unit")"
    if grep_any_pattern "$base" || grep -qi -f <(printf '%s\n' "${PATTERNS[@]}") "$unit" 2>/dev/null; then
      SYSTEMD_HIT=1
      FOUND_ANY=1
      echo "  [検出] ${unit}"
      echo "         状態: $(systemctl is-active "$base" 2>/dev/null || echo '不明/未登録')" \
           "/ $(systemctl is-enabled "$base" 2>/dev/null || echo '不明')"
      echo "         削除の目安コマンド(内容を確認してから手動実行してください):"
      echo "           systemctl disable --now '${base}'   # <REVIEW> 先に他サービスが依存していないか確認"
      echo "           rm '${unit}'                          # <REVIEW>"
      echo "           systemctl daemon-reload"
      echo
    fi
  done < <(find /etc/systemd/system -maxdepth 1 \( -name '*.service' -o -name '*.timer' \) -print0 2>/dev/null)
fi
if [ "$SYSTEMD_HIT" -eq 0 ]; then
  echo "  該当する systemd unit は見つかりませんでした。"
fi
echo

# ── 2. crontab ──────────────────────────────────────────────────
echo "--- [2/3] crontab ---"
CRON_HIT=0

check_crontab_lines() {
  # $1 = 表示用ラベル, $2 = crontab の中身(標準入力)
  local label="$1"
  while IFS= read -r line; do
    [ -z "$line" ] && continue
    case "$line" in \#*) continue ;; esac
    if grep_any_pattern "$line"; then
      CRON_HIT=1
      FOUND_ANY=1
      echo "  [検出] (${label}) ${line}"
    fi
  done
}

if command -v crontab >/dev/null 2>&1; then
  crontab -l 2>/dev/null | check_crontab_lines "root crontab" || true
fi
if [ -d /var/spool/cron/crontabs ]; then
  for f in /var/spool/cron/crontabs/*; do
    [ -f "$f" ] || continue
    user="$(basename "$f")"
    check_crontab_lines "user:${user}" < "$f"
  done
fi
if [ -d /etc/cron.d ]; then
  for f in /etc/cron.d/*; do
    [ -f "$f" ] || continue
    check_crontab_lines "/etc/cron.d/$(basename "$f")" < "$f"
  done
fi

if [ "$CRON_HIT" -eq 1 ]; then
  echo
  echo "  削除の目安: 該当行を見つけたcrontabを 'crontab -e'(該当ユーザーで)"
  echo "  または /etc/cron.d/該当ファイルを直接編集し、行を削除してください。"
  echo "  <REVIEW> このスクリプトは crontab を書き換えません。"
else
  echo "  該当する crontab エントリは見つかりませんでした。"
fi
echo

# ── 3. certbot renewal 設定 + hookスクリプト ─────────────────────
echo "--- [3/3] certbot renewal 設定 / 証明書更新フック ---"
CERT_HIT=0
if [ -d /etc/letsencrypt/renewal ]; then
  for conf in /etc/letsencrypt/renewal/*.conf; do
    [ -f "$conf" ] || continue
    if grep_any_pattern "$(basename "$conf")" || grep -qi -f <(printf '%s\n' "${PATTERNS[@]}") "$conf" 2>/dev/null; then
      CERT_HIT=1
      FOUND_ANY=1
      echo "  [検出] ${conf}"
      # deploy-hook / pre-hook / post-hook が指すスクリプトも表示する
      grep -E '^(deploy|pre|post)_hook' "$conf" 2>/dev/null | sed 's/^/         hook: /'
      echo "         削除の目安コマンド:"
      echo "           certbot delete --cert-name '$(basename "$conf" .conf)'   # <REVIEW> 証明書と設定を両方消す公式コマンド"
      echo "           # 上記は該当ドメインの証明書ファイル一式もまとめて削除するため、"
      echo "           # 本当に他で使われていないことを確認してから実行してください。"
      echo
    fi
  done
fi
if [ "$CERT_HIT" -eq 0 ]; then
  echo "  該当する certbot renewal 設定は見つかりませんでした。"
fi
echo

echo "=========================================================="
if [ "$FOUND_ANY" -eq 1 ]; then
  echo " 検出結果あり。上記の <REVIEW> 箇所を確認の上、手動で削除してください。"
  echo " このスクリプト自身は一切の変更を行っていません(dry-run)。"
  exit 0
else
  echo " 該当する残骸は見つかりませんでした。"
  exit 0
fi
