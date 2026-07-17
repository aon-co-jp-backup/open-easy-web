//! HTTPS証明書の自動取得(Let's Encrypt / certbot)。
//!
//! ドメインのnginx vhostが(webroot方式のACMEチャレンジに応答できる状態で)
//! 既に配置・reload済みであることを前提に、`certbot certonly --webroot`を
//! 実行して証明書を取得する。取得済みかどうかは
//! `/etc/letsencrypt/live/<domain>/fullchain.pem`の有無で判定する
//! (二重取得を避ける)。

use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
pub enum TlsError {
    CertbotFailed(String),
    CertbotNotFound(std::io::Error),
}

impl std::fmt::Display for TlsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TlsError::CertbotFailed(msg) => write!(f, "certbot failed: {msg}"),
            TlsError::CertbotNotFound(e) => write!(f, "certbot not found: {e}"),
        }
    }
}

pub fn cert_path(domain: &str) -> PathBuf {
    Path::new("/etc/letsencrypt/live").join(domain).join("fullchain.pem")
}

pub fn cert_exists(domain: &str) -> bool {
    cert_path(domain).exists()
}

/// `certbot certonly --webroot`で証明書を取得する。既に取得済みなら
/// 何もせずOkを返す(冪等)。
pub fn ensure_cert(domain: &str, webroot: &Path, email: &str) -> Result<(), TlsError> {
    if cert_exists(domain) {
        return Ok(());
    }

    let output = Command::new("certbot")
        .arg("certonly")
        .arg("--webroot")
        .arg("-w")
        .arg(webroot)
        .arg("-d")
        .arg(domain)
        .arg("--non-interactive")
        .arg("--agree-tos")
        .arg("-m")
        .arg(email)
        .output()
        .map_err(TlsError::CertbotNotFound)?;

    if output.status.success() && cert_exists(domain) {
        Ok(())
    } else {
        Err(TlsError::CertbotFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cert_path_uses_letsencrypt_live_convention() {
        let path = cert_path("example.tokyo");
        assert_eq!(
            path,
            Path::new("/etc/letsencrypt/live/example.tokyo/fullchain.pem")
        );
    }

    #[test]
    fn cert_exists_is_false_for_unknown_domain() {
        assert!(!cert_exists("definitely-not-a-real-domain-xyz123.invalid"));
    }
}
