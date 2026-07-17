#!/usr/bin/env bash
# open-easy-web をローカル/LAN上の任意のIPアドレスから配信する(開発用の簡易サーバー)。
# 使い方: scripts/serve.sh [BIND_IP] [PORT]
# 例:    scripts/serve.sh 0.0.0.0 8080
#        scripts/serve.sh 192.168.1.50 8080
set -euo pipefail
BIND_IP="${1:-0.0.0.0}"
PORT="${2:-8080}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "open-easy-web を http://${BIND_IP}:${PORT}/ で配信します(index.html + pkg/)"
echo "先に 'cargo build --target wasm32-unknown-unknown && wasm-bindgen ...' で pkg/ を生成しておくこと。"
cd "${REPO_ROOT}"
exec python3 -m http.server "${PORT}" --bind "${BIND_IP}"
