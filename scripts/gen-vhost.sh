#!/usr/bin/env bash
# KUSANAGIの「サイト追加」相当: ドメイン/サブドメイン + IP + バックエンドスタックの
# 組み合わせから Nginx/Apache の vhost 設定ファイル(HTTP+HTTPS+基本的な
# ドメイン/リバースプロキシ配線)を自動生成する。実際のDNS登録(レジストラ
# でのAレコード/CNAME追加)は別途手動で行うこと。TLS証明書の取得・
# 自動更新は scripts/setup-tls.sh を参照。
#
# 注: WordPress高速化キット「KUSANAGI」のような高速化(gzip圧縮・静的
# アセットキャッシュ・FastCGIバッファ調整・keepaliveアップストリーム
# プーリング等)は本リポジトリの対象外。これらは open-runo/poem-cosmo-tauri
# 側でネイティブRust実装(hyperミドルウェア)として提供する方針(詳細は
# CLAUDE.mdの「関連プロジェクト」節を参照)。本スクリプトが生成する
# vhostはドメイン・HTTPS・基本的なリバースプロキシ配線のみを担当する。
#
# 使い方:
#   scripts/gen-vhost.sh [--stack=STACK] [--engine=ENGINE] <DOMAIN> <BIND_IP> [UPSTREAM] [WEBROOT]
#
# ENGINE (省略時は both、生成のみ・自動配置はしない):
#   nginx   - Nginx用vhostのみ生成
#   apache  - Apache用vhostのみ生成
#   both    - 両方生成(従来通り。配置するエンジンは利用者が選ぶ)
#
# 注: open-web-server はフロントのWebサーバー(vhost/TLS終端)ではなく、
# --stack=proxy のUPSTREAM(リバースプロキシ先)として指定するバックエンド。
# ENGINEの選択肢には含まれない(2026-07-13検証: gen-vhost.sh --stack=proxy
# 経由でのopen-web-server-gatewayへの疎通は確認済み、HANDOFF参照)。
#
# 登録済みドメインのエンジンを後から切り替える場合は
# scripts/switch-engine.sh <DOMAIN> <nginx|apache> を使うこと。
#
# STACK (省略時は static):
#   static     - 静的サイト配信のみ(open-easy-web自身など)。UPSTREAM不要。
#   proxy      - 汎用リバースプロキシ(aruaru-db・open-web-server・
#                open-raid-z系(Rust + Poem等)や任意のHTTPバックエンド向け)。
#   wordpress  - WordPress向け(PHP-FPM)。UPSTREAMにPHP-FPMの
#                ソケット/アドレスを指定。
#   laravel    - Laravel向け(PHP-FPM)。WEBROOTはpublic/を指定。
#   fastapi    - FastAPI(ASGI)向け(uvicorn/gunicorn手前のリバースプロキシ、
#                WebSocket/ストリーミング対応)。
#
# 例(open-easy-web本体、静的配信):
#   scripts/gen-vhost.sh --stack=static easyweb.example.com 203.0.113.10
# 例(aruaru-db等、汎用バックエンドへのリバースプロキシ):
#   scripts/gen-vhost.sh --stack=proxy tool.example.com 203.0.113.10 127.0.0.1:9000
# 例(WordPress、PHP-FPMソケット指定):
#   scripts/gen-vhost.sh --stack=wordpress blog.example.com 203.0.113.10 \
#     unix:/run/php/php8.3-fpm.sock /var/www/blog
# 例(Laravel、public/ディレクトリを明示):
#   scripts/gen-vhost.sh --stack=laravel app.example.com 203.0.113.10 \
#     unix:/run/php/php8.3-fpm.sock /var/www/app/public
# 例(FastAPI、ASGIサーバーへのリバースプロキシ):
#   scripts/gen-vhost.sh --stack=fastapi api.example.com 203.0.113.10 127.0.0.1:8000

set -euo pipefail

STACK="static"
ENGINE="both"
ARGS=()
for arg in "$@"; do
  case "$arg" in
    --stack=*) STACK="${arg#--stack=}" ;;
    --engine=*) ENGINE="${arg#--engine=}" ;;
    *) ARGS+=("$arg") ;;
  esac
done
set -- "${ARGS[@]+"${ARGS[@]}"}"

case "$STACK" in
  static|proxy|wordpress|laravel|fastapi) ;;
  *)
    echo "不明なスタック: '${STACK}'(static/proxy/wordpress/laravel/fastapi のいずれかを指定)" >&2
    exit 1
    ;;
esac

case "$ENGINE" in
  nginx|apache|both) ;;
  *)
    echo "不明なエンジン: '${ENGINE}'(nginx/apache/both のいずれかを指定)" >&2
    exit 1
    ;;
esac

if [ $# -lt 2 ]; then
  echo "使い方: $0 [--stack=static|proxy|wordpress|laravel|fastapi] [--engine=nginx|apache|both] <DOMAIN> <BIND_IP> [UPSTREAM] [WEBROOT]" >&2
  exit 1
fi

DOMAIN="$1"
BIND_IP="$2"
UPSTREAM="${3:-}"
WEBROOT="${4:-/var/www/${DOMAIN}}"

if [ "$STACK" != "static" ] && [ -z "$UPSTREAM" ]; then
  echo "スタック '${STACK}' には UPSTREAM(バックエンドのソケット/host:port)の指定が必要です。" >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
OUT_DIR="${REPO_ROOT}/deploy/generated"
mkdir -p "${OUT_DIR}"

# nginxのnamed upstreamブロック名。ドメインごとに一意になるよう
# 英数字以外を "_" に置換する(proxy/fastapiスタックでのみ使用)。
UPSTREAM_NAME="up_$(echo "${DOMAIN}" | tr -c 'a-zA-Z0-9' '_')"

render() {
  local template="$1" out="$2"
  sed \
    -e "s#{{DOMAIN}}#${DOMAIN}#g" \
    -e "s#{{IP}}#${BIND_IP}#g" \
    -e "s#{{UPSTREAM_NAME}}#${UPSTREAM_NAME}#g" \
    -e "s#{{UPSTREAM}}#${UPSTREAM}#g" \
    -e "s#{{WEBROOT}}#${WEBROOT}#g" \
    "${template}" > "${out}"
  echo "生成: ${out}"
}

if [ "$ENGINE" = "nginx" ] || [ "$ENGINE" = "both" ]; then
  render "${REPO_ROOT}/deploy/nginx/vhost-${STACK}.conf.template"  "${OUT_DIR}/${DOMAIN}.nginx.conf"
fi
if [ "$ENGINE" = "apache" ] || [ "$ENGINE" = "both" ]; then
  render "${REPO_ROOT}/deploy/apache/vhost-${STACK}.conf.template" "${OUT_DIR}/${DOMAIN}.apache.conf"
fi

# 監視/自動更新対象ドメインの一覧に追記(scripts/check-all-tls.sh が参照)。
DOMAINS_FILE="${OUT_DIR}/domains.txt"
touch "${DOMAINS_FILE}"
if ! grep -qxF "${DOMAIN}" "${DOMAINS_FILE}"; then
  echo "${DOMAIN}" >> "${DOMAINS_FILE}"
fi

# ドメインごとの現在の有効エンジンを記録(scripts/switch-engine.sh が参照・更新)。
# ENGINE=both の場合は明示的な選択が無いため記録しない(未選択のまま)。
ENGINES_FILE="${OUT_DIR}/engines.txt"
touch "${ENGINES_FILE}"
if [ "$ENGINE" != "both" ]; then
  grep -vxE "${DOMAIN}:.*" "${ENGINES_FILE}" > "${ENGINES_FILE}.tmp" 2>/dev/null || true
  mv "${ENGINES_FILE}.tmp" "${ENGINES_FILE}" 2>/dev/null || true
  echo "${DOMAIN}:${ENGINE}" >> "${ENGINES_FILE}"
fi

cat <<EOF

次の手順:
  1. レジストリでドメイン/サブドメイン(${DOMAIN})のAレコードを ${BIND_IP} に向ける(DNS登録は各自実施)。
  2. 生成された設定ファイルを Nginx/Apache の設定ディレクトリに配置し、リロードする。
     - Nginx : ${OUT_DIR}/${DOMAIN}.nginx.conf
     - Apache: ${OUT_DIR}/${DOMAIN}.apache.conf
     (Apache版を使う場合は、テンプレート冒頭のコメントに記載のモジュールを
     事前に a2enmod で有効化しておくこと)
  3. HTTPS証明書を取得・自動更新を有効化する:
       scripts/setup-tls.sh ${DOMAIN} admin@${DOMAIN} ${WEBROOT}
  4. 有効期限の自動監視を有効化する(未設定なら1回だけ):
       deploy/systemd/install-systemd-units.sh
  5. 複数サイトを管理する場合は、このスクリプトをサイトごとに繰り返し実行する
     (スタック・UPSTREAM・WEBROOTを変えるだけでよい)。
  6. open-easy-web のGUI側「サイト管理」画面でも同じ接続先情報を登録しておくと、
     ブラウザ側から一覧・接続テストができる。
  7. 登録済みドメインのエンジン(Nginx/Apache)を後から切り替える場合:
       scripts/switch-engine.sh ${DOMAIN} nginx   # または apache
EOF
