#!/usr/bin/env bash
# ドメイン間の設定混線(vhostの取り違え・意図しないクロスドメイン
# リダイレクト・別ドメインの内容が誤って返る等)を検知するスモークテスト。
#
# 背景: 2026-07-21、"runo.tokyo にアクセスすると easy-web.tokyo に強制的に
# 画面遷移する"というバグ報告があった。原因はサーバー側の実配線ではなく
# 特定できなかった(nginx設定・実HTTPレスポンスとも問題なしと確認)ものの、
# 過去にも「nginx locationブロックが誤って別のserver{}ブロックに追加され、
# 構文チェック(nginx -t)は通るが実際のトラフィックでは別サイトの内容が
# 返る」という実バグが起きている(RGitデプロイ時)。この種の「構文は
# 正しいがドメインを跨いで混線する」バグは、`nginx -t`だけでは検知できず、
# 実際にHTTPリクエストを送って初めて分かるため、専用のチェックを用意する
# (`open-raid-z`のCLAUDE.md運用ルール「ドメイン跨ぎ混線チェック」参照)。
#
# 使い方: scripts/check-vhost-isolation.sh [CHECKS_FILE]
#   CHECKS_FILE 既定値: deploy/generated/domain-checks.txt
#   各行フォーマット: URL<TAB>期待される本文中の一意な文字列
#     例: https://runo.tokyo/<TAB>東京都西部
#   空行・#始まりの行は無視する。
#
# 各URLについて以下を確認する:
#   1. リダイレクトを追いかけた最終到達先ホストが、要求したホストと
#      同一(またはbare⇔www・http→httpsの範囲内)であること
#      ——別ドメインへ飛ばされていないか(今回の報告バグのクラス)。
#   2. レスポンス本文に、そのドメイン固有の期待文字列が含まれること
#      ——別ドメインの内容が誤って返っていないか。
#   3. レスポンス本文に、*他の*行で登録されている期待文字列が
#      誤って混入していないか(取り違えの直接検知)。
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
CHECKS_FILE="${1:-${REPO_ROOT}/deploy/generated/domain-checks.txt}"

if [ ! -f "${CHECKS_FILE}" ]; then
  echo "チェック対象が無い(${CHECKS_FILE} が未生成)。scripts/gen-vhost.sh 等でドメインを登録後、同ファイルを用意してください。"
  exit 0
fi

# ホストの正規化: www.を剥がし、末尾の/を除去して比較する
# (bare/wwwの揺れ・トレイリングスラッシュの違いは同一ドメインとみなす)。
normalize_host() {
  echo "$1" | sed -E 's#^https?://##; s#^www\.##; s#/.*$##'
}

urls=()
markers=()
while IFS=$'\t' read -r url marker; do
  [ -z "${url:-}" ] && continue
  case "${url}" in \#*) continue ;; esac
  urls+=("${url}")
  markers+=("${marker}")
done < "${CHECKS_FILE}"

status=0
for i in "${!urls[@]}"; do
  url="${urls[$i]}"
  expected="${markers[$i]}"
  req_host="$(normalize_host "${url}")"

  effective_url="$(curl -sL -o /tmp/vhost-isolation-body.$$ -w '%{url_effective}' --max-time 15 "${url}" || true)"
  body="$(cat /tmp/vhost-isolation-body.$$ 2>/dev/null || echo '')"
  rm -f /tmp/vhost-isolation-body.$$

  final_host="$(normalize_host "${effective_url}")"

  ok=1
  if [ "${final_host}" != "${req_host}" ]; then
    echo "❌ [クロスドメイン混線] ${url} -> 最終到達先ホストが不一致 (${req_host} != ${final_host}, 実際のリダイレクト先: ${effective_url})"
    ok=0
  fi
  if ! printf '%s' "${body}" | grep -qF -- "${expected}"; then
    echo "❌ [内容不一致] ${url} の本文に期待文字列が見つかりません: \"${expected}\""
    ok=0
  fi
  for j in "${!markers[@]}"; do
    [ "$j" = "$i" ] && continue
    other="${markers[$j]}"
    [ -z "${other}" ] && continue
    if printf '%s' "${body}" | grep -qF -- "${other}"; then
      echo "⚠️  [取り違え疑い] ${url} の本文に別ドメイン(${urls[$j]})の期待文字列 \"${other}\" が混入しています"
      ok=0
    fi
  done

  if [ "${ok}" = "1" ]; then
    echo "✅ ${url} (${expected})"
  else
    status=1
  fi
done

exit "${status}"
