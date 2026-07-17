//! PHP-FPM用nginx vhostの自動生成・配置・reload。
//!
//! `deploy/nginx/vhost-php.conf.template` を読み込みプレースホルダを
//! 置換する(既存`scripts/gen-vhost.sh`のsedロジックをRustで再実装、
//! 両者は用途が異なるため並存させる——シェル版は手動CLI運用、こちらは
//! UI経由の自動運用)。
//!
//! 書き込み先は `sites-available`ではなく`/etc/nginx/conf.d/`を使う。
//! nginxは`conf.d/`を`sites-enabled/`より先に読み込むため、他ツールが
//! 同じドメインのvhostを`sites-enabled/`側に後から生成しても、
//! こちらが優先される(実VPS運用でaruaru-easywebとの重複時に実証済みの
//! 挙動)。

use crate::tls;
use std::path::{Path, PathBuf};
use std::process::Command;

const TEMPLATE_PLACEHOLDER_DOMAIN: &str = "{{DOMAIN}}";
const TEMPLATE_PLACEHOLDER_IP: &str = "{{IP}}";
const TEMPLATE_PLACEHOLDER_UPSTREAM: &str = "{{UPSTREAM}}";
const TEMPLATE_PLACEHOLDER_WEBROOT: &str = "{{WEBROOT}}";

#[derive(Debug)]
pub struct VhostRequest<'a> {
    pub domain: &'a str,
    pub bind_ip: &'a str,
    pub php_fpm_upstream: &'a str,
    pub webroot: &'a Path,
    pub template_path: &'a Path,
    pub nginx_conf_d: &'a Path,
}

#[derive(Debug)]
pub enum VhostError {
    TemplateRead(std::io::Error),
    Write(std::io::Error),
    NginxTestFailed(String),
    NginxReloadFailed(String),
}

impl std::fmt::Display for VhostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VhostError::TemplateRead(e) => write!(f, "テンプレート読み込み失敗: {e}"),
            VhostError::Write(e) => write!(f, "vhost書き込み失敗: {e}"),
            VhostError::NginxTestFailed(msg) => write!(f, "nginx -t 失敗: {msg}"),
            VhostError::NginxReloadFailed(msg) => write!(f, "nginx reload失敗: {msg}"),
        }
    }
}

fn render_template(template: &str, req: &VhostRequest) -> String {
    template
        .replace(TEMPLATE_PLACEHOLDER_DOMAIN, req.domain)
        .replace(TEMPLATE_PLACEHOLDER_IP, req.bind_ip)
        .replace(TEMPLATE_PLACEHOLDER_UPSTREAM, req.php_fpm_upstream)
        .replace(
            TEMPLATE_PLACEHOLDER_WEBROOT,
            &req.webroot.to_string_lossy(),
        )
}

fn conf_path(nginx_conf_d: &Path, domain: &str) -> PathBuf {
    nginx_conf_d.join(format!("{domain}.conf"))
}

/// vhostを生成・配置し、`nginx -t`→`systemctl reload nginx`まで行う。
/// `nginx -t`が失敗した場合は書き込みをロールバックする。
pub fn apply(req: &VhostRequest) -> Result<PathBuf, VhostError> {
    let template = std::fs::read_to_string(req.template_path).map_err(VhostError::TemplateRead)?;
    let rendered = render_template(&template, req);
    let out_path = conf_path(req.nginx_conf_d, req.domain);

    let previous = std::fs::read_to_string(&out_path).ok();
    std::fs::write(&out_path, &rendered).map_err(VhostError::Write)?;

    let test = Command::new("nginx").arg("-t").output();
    match test {
        Ok(output) if output.status.success() => {}
        Ok(output) => {
            rollback(&out_path, previous);
            return Err(VhostError::NginxTestFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }
        Err(e) => {
            rollback(&out_path, previous);
            return Err(VhostError::NginxTestFailed(e.to_string()));
        }
    }

    let reload = Command::new("systemctl")
        .args(["reload", "nginx"])
        .output();
    match reload {
        Ok(output) if output.status.success() => Ok(out_path),
        Ok(output) => Err(VhostError::NginxReloadFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        )),
        Err(e) => Err(VhostError::NginxReloadFailed(e.to_string())),
    }
}

fn rollback(out_path: &Path, previous: Option<String>) {
    match previous {
        Some(content) => {
            let _ = std::fs::write(out_path, content);
        }
        None => {
            let _ = std::fs::remove_file(out_path);
        }
    }
}

/// ドメインのnginx vhost設定を削除する(ドメイン登録の取り消し)。
/// `nginx -t`が失敗した場合はロールバックする。ウェブルート配下の
/// アップロード済みファイルや取得済み証明書自体は削除しない
/// (破壊的操作を最小限にするため)。
pub fn remove(domain: &str, nginx_conf_d: &Path) -> Result<(), VhostError> {
    let out_path = conf_path(nginx_conf_d, domain);
    let previous = std::fs::read_to_string(&out_path).ok();
    if previous.is_none() {
        return Ok(());
    }
    std::fs::remove_file(&out_path).map_err(VhostError::Write)?;

    let test = Command::new("nginx").arg("-t").output();
    match test {
        Ok(output) if output.status.success() => {}
        Ok(output) => {
            rollback(&out_path, previous);
            return Err(VhostError::NginxTestFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }
        Err(e) => {
            rollback(&out_path, previous);
            return Err(VhostError::NginxTestFailed(e.to_string()));
        }
    }

    let reload = Command::new("systemctl").args(["reload", "nginx"]).output();
    match reload {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => Err(VhostError::NginxReloadFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        )),
        Err(e) => Err(VhostError::NginxReloadFailed(e.to_string())),
    }
}

#[derive(Debug)]
pub struct AutoTlsRequest<'a> {
    pub domain: &'a str,
    pub bind_ip: &'a str,
    pub php_fpm_upstream: &'a str,
    pub webroot: &'a Path,
    /// 証明書取得前に使うHTTPのみのテンプレート(サイトをHTTPで即配信)。
    pub http_only_template_path: &'a Path,
    /// 証明書取得後に使う、80→443リダイレクト+HTTPS配信のテンプレート。
    pub https_template_path: &'a Path,
    pub nginx_conf_d: &'a Path,
    /// Let's Encryptアカウント登録に使うメールアドレス。
    pub acme_email: &'a str,
}

#[derive(Debug)]
pub struct AutoTlsOutcome {
    pub vhost_path: PathBuf,
    pub https_enabled: bool,
    /// HTTPSが有効化できなかった場合の理由(certbot失敗時など)。
    pub tls_note: Option<String>,
}

#[derive(Debug)]
pub enum AutoTlsError {
    Vhost(VhostError),
}

impl std::fmt::Display for AutoTlsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AutoTlsError::Vhost(e) => write!(f, "{e}"),
        }
    }
}

/// ドメインを「登録」する一連の流れを自動化する:
/// (1) まずHTTPのみのvhostを適用してサイトを即座に閲覧可能にする
///     (ACME HTTP-01チャレンジにも同じwebrootで応答できる状態にする)。
/// (2) 証明書が未取得なら`certbot certonly --webroot`で取得を試みる。
/// (3) 取得できれば、80→443リダイレクト+HTTPS配信のvhostに差し替える。
///     取得できなくても(2)の時点でサイト自体はHTTPで動作し続ける
///     ——サイトの可用性は損なわない設計。
pub fn apply_with_auto_tls(req: &AutoTlsRequest) -> Result<AutoTlsOutcome, AutoTlsError> {
    let http_only = VhostRequest {
        domain: req.domain,
        bind_ip: req.bind_ip,
        php_fpm_upstream: req.php_fpm_upstream,
        webroot: req.webroot,
        template_path: req.http_only_template_path,
        nginx_conf_d: req.nginx_conf_d,
    };
    let vhost_path = apply(&http_only).map_err(AutoTlsError::Vhost)?;

    match tls::ensure_cert(req.domain, req.webroot, req.acme_email) {
        Ok(()) => {
            let https = VhostRequest {
                domain: req.domain,
                bind_ip: req.bind_ip,
                php_fpm_upstream: req.php_fpm_upstream,
                webroot: req.webroot,
                template_path: req.https_template_path,
                nginx_conf_d: req.nginx_conf_d,
            };
            match apply(&https) {
                Ok(path) => Ok(AutoTlsOutcome { vhost_path: path, https_enabled: true, tls_note: None }),
                Err(e) => Ok(AutoTlsOutcome {
                    vhost_path,
                    https_enabled: false,
                    tls_note: Some(format!(
                        "証明書は取得できましたが、HTTPS用vhostの適用に失敗しました: {e}"
                    )),
                }),
            }
        }
        Err(e) => Ok(AutoTlsOutcome {
            vhost_path,
            https_enabled: false,
            tls_note: Some(format!(
                "証明書取得に失敗したため、現在はHTTPのみで配信しています: {e}"
            )),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_template_substitutes_all_placeholders() {
        let template = "server_name {{DOMAIN}}; listen {{IP}}:80; \
             fastcgi_pass {{UPSTREAM}}; root {{WEBROOT}};";
        let req = VhostRequest {
            domain: "example.tokyo",
            bind_ip: "0.0.0.0",
            php_fpm_upstream: "unix:/run/php-fpm/www.sock",
            webroot: Path::new("/var/www/example.tokyo"),
            template_path: Path::new("unused-in-this-test"),
            nginx_conf_d: Path::new("unused-in-this-test"),
        };
        let rendered = render_template(template, &req);
        assert!(rendered.contains("server_name example.tokyo;"));
        assert!(rendered.contains("listen 0.0.0.0:80;"));
        assert!(rendered.contains("fastcgi_pass unix:/run/php-fpm/www.sock;"));
        assert!(rendered.contains("root /var/www/example.tokyo;"));
        assert!(!rendered.contains("{{"));
    }

    #[test]
    fn conf_path_uses_domain_conf_filename() {
        let path = conf_path(Path::new("/etc/nginx/conf.d"), "example.tokyo");
        assert_eq!(path, Path::new("/etc/nginx/conf.d/example.tokyo.conf"));
    }
}
