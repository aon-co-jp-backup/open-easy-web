# open-easy-web Selfhosting FAQ (accountinstellingen & 2FA)

📖 Andere talen: [日本語](manual-JAPAN.md) / [English](manual-ENGLISH.md) /
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

## V1. Als ik dit download en op mijn eigen VPS, pc, telefoon of tablet draai, kan ik dan mijn eigen e-mailadres en telefoonnummer registreren?

**Ja, dat kan.** Er is geen self-service registratieformulier in de browser (openbare registratie is op 2026-07-15 bewust uitgeschakeld om veiligheidsredenen). In plaats daarvan stelt u **uw eigen** e-mailadres en telefoonnummer in als het enige inlogaccount via **omgevingsvariabelen** bij het opstarten van de server.

| Omgevingsvariabele | Verplicht/Optioneel | Betekenis |
|---|---|---|
| `OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL` | Verplicht | Uw eigen e-mailadres |
| `OPEN_EASYWEB_FIXED_ACCOUNT_PHONE` | Optioneel | Uw eigen telefoonnummer |
| `OPEN_EASYWEB_FIXED_ACCOUNT_BACKUP_EMAIL` | Optioneel | Een back-up e-mailadres |

Als u geen telefoonnummer instelt, is een back-up e-mailadres vereist (minstens één van de twee moet ingesteld zijn).

**Configuratie per platform:**
- **Windows / Linux (VPS enz.)**: stel dit in als omgevingsvariabele bij installatie, of in het systemd-servicebestand.
- **Android**: voer uw e-mailadres in op het scherm "Vast account instellen" in de app (de app weigert te starten als dit niet is ingesteld — een bewuste veiligheidsmaatregel).

Kortom: uw eigen selfhosted instantie gebruikt precies hetzelfde mechanisme als de productie-instantie (easy-web.tokyo), die op zijn beurt draait met het eigen adres van de eigenaar.

## V2. Als ik alleen een eenvoudige mobiele telefoon heb (geen smartphone), kan ik dan tweefactorauthenticatie (2FA) op mijn pc bevestigen?

**Ja, dat kan.** Het instellingsscherm voor 2FA (TOTP via authenticator-app) toont geen QR-code-afbeelding om te scannen met een smartphonecamera — het toont rechtstreeks de **geheime tekenreeks als platte tekst**.

Deze tekenreeks werkt met elke TOTP-app waarin u een geheim handmatig kunt invoeren — niet alleen met smartphone-authenticators. Als u alleen een eenvoudige telefoon heeft, heeft u twee opties:

1. Gebruik in plaats daarvan **e-mail-OTP** (de eenvoudigste optie als uw eenvoudige telefoon providermail kan ontvangen).
2. Voer het "geheim" dat tijdens de 2FA-instelling wordt getoond handmatig in bij een **pc-authenticator-app** (bijvoorbeeld WinAuth of een browserextensie), en lees vervolgens de 6-cijferige code op uw pc-scherm bij het inloggen.

Beide methoden werken direct zonder speciale configuratie.
