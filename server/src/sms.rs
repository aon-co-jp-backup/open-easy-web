//! SMS経由でのOTP送信。特定のSMSゲートウェイに直接依存させず、
//! `OPEN_EASYWEB_SMS_WEBHOOK_URL`(+任意の`OPEN_EASYWEB_SMS_WEBHOOK_TOKEN`)
//! というシンプルなWebhook方式で任意のSMS APIプロバイダに接続できるようにする
//! (Twilio等、契約したプロバイダのURLをそのまま設定すれば動く想定)。
//! 未設定の場合はSMTPと同じく「設定されていない」エラーを返し、
//! メール(セカンドメール)経由のログインへフォールバックできるようにする。

#[derive(Debug, Clone)]
pub struct SmsConfig {
    pub webhook_url: String,
    pub webhook_token: Option<String>,
}

impl SmsConfig {
    pub fn from_env() -> Option<Self> {
        Some(Self {
            webhook_url: std::env::var("OPEN_EASYWEB_SMS_WEBHOOK_URL").ok()?,
            webhook_token: std::env::var("OPEN_EASYWEB_SMS_WEBHOOK_TOKEN").ok(),
        })
    }
}

#[derive(Debug)]
pub enum SmsError {
    Send(String),
}

impl std::fmt::Display for SmsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SmsError::Send(e) => write!(f, "SMS送信失敗: {e}"),
        }
    }
}

/// OTPをSMSで送信する。`webhook_url`へ`{"to": <電話番号>, "body": <本文>}`を
/// POSTするだけの薄い実装——実際のSMSゲートウェイAPI仕様への変換は
/// Webhook側(プロバイダのAPI Gatewayやサーバーレス関数)に任せる設計。
pub async fn send_otp(config: SmsConfig, to: String, code: String) -> Result<(), SmsError> {
    let body = format!(
        "open-easy-web ログインコード / login code: {code} (5分間有効 / valid for 5 minutes)\n\
         連絡先の変更: https://easy-web.tokyo/ にログイン後「アカウント」→「メールアドレス変更」から。 / \
         To change contact info, log in at https://easy-web.tokyo/ and use Account → Change email address."
    );
    let client = reqwest::Client::new();
    let mut request = client
        .post(&config.webhook_url)
        .json(&serde_json::json!({ "to": to, "body": body }));
    if let Some(token) = &config.webhook_token {
        request = request.bearer_auth(token);
    }
    let response = request.send().await.map_err(|e| SmsError::Send(e.to_string()))?;
    if !response.status().is_success() {
        return Err(SmsError::Send(format!("webhook returned {}", response.status())));
    }
    Ok(())
}
