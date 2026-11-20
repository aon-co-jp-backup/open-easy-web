# open-easy-web Self-Hosting FAQ (Account Setup & 2FA)

📖 Other languages: [日本語](manual-JAPAN.md) / [English](manual-ENGLISH.md) /
[中文](manual-CHINA.md) / [한국어](manual-KOREA.md) /
[Español](manual-SPAIN.md) / [Français](manual-FRANCE.md) /
[Deutsch](manual-GERMANY.md) / [Italiano](manual-ITALY.md) /
[Русский](manual-RUSSIA.md) / [العربية](manual-ARABIA.md) /
[Português](manual-PORTUGAL.md) / [Nederlands](manual-NETHERLANDS.md) /
[Türkçe](manual-TURKEY.md) / [Polski](manual-POLAND.md) /
[Tiếng Việt](manual-VIETNAM.md) / [ไทย](manual-THAILAND.md) /
[Bahasa Indonesia](manual-INDONESIA.md) / [हिन्दी](manual-INDIA.md) /
[فارسی](manual-IRAN(PERUSHA).md)

---

## Q1. If I download this and run it on my own VPS, PC, phone, or tablet, can I register my own email address and phone number?

**Yes.** There is no self-service "sign up" form in the browser (public registration was intentionally disabled on 2026-07-15 for security reasons). Instead, you set **your own** email address and phone number as the single login account via **environment variables at startup**.

| Environment variable | Required/Optional | Meaning |
|---|---|---|
| `OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL` | Required | Your own email address |
| `OPEN_EASYWEB_FIXED_ACCOUNT_PHONE` | Optional | Your own phone number |
| `OPEN_EASYWEB_FIXED_ACCOUNT_BACKUP_EMAIL` | Optional | A backup email address |

If you don't set a phone number, a backup email is required (at least one of the two must be set).

**How to configure it per platform:**
- **Windows / Linux (VPS, etc.)**: set it as an environment variable at install time, or in the systemd service file.
- **Android**: enter your email address in the app's "Fixed Account Setup" screen (the app refuses to start if this is unset — a deliberate safety measure).

In short: your own self-hosted instance uses exactly the same mechanism as the production instance (easy-web.tokyo), which itself runs on the owner's own address.

## Q2. If I only have a feature phone, can I confirm two-factor authentication (2FA) on my PC?

**Yes.** The 2FA (authenticator-app TOTP) setup screen does not render a QR-code image for a smartphone camera to scan — it displays the **plain-text secret string** directly.

That string works with any TOTP app that lets you type in a secret manually — not just a smartphone authenticator. If you only have a feature phone, you have two options:

1. Use **email OTP** instead (the simplest option if your feature phone can receive carrier email).
2. Type the "secret" shown during 2FA setup into a **PC-based authenticator app** (e.g. WinAuth, or a browser-extension authenticator), then read the 6-digit code off your PC screen when logging in.

Both paths work out of the box with no special configuration needed.
