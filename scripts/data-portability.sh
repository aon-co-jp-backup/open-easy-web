#!/bin/sh
# open-easy-webのサーバー側データ(サイト管理のプロファイルはブラウザ
# localStorage側のため対象外——サーバーが実際に読み書きするファイルのみ)
# のバックアップ/リストア。
#
# ユーザー指示(2026-07-29)「インストール機能は、あらかじめ残っていた
# 関連DATAを取り込むかどうか質問する機能とアンインストール機能で扱っている
# DATAをGoogleドライブやその他Githubや関連の公開も非公開も選択可能として
# その場所を選択して保存可能。その逆も可能」への対応。
#
# 対象データファイル(既定パス、環境変数で変更されていればそちらを使う):
#   - $OPEN_EASYWEB_USERS_STATE          (既定 /var/www/.open-easy-web-users.json)
#   - $OPEN_EASYWEB_AI_STATE             (既定 /var/www/.open-easy-web-ai-state.json)
#   - $OPEN_EASYWEB_AUTO_UPDATE_SETTINGS_FILE (既定 /var/www/.open-easy-web-auto-update.json)
#   - $OPEN_EASYWEB_DIST_SYNC_JOURNAL_DIR (既定 /var/www/.open-easy-web-dist-sync-journal)
#
# 保存先(「その場所を選択して保存可能」への対応):
#   - local:  ローカルの任意パスへtar.gzとして保存/読み込み
#   - github: 指定リポジトリへpush/そこからpull(公開/非公開はユーザーが
#             GitHub側でリポジトリ作成時に選択、このスクリプトはトークンの
#             権限に従うのみ)
#   - rclone: `rclone`(ユーザー自身が事前に設定した任意のリモート——
#             Googleドライブに限らず、rcloneが対応する全クラウドストレージ
#             を選べる)。**Googleドライブ自体へのOAuth認証はこのスクリプト
#             から代行しない**(このエコシステム共通方針、他社サービスの
#             認証情報を代行取得しない)——ユーザーが`rclone config`で
#             事前にGoogleドライブ等のリモートを設定済みであることが前提。
#
# 正直な開示: バックアップ対象はサーバーが直接読み書きする上記4項目のみ。
# アップロードされたサイトファイル自体(`OPEN_EASYWEB_SITES_ROOT`、既定
# `/var/www`)は既定で巨大になりうるため、既定では対象外とし
# `--include-sites-root`を明示指定した場合のみ含める。

set -eu

DATA_FILES="${OPEN_EASYWEB_USERS_STATE:-/var/www/.open-easy-web-users.json} \
${OPEN_EASYWEB_AI_STATE:-/var/www/.open-easy-web-ai-state.json} \
${OPEN_EASYWEB_AUTO_UPDATE_SETTINGS_FILE:-/var/www/.open-easy-web-auto-update.json}"
DATA_DIRS="${OPEN_EASYWEB_DIST_SYNC_JOURNAL_DIR:-/var/www/.open-easy-web-dist-sync-journal}"

usage() {
    cat <<'EOF'
使い方:
  data-portability.sh backup local  <保存先.tar.gz>
  data-portability.sh backup github <repo-url> <branch> <github-token>
  data-portability.sh backup rclone <rclone-remote:パス>

  data-portability.sh restore local  <読み込み元.tar.gz>
  data-portability.sh restore github <repo-url> <branch> <github-token>
  data-portability.sh restore rclone <rclone-remote:パス>
EOF
}

build_archive() {
    dest_tgz="$1"
    include_sites_root="${2:-}"
    tmp_root="$(mktemp -d)"
    trap 'rm -rf "$tmp_root"' EXIT

    for f in $DATA_FILES; do
        if [ -f "$f" ]; then
            mkdir -p "$tmp_root$(dirname "$f")"
            cp "$f" "$tmp_root$f"
        fi
    done
    for d in $DATA_DIRS; do
        if [ -d "$d" ]; then
            mkdir -p "$tmp_root$(dirname "$d")"
            cp -r "$d" "$tmp_root$d"
        fi
    done
    if [ "$include_sites_root" = "--include-sites-root" ]; then
        sites_root="${OPEN_EASYWEB_SITES_ROOT:-/var/www}"
        if [ -d "$sites_root" ]; then
            mkdir -p "$tmp_root$(dirname "$sites_root")"
            cp -r "$sites_root" "$tmp_root$sites_root"
        fi
    fi

    tar czf "$dest_tgz" -C "$tmp_root" .
    echo "==> バックアップ作成完了: $dest_tgz ($(du -h "$dest_tgz" | cut -f1))"
}

extract_archive() {
    src_tgz="$1"
    echo "==> $src_tgz を展開し、元のパスへ復元します"
    tar xzf "$src_tgz" -C /
    echo "==> 復元完了"
}

cmd="${1:-}"
mode="${2:-}"

case "$cmd" in
    backup)
        case "$mode" in
            local)
                dest="${3:?保存先パス(例: /root/open-easy-web-backup.tar.gz)を指定してください}"
                build_archive "$dest"
                ;;
            github)
                repo_url="${3:?GitHubリポジトリURLを指定してください}"
                branch="${4:-main}"
                token="${5:?GitHubトークンを指定してください}"
                archive_tmp="$(mktemp -d)/backup.tar.gz"
                mkdir -p "$(dirname "$archive_tmp")"
                build_archive "$archive_tmp"
                repo_dir="$(mktemp -d)"
                auth_url=$(echo "$repo_url" | sed "s#https://#https://${token}:@#")
                git clone --branch "$branch" "$auth_url" "$repo_dir" 2>/dev/null || git clone "$auth_url" "$repo_dir"
                cp "$archive_tmp" "$repo_dir/open-easy-web-backup.tar.gz"
                git -C "$repo_dir" add open-easy-web-backup.tar.gz
                git -C "$repo_dir" -c user.email="open-easy-web@localhost" -c user.name="open-easy-web" \
                    commit -m "backup $(date -u +%Y-%m-%dT%H:%M:%SZ)" --allow-empty
                git -C "$repo_dir" push origin "$branch"
                echo "==> GitHub($repo_url, branch=$branch)へバックアップをpushしました"
                rm -rf "$repo_dir"
                ;;
            rclone)
                remote_path="${3:?rcloneリモート(例: gdrive:backups/open-easy-web.tar.gz)を指定してください}"
                if ! command -v rclone >/dev/null 2>&1; then
                    echo "rcloneがインストールされていません。事前に 'rclone config' でリモート(Googleドライブ等)を設定してください。" >&2
                    exit 1
                fi
                archive_tmp="$(mktemp -d)/backup.tar.gz"
                build_archive "$archive_tmp"
                rclone copyto "$archive_tmp" "$remote_path"
                echo "==> rclone経由で $remote_path へバックアップを保存しました"
                ;;
            *) usage; exit 1 ;;
        esac
        ;;
    restore)
        case "$mode" in
            local)
                src="${3:?読み込み元パスを指定してください}"
                extract_archive "$src"
                ;;
            github)
                repo_url="${3:?GitHubリポジトリURLを指定してください}"
                branch="${4:-main}"
                token="${5:?GitHubトークンを指定してください}"
                repo_dir="$(mktemp -d)"
                auth_url=$(echo "$repo_url" | sed "s#https://#https://${token}:@#")
                git clone --branch "$branch" --depth 1 "$auth_url" "$repo_dir"
                extract_archive "$repo_dir/open-easy-web-backup.tar.gz"
                rm -rf "$repo_dir"
                ;;
            rclone)
                remote_path="${3:?rcloneリモートを指定してください}"
                if ! command -v rclone >/dev/null 2>&1; then
                    echo "rcloneがインストールされていません。" >&2
                    exit 1
                fi
                archive_tmp="$(mktemp -d)/backup.tar.gz"
                rclone copyto "$remote_path" "$archive_tmp"
                extract_archive "$archive_tmp"
                ;;
            *) usage; exit 1 ;;
        esac
        ;;
    *) usage; exit 1 ;;
esac
