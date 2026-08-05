# open-easy-web Selbsthosting-Handbuch (Kontoeinrichtung & 2FA)

📖 Andere Sprachen: [日本語](MANUAL.md) / [English](MANUAL-English.md) /
[中文](MANUAL-Chinese.md) / [한국어](MANUAL-Korea.md) /
[Español](MANUAL-Spain.md) / [Français](MANUAL-France.md) /
[Deutsch](MANUAL-Germany.md) / [Italiano](MANUAL-Italy.md) /
[Русский](MANUAL-Russia.md) / [العربية](MANUAL-Arabic.md) /
[Português](MANUAL-Portugal.md) / [Nederlands](MANUAL-Netherlands.md) /
[Türkçe](MANUAL-Turkey.md) / [Polski](MANUAL-Poland.md) /
[Tiếng Việt](MANUAL-Vietnam.md) / [ไทย](MANUAL-Thailand.md) /
[Bahasa Indonesia](MANUAL-Indonesia.md) / [हिन्दी](MANUAL-India.md)

---

## F1. Wenn ich dies herunterlade und auf meinem eigenen VPS, PC, Telefon oder Tablet betreibe, kann ich meine eigene E-Mail-Adresse und Telefonnummer registrieren?

**Ja.** Es gibt kein Self-Service-Registrierungsformular im Browser (die öffentliche Registrierung wurde am 2026-07-15 aus Sicherheitsgründen absichtlich deaktiviert). Stattdessen legen Sie **Ihre eigene** E-Mail-Adresse und Telefonnummer als einziges Login-Konto über **Umgebungsvariablen** beim Serverstart fest.

| Umgebungsvariable | Erforderlich/Optional | Bedeutung |
|---|---|---|
| `OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL` | Erforderlich | Ihre eigene E-Mail-Adresse |
| `OPEN_EASYWEB_FIXED_ACCOUNT_PHONE` | Optional | Ihre eigene Telefonnummer |
| `OPEN_EASYWEB_FIXED_ACCOUNT_BACKUP_EMAIL` | Optional | Eine Ersatz-E-Mail-Adresse |

Wenn Sie keine Telefonnummer angeben, ist eine Ersatz-E-Mail-Adresse erforderlich (mindestens eines von beiden muss gesetzt sein).

**Konfiguration je Plattform:**
- **Windows / Linux (VPS usw.)**: als Umgebungsvariable bei der Installation oder in der systemd-Dienstdatei festlegen.
- **Android**: die E-Mail-Adresse im App-internen Bildschirm „Feste Kontoeinrichtung" eingeben (die App verweigert den Start, wenn dies nicht gesetzt ist — eine bewusste Sicherheitsmaßnahme).

Kurz gesagt: Ihre eigene selbstgehostete Instanz verwendet genau denselben Mechanismus wie die Produktionsinstanz (easy-web.tokyo), die selbst mit der eigenen Adresse des Betreibers läuft.

## F2. Wenn ich nur ein einfaches Mobiltelefon (kein Smartphone) besitze, kann ich die Zwei-Faktor-Authentifizierung (2FA) auf meinem PC bestätigen?

**Ja.** Der 2FA-Einrichtungsbildschirm (TOTP über Authenticator-App) zeigt kein QR-Code-Bild zum Scannen mit der Smartphone-Kamera an — er zeigt direkt die **geheime Zeichenkette als Klartext** an.

Diese Zeichenkette funktioniert mit jeder TOTP-App, die eine manuelle Eingabe des Geheimnisses erlaubt — nicht nur mit Smartphone-Authenticatoren. Wenn Sie nur ein einfaches Mobiltelefon besitzen, haben Sie zwei Möglichkeiten:

1. Verwenden Sie stattdessen **E-Mail-OTP** (die einfachste Option, wenn Ihr einfaches Telefon Anbieter-E-Mails empfangen kann).
2. Geben Sie das bei der 2FA-Einrichtung angezeigte „Geheimnis" manuell in eine **PC-Authenticator-App** ein (z. B. WinAuth oder eine Browsererweiterung), und lesen Sie beim Anmelden den 6-stelligen Code auf Ihrem PC-Bildschirm ab.

Beide Wege funktionieren ohne zusätzliche Konfiguration sofort.
