# open-easy-web FAQ dotyczące samodzielnego hostingu (konfiguracja konta i 2FA)

📖 Inne języki: [日本語](manual-JAPAN.md) / [English](manual-ENGLISH.md) /
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

## P1. Jeśli pobiorę to i uruchomię na własnym VPS, komputerze, telefonie lub tablecie, czy mogę zarejestrować własny adres e-mail i numer telefonu?

**Tak, można.** W przeglądarce nie ma formularza samodzielnej rejestracji (publiczna rejestracja została celowo wyłączona 2026-07-15 ze względów bezpieczeństwa). Zamiast tego ustawiasz **własny** adres e-mail i numer telefonu jako jedyne konto logowania za pomocą **zmiennych środowiskowych** podczas uruchamiania serwera.

| Zmienna środowiskowa | Wymagane/Opcjonalne | Znaczenie |
|---|---|---|
| `OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL` | Wymagane | Twój własny adres e-mail |
| `OPEN_EASYWEB_FIXED_ACCOUNT_PHONE` | Opcjonalne | Twój własny numer telefonu |
| `OPEN_EASYWEB_FIXED_ACCOUNT_BACKUP_EMAIL` | Opcjonalne | Zapasowy adres e-mail |

Jeśli nie ustawisz numeru telefonu, wymagany jest zapasowy adres e-mail (przynajmniej jedno z tych dwóch musi być ustawione).

**Sposób konfiguracji w zależności od platformy:**
- **Windows / Linux (VPS itp.)**: ustaw jako zmienną środowiskową podczas instalacji lub w pliku usługi systemd.
- **Android**: wprowadź swój adres e-mail na ekranie „Konfiguracja stałego konta” w aplikacji (aplikacja odmówi uruchomienia, jeśli nie jest to ustawione — celowy środek bezpieczeństwa).

Krótko mówiąc: Twoja własna instancja self-hosted wykorzystuje dokładnie ten sam mechanizm, co instancja produkcyjna (easy-web.tokyo), która sama działa z adresem właściciela.

## P2. Jeśli mam tylko podstawowy telefon komórkowy (nie smartfon), czy mogę potwierdzić uwierzytelnianie dwuskładnikowe (2FA) na komputerze?

**Tak, można.** Ekran konfiguracji 2FA (TOTP za pomocą aplikacji uwierzytelniającej) nie wyświetla obrazu kodu QR do zeskanowania kamerą smartfona — wyświetla bezpośrednio **ciąg tajny w postaci zwykłego tekstu**.

Ten ciąg działa z dowolną aplikacją TOTP, która umożliwia ręczne wprowadzenie tajnego klucza — nie tylko z uwierzytelniaczami na smartfonach. Jeśli masz tylko podstawowy telefon, masz dwie opcje:

1. Użyj zamiast tego **OTP przez e-mail** (najprostsza opcja, jeśli Twój podstawowy telefon może odbierać e-maile od operatora).
2. Wprowadź ręcznie „tajny klucz” wyświetlany podczas konfiguracji 2FA do **aplikacji uwierzytelniającej na komputerze** (np. WinAuth lub rozszerzenie przeglądarki), a następnie odczytaj 6-cyfrowy kod wyświetlany na ekranie komputera podczas logowania.

Obie metody działają od razu bez specjalnej konfiguracji.
