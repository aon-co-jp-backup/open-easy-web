# open-easy-web FAQ per l'auto-hosting (configurazione account e 2FA)

📖 Altre lingue: [日本語](manual-JAPAN.md) / [English](manual-ENGLISH.md) /
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

## D1. Se scarico questo software e lo eseguo sul mio VPS, PC, telefono o tablet, posso registrare il mio indirizzo email e numero di telefono?

**Sì.** Non esiste un modulo di registrazione self-service nel browser (la registrazione pubblica è stata disabilitata intenzionalmente il 2026-07-15 per motivi di sicurezza). Al contrario, si imposta il **proprio** indirizzo email e numero di telefono come unico account di accesso tramite **variabili d'ambiente** all'avvio del server.

| Variabile d'ambiente | Obbligatoria/Opzionale | Significato |
|---|---|---|
| `OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL` | Obbligatoria | Il proprio indirizzo email |
| `OPEN_EASYWEB_FIXED_ACCOUNT_PHONE` | Opzionale | Il proprio numero di telefono |
| `OPEN_EASYWEB_FIXED_ACCOUNT_BACKUP_EMAIL` | Opzionale | Un indirizzo email di riserva |

Se non si imposta un numero di telefono, è richiesta un'email di riserva (almeno una delle due deve essere impostata).

**Come configurarlo per piattaforma:**
- **Windows / Linux (VPS, ecc.)**: impostarla come variabile d'ambiente durante l'installazione, oppure nel file di servizio systemd.
- **Android**: inserire l'indirizzo email nella schermata "Configurazione account fisso" dell'app (l'app rifiuta di avviarsi se non è impostato — una misura di sicurezza deliberata).

In sintesi: la propria istanza self-hosted utilizza esattamente lo stesso meccanismo dell'istanza di produzione (easy-web.tokyo), che a sua volta funziona con l'indirizzo del proprietario.

## D2. Se ho solo un telefono cellulare di base (non uno smartphone), posso confermare l'autenticazione a due fattori (2FA) sul mio PC?

**Sì.** La schermata di configurazione 2FA (TOTP tramite app di autenticazione) non mostra un'immagine di codice QR da scansionare con la fotocamera dello smartphone — mostra direttamente la **stringa segreta in testo semplice**.

Questa stringa funziona con qualsiasi app TOTP che consenta l'inserimento manuale di un segreto — non solo con gli autenticatori per smartphone. Se si dispone solo di un telefono di base, sono disponibili due opzioni:

1. Utilizzare invece l'**OTP via email** (l'opzione più semplice se il telefono di base può ricevere email dell'operatore).
2. Inserire manualmente il "segreto" mostrato durante la configurazione 2FA in un'**app di autenticazione per PC** (ad esempio WinAuth o un'estensione del browser), quindi leggere il codice a 6 cifre sullo schermo del PC al momento dell'accesso.

Entrambi i metodi funzionano immediatamente senza configurazioni particolari.
