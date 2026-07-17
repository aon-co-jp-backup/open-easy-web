//! SMTP経由でのOTPメール送信。`lettre`の同期SMTPクライアントを
//! `tokio::task::spawn_blocking`でオフロードして使う(async関数内で
//! 同期I/Oを直接呼ばない、という既存エコシステムの方針に合わせる)。

use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from: String,
}

impl SmtpConfig {
    pub fn from_env() -> Option<Self> {
        Some(Self {
            host: std::env::var("OPEN_EASYWEB_SMTP_HOST").ok()?,
            port: std::env::var("OPEN_EASYWEB_SMTP_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(587),
            username: std::env::var("OPEN_EASYWEB_SMTP_USERNAME").ok()?,
            password: std::env::var("OPEN_EASYWEB_SMTP_PASSWORD").ok()?,
            from: std::env::var("OPEN_EASYWEB_SMTP_FROM").ok()?,
        })
    }
}

#[derive(Debug)]
pub enum MailError {
    Build(String),
    Send(String),
}

impl std::fmt::Display for MailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MailError::Build(e) => write!(f, "メール作成失敗: {e}"),
            MailError::Send(e) => write!(f, "メール送信失敗: {e}"),
        }
    }
}

/// ログインおよび連絡先変更のUIが実際に公開されているURL。
/// (2026-07-15: 単なる案内文からクリック可能な実リンクへ変更)
const SITE_URL: &str = "https://easy-web.tokyo/";

fn build_and_send(config: &SmtpConfig, to: &str, code: &str) -> Result<(), MailError> {
    let body = format!(
        "open-easy-web ログイン用ワンタイムパスワード / One-time password for open-easy-web login\n\n\
         コード / Code: {code}\n\
         このコードは5分間有効です。 / This code is valid for 5 minutes.\n\n\
         心当たりがない場合はこのメールを無視してください。\n\
         If you did not request this, please ignore this email。\n\n\
         --\n\
         携帯電話番号やメールアドレスの変更はこちら / To change your phone number or email address:\n\
         {SITE_URL}\n\
         ログイン後、「アカウント / Account」→「メールアドレス変更 / Change email address」から変更できます。\n\
         After logging in, use \"アカウント / Account\" → \"メールアドレス変更 / Change email address\"."
    );

    let email = Message::builder()
        .from(config.from.parse().map_err(|e| MailError::Build(format!("{e}")))?)
        .to(to.parse().map_err(|e| MailError::Build(format!("{e}")))?)
        .subject("open-easy-web ログインコード / Login code")
        .header(ContentType::TEXT_PLAIN)
        .body(body)
        .map_err(|e| MailError::Build(format!("{e}")))?;

    let creds = Credentials::new(config.username.clone(), config.password.clone());
    let mailer = SmtpTransport::starttls_relay(&config.host)
        .map_err(|e| MailError::Send(format!("{e}")))?
        .port(config.port)
        .credentials(creds)
        .build();

    mailer.send(&email).map_err(|e| MailError::Send(format!("{e}")))?;
    Ok(())
}

/// OTPメールを非同期に送信する(内部でblocking taskへオフロード)。
pub async fn send_otp(config: SmtpConfig, to: String, code: String) -> Result<(), MailError> {
    tokio::task::spawn_blocking(move || build_and_send(&config, &to, &code))
        .await
        .map_err(|e| MailError::Send(format!("task panicked: {e}")))?
}

/// `field`の表示名(日英併記)。「主メール」「セカンドメール」
/// 「携帯電話番号」のどれについての変更確認メールかを本文に埋め込むため。
fn field_label(field: &str) -> &'static str {
    match field {
        "email" => "メールアドレス(主) / primary email address",
        "backup_email" => "セカンドメール / second (backup) email address",
        "phone" => "携帯電話番号 / phone number",
        _ => "連絡先 / contact",
    }
}

fn build_and_send_contact_change_confirmation(
    config: &SmtpConfig,
    current_email: &str,
    field: &str,
    new_value: &str,
    token: &str,
) -> Result<(), MailError> {
    let label = field_label(field);
    let confirm_url = format!("{SITE_URL}api/auth/confirm-email-change?token={token}");
    let body = format!(
        "{label} 変更の確認 / Confirm your {label} change\n\n\
         このメールアドレス({current_email})宛に、{label} を「{new_value}」へ変更する\
         リクエストが届いています。この変更に心当たりがある場合のみ、以下のリンクに\
         アクセスしてください。\n\n\
         {confirm_url}\n\n\
         このリンクは30分間有効です。心当たりがない場合は無視してください\
         (変更は自動的には行われません)。\n\n\
         --\n\
         A request to change this account's {label} to \"{new_value}\" was received.\n\
         If you initiated this, visit the link above to confirm. This link is valid for \
         30 minutes. If you did not request this, you can safely ignore this email — no \
         change will be made automatically."
    );

    let email = Message::builder()
        .from(config.from.parse().map_err(|e| MailError::Build(format!("{e}")))?)
        .to(current_email.parse().map_err(|e| MailError::Build(format!("{e}")))?)
        .subject(format!("{label} 変更の確認 / Confirm {label} change"))
        .header(ContentType::TEXT_PLAIN)
        .body(body)
        .map_err(|e| MailError::Build(format!("{e}")))?;

    let creds = Credentials::new(config.username.clone(), config.password.clone());
    let mailer = SmtpTransport::starttls_relay(&config.host)
        .map_err(|e| MailError::Send(format!("{e}")))?
        .port(config.port)
        .credentials(creds)
        .build();

    mailer.send(&email).map_err(|e| MailError::Send(format!("{e}")))?;
    Ok(())
}

/// 連絡先(主メール・セカンドメール・電話番号)変更の確認リンクを、
/// **現在登録済みの主メール宛に**送信する(新しい連絡先宛ではない——
/// アカウント乗っ取り防止のため)。
pub async fn send_contact_change_confirmation(
    config: SmtpConfig,
    current_email: String,
    field: String,
    new_value: String,
    token: String,
) -> Result<(), MailError> {
    tokio::task::spawn_blocking(move || {
        build_and_send_contact_change_confirmation(&config, &current_email, &field, &new_value, &token)
    })
    .await
    .map_err(|e| MailError::Send(format!("task panicked: {e}")))?
}
