//! 更新のお知らせメール(標準機能、2026-11-04新設、ユーザー指示「バージョンアップ更新が必要なタイミングや自動更新では対応できない
//! 範囲でClaudeにお願いしたい場合などに、e-mailでお知らせする機能と、e-mail登録機能を open-easy-web に搭載して」)。
//!
//! - **宛先の登録**: 管理者(`x-admin-token`)が、お知らせを受け取るメールアドレスを登録・削除する。
//!   公開の自己登録にはしない(確認メールの無い登録は、他人のアドレスへの迷惑メールの踏み台になるため)。
//! - **お知らせの送信**: 外部の仕組み(`version-control`、`rsa-rs` の毎晩・毎朝の更新、自動保守など)が
//!   `POST /admin/notify/send` を呼ぶと、登録済みの全宛先へ日英併記で送る。種別は
//!   `update-needed`(更新が必要)/ `needs-claude`(自動更新では対応できず、Claudeへの依頼が必要)/ `heavy-test`(重い試験が必要で、VPSでは走らせず手元で行う)/ `info`。
//! - **連投の抑制**: 同じ件名は、一定時間(既定6時間)内には再送しない(毎晩の実行で同じ知らせが溜まらないように)。
//! - 履歴(直近50件)は状態ファイルに残し、画面に出す。SMTP は `OPEN_EASYWEB_SMTP_*`(`mail.rs`)を使う。

use crate::mail::{self, SmtpConfig};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

const MAX_RECIPIENTS: usize = 20;
const MAX_HISTORY: usize = 50;
const MAX_SUBJECT: usize = 200;
const MAX_BODY: usize = 8000;
/// 同じ件名を再送しない時間(秒)。
const DEDUP_SECS: i64 = 6 * 3600;

fn limits(max_bytes: usize) -> rust_json::Limits {
    rust_json::Limits { max_bytes, max_depth: 16, reject_duplicate_keys: true }
}

/// 外から来るリクエスト本文(JSON)を、RS-JSON で、上限つき(16 KiB)で読む。
pub fn parse_body<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    let text = std::str::from_utf8(bytes).map_err(|e| format!("本文が UTF-8 ではありません: {e}"))?;
    rust_json::from_str_secure(text, &limits(16 * 1024)).map_err(|e| format!("invalid JSON: {e}"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub at: i64,
    pub kind: String,
    pub subject: String,
    /// 送った宛先の数。
    pub sent: usize,
    /// 送れなかった宛先の数。
    pub failed: usize,
    /// 連投の抑制で送らなかった。
    #[serde(default)]
    pub suppressed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Saved {
    #[serde(default)]
    recipients: Vec<String>,
    #[serde(default)]
    history: Vec<Event>,
}

pub struct UpdateNotify {
    path: PathBuf,
    inner: Mutex<Saved>,
}

#[derive(Debug, Deserialize)]
pub struct SendRequest {
    pub kind: String,
    pub subject: String,
    pub body: String,
}

fn now_secs() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// メールアドレスとして使えるか(`lettre` の解析 + 改行・空白の拒否)。
pub fn valid_email(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 254
        && !s.chars().any(|c| c.is_whitespace() || c.is_control() || c == ',' || c == ';' || c == '<' || c == '>')
        && s.parse::<lettre::Address>().is_ok()
}

fn kind_label(kind: &str) -> &'static str {
    match kind {
        "update-needed" => "更新が必要です / Update needed",
        "needs-claude" => "自動更新では対応できません。Claudeへのご依頼が必要です / Automatic update cannot handle this: please ask Claude",
        "heavy-test" => "重い試験が必要です。VPSでは走らせず、手元(WSL)で実施してください / Heavy tests needed: not run on the VPS, please run them locally (WSL)",
        _ => "お知らせ / Notice",
    }
}

impl UpdateNotify {
    pub fn load(path: PathBuf) -> Self {
        let saved = std::fs::read_to_string(&path).ok().and_then(|t| rust_json::from_str_secure::<Saved>(&t, &limits(1024 * 1024)).ok()).unwrap_or_default();
        Self { path, inner: Mutex::new(saved) }
    }

    fn save(&self, s: &Saved) {
        if let Ok(text) = rust_json::fast::to_string_pretty(s) {
            if let Some(dir) = self.path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let tmp = self.path.with_extension("json.tmp");
            if std::fs::write(&tmp, text).is_ok() {
                let _ = std::fs::rename(&tmp, &self.path);
            }
        }
    }

    pub fn view(&self, smtp_configured: bool) -> serde_json::Value {
        let g = self.inner.lock().map(|g| g.clone()).unwrap_or_default();
        serde_json::json!({ "recipients": g.recipients, "history": g.history, "smtp_configured": smtp_configured })
    }

    /// 宛先の一覧を置き換える(重複と大文字小文字の違いは除く)。
    pub fn set_recipients(&self, emails: Vec<String>) -> Result<Vec<String>, String> {
        let mut out: Vec<String> = Vec::new();
        for e in emails {
            let e = e.trim().to_string();
            if !valid_email(&e) {
                return Err(format!("メールアドレスとして使えません: {e}"));
            }
            if !out.iter().any(|x| x.eq_ignore_ascii_case(&e)) {
                out.push(e);
            }
        }
        if out.len() > MAX_RECIPIENTS {
            return Err(format!("宛先は {MAX_RECIPIENTS} 件までです"));
        }
        let mut g = self.inner.lock().map_err(|_| "内部エラー".to_string())?;
        g.recipients = out.clone();
        self.save(&g);
        Ok(out)
    }

    fn record(&self, ev: Event) {
        if let Ok(mut g) = self.inner.lock() {
            g.history.push(ev);
            let n = g.history.len();
            if n > MAX_HISTORY {
                g.history.drain(0..n - MAX_HISTORY);
            }
            self.save(&g);
        }
    }

    /// 登録済みの全宛先へお知らせを送る。`force` でないとき、同じ件名が `DEDUP_SECS` 以内なら送らない。
    pub async fn send(&self, smtp: Option<SmtpConfig>, req: SendRequest, force: bool) -> Result<serde_json::Value, String> {
        let kind = match req.kind.as_str() {
            "update-needed" | "needs-claude" | "heavy-test" | "info" => req.kind.clone(),
            _ => return Err("kind は update-needed / needs-claude / heavy-test / info のどれかです".to_string()),
        };
        let subject: String = req.subject.chars().filter(|c| !c.is_control()).collect();
        if subject.trim().is_empty() || subject.chars().count() > MAX_SUBJECT {
            return Err(format!("subject は 1〜{MAX_SUBJECT} 文字です"));
        }
        if req.body.chars().count() > MAX_BODY {
            return Err(format!("body は {MAX_BODY} 文字までです"));
        }
        let smtp = smtp.ok_or_else(|| "SMTP が未設定です(OPEN_EASYWEB_SMTP_*)".to_string())?;
        let (recipients, suppressed) = {
            let g = self.inner.lock().map_err(|_| "内部エラー".to_string())?;
            let now = now_secs();
            let dup = !force && g.history.iter().rev().any(|e| !e.suppressed && e.sent > 0 && e.subject == subject && now - e.at < DEDUP_SECS);
            (g.recipients.clone(), dup)
        };
        if recipients.is_empty() {
            return Err("宛先が登録されていません".to_string());
        }
        if suppressed {
            self.record(Event { at: now_secs(), kind, subject, sent: 0, failed: 0, suppressed: true });
            return Ok(serde_json::json!({ "sent": 0, "failed": 0, "suppressed": true }));
        }
        let full_subject = format!("[open-easy-web] {subject}");
        let body = format!(
            "{}\n\n{}\n\n--\n\
             このメールは open-easy-web の「更新のお知らせ」機能から送られました。宛先の変更は管理画面で行えます。\n\
             This message was sent by the update-notice feature of open-easy-web. Recipients can be changed in the admin page.",
            kind_label(&kind),
            req.body
        );
        let (mut sent, mut failed) = (0usize, 0usize);
        for to in recipients {
            match mail::send_notice(smtp.clone(), to, full_subject.clone(), body.clone()).await {
                Ok(()) => sent += 1,
                Err(_) => failed += 1,
            }
        }
        self.record(Event { at: now_secs(), kind, subject, sent, failed, suppressed: false });
        Ok(serde_json::json!({ "sent": sent, "failed": failed, "suppressed": false }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> (tempfile::TempDir, UpdateNotify) {
        let d = tempfile::tempdir().unwrap();
        let n = UpdateNotify::load(d.path().join("n.json"));
        (d, n)
    }

    #[test]
    fn email_validation_rejects_injection_and_junk() {
        assert!(valid_email("a@example.com"));
        assert!(!valid_email("a@example.com\nBcc: x@y.z"));
        assert!(!valid_email("a@example.com, b@example.com"));
        assert!(!valid_email("<a@example.com>"));
        assert!(!valid_email("no-at-sign"));
        assert!(!valid_email(""));
    }

    #[test]
    fn recipients_are_deduped_validated_capped_and_persisted() {
        let (d, n) = tmp();
        let r = n.set_recipients(vec!["A@Example.com".into(), "a@example.com".into(), " b@example.com ".into()]).unwrap();
        assert_eq!(r, vec!["A@Example.com", "b@example.com"]);
        assert!(n.set_recipients(vec!["bad".into()]).is_err());
        assert!(n.set_recipients((0..21).map(|i| format!("u{i}@example.com")).collect()).is_err());
        // 失敗した設定は、前の宛先を壊さない。
        let n2 = UpdateNotify::load(d.path().join("n.json"));
        assert_eq!(n2.view(true)["recipients"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn send_validates_kind_subject_and_needs_smtp_and_recipients() {
        let (_d, n) = tmp();
        let req = |k: &str, s: &str| SendRequest { kind: k.into(), subject: s.into(), body: "b".into() };
        assert!(n.send(None, req("bogus", "s"), false).await.is_err());
        assert!(n.send(None, req("info", "  "), false).await.is_err());
        // SMTP が無ければ、宛先の有無の前に断る。
        assert!(n.send(None, req("info", "s"), false).await.unwrap_err().contains("SMTP"));
    }

    #[test]
    fn history_is_capped() {
        let (_d, n) = tmp();
        for i in 0..60 {
            n.record(Event { at: i, kind: "info".into(), subject: format!("s{i}"), sent: 1, failed: 0, suppressed: false });
        }
        assert_eq!(n.view(true)["history"].as_array().unwrap().len(), MAX_HISTORY);
    }
}
