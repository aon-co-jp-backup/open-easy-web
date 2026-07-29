#!/bin/sh
# open-easy-webのアンインストールスクリプト(install.shの対になるもの、
# 2026-07-29新設)。
#
# ユーザー指示「アンインストール機能で扱っているDATAをGoogleドライブや
# その他Githubや関連の公開も非公開も選択可能としてその場所を選択して
# 保存可能。その逆も可能」への対応——サービス停止・削除の前に、
# データを退避するかどうかを尋ねる。
#
# 使い方: sudo ./uninstall.sh

set -eu

INSTALL_DIR="/usr/local/bin"
SERVICE_FILE="/etc/systemd/system/open-easy-web.service"

if [ "$(id -u)" -ne 0 ]; then
    echo "root権限で実行してください(例: sudo ./uninstall.sh)" >&2
    exit 1
fi

echo "==> アンインストール前にデータを退避しますか?(推奨)"
echo "    ユーザー情報・AI検出重み・自動アップデート設定等を保存できます。"
printf "    退避しますか? [Y/n]: "
read -r backup_answer || backup_answer="y"
case "$backup_answer" in
    [nN]*)
        echo "    データ退避をスキップします(この後のアンインストールで失われます)。"
        ;;
    *)
        echo "    保存先を選んでください: 1) ローカルのtar.gz  2) GitHubリポジトリ(公開/非公開はリポジトリ側の設定に従う)  3) rclone(Googleドライブ等)"
        printf "    番号を入力 [1-3]: "
        read -r backup_kind || backup_kind=""
        case "$backup_kind" in
            1)
                printf "    保存先パス [/root/open-easy-web-backup-$(date +%Y%m%d%H%M%S).tar.gz]: "
                read -r p
                p="${p:-/root/open-easy-web-backup-$(date +%Y%m%d%H%M%S).tar.gz}"
                "$(dirname "$0")/scripts/data-portability.sh" backup local "$p"
                ;;
            2)
                printf "    リポジトリURL(事前にGitHub側で公開/非公開を選んで作成しておいてください): "; read -r repo_url
                printf "    ブランチ [main]: "; read -r branch; branch="${branch:-main}"
                printf "    GitHubトークン: "; read -r gh_token
                "$(dirname "$0")/scripts/data-portability.sh" backup github "$repo_url" "$branch" "$gh_token"
                ;;
            3)
                printf "    rcloneリモート(例: gdrive:backups/open-easy-web.tar.gz): "; read -r remote
                "$(dirname "$0")/scripts/data-portability.sh" backup rclone "$remote"
                ;;
            *)
                echo "    選択が無効なため、データ退避をスキップします。"
                ;;
        esac
        ;;
esac

echo "==> サービスを停止・無効化します"
systemctl stop open-easy-web 2>/dev/null || true
systemctl disable open-easy-web 2>/dev/null || true

if [ -f "$SERVICE_FILE" ]; then
    echo "==> systemdユニットを削除($SERVICE_FILE)"
    rm -f "$SERVICE_FILE"
    systemctl daemon-reload
fi

if [ -f "${INSTALL_DIR}/open-easy-web-server" ]; then
    echo "==> バイナリを削除(${INSTALL_DIR}/open-easy-web-server)"
    rm -f "${INSTALL_DIR}/open-easy-web-server"
fi

echo "==> アンインストール完了。"
echo "    データファイル自体(/var/www/.open-easy-web-*.json等)は削除していません。"
echo "    完全に削除したい場合は手動で削除してください。"
