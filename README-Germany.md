# open-easy-web

**„Das zweite KUSANAGI" — nach dem Upload per IP-Adresse starten und
Domain-Registrierung + automatisches HTTPS einfach anwenden (Rust →
WebAssembly, ohne Framework-Abhängigkeit)**

Wie das WordPress-Beschleunigungskit „KUSANAGI" will `open-easy-web` den
Weg von „App hochladen" zu **Start per IP-Adresse → vereinfachte
Domain-Registrierung → automatisches HTTPS** in einem Fluss abdecken.
Enthält einen „Site-Verwaltungs"-Bildschirm und erzeugt eine
Basis-Reverse-Proxy-vhost-Konfiguration (Nginx/Apache) für WordPress,
PHP+Laravel, Python+FastAPI oder beliebige Backends. **Keine
Datenbankanbindung** (bewusst außerhalb des Umfangs).

**Aufteilung vom 2026-07-13 aus `aruaru-web`**: alles, was `aruaru-web`
unter „einfache Domain/Subdomain-Registrierung und -Löschung",
„automatische HTTPS-Überwachung/-Ausstellung/-Erneuerung" und
„einfacher Site-Betrieb nach Upload" entwickelte — **alles außer den
KUSANAGI-Beschleunigungsfunktionen** — ist hierher verschoben worden.
Die Beschleunigungsfunktionen (gzip-Komprimierung, langfristiges
Caching statischer Assets, FastCGI-Puffer-Tuning, Upstream-Keepalive-
Pooling) werden nicht mehr als Nginx/Apache-Konfiguration erzeugt,
sondern als **native Rust-Implementierungen (hyper-Middleware) in
`open-runo`/`poem-cosmo-tauri`** konsolidiert.

📖 Andere Sprachen: [日本語](README-Japan.md) / [English](README-English.md) /
[中文](README-Chinese.md) / [한국어](README-Korea.md) / [Español](README-Spain.md) /
[Français](README-France.md) / [Deutsch](README-Germany.md) / [Italiano](README-Italy.md) /
[Русский](README-Russia.md) / [العربية](README-Arabic.md)

---

## Was heute funktioniert

- **Site-Verwaltungsbildschirm**: mehrere Deployment-Ziele registrieren/
  bearbeiten/löschen, gespeichert in `localStorage`. Pro Karte ein
  „Verbindungstest"-Button, Port-Validierung (1-65535), Bestätigung vor
  dem Löschen, JSON-Export/-Import.
- **Start per IP-Adresse**: `scripts/serve.sh <BIND_IP> <PORT>`.
- **vhost-Erzeugung + automatisches HTTPS**: `scripts/gen-vhost.sh
  [--stack=STACK] <DOMAIN> <BIND_IP> [UPSTREAM] [WEBROOT]` für 5
  Stacks: `static`, `proxy`, `wordpress`, `laravel`, `fastapi`. **Das
  Beschleunigungs-Tuning ist hier bewusst ausgeschlossen** — siehe
  `open-runo`/`poem-cosmo-tauri`.
- **Automatische HTTPS-Überwachung/-Erneuerung**:
  `scripts/setup-tls.sh` (Let's Encrypt via certbot),
  `deploy/systemd/install-systemd-units.sh` installiert Timer für
  Erneuerung (2x/Tag) und Ablaufüberwachung (1x/Tag).
- **VPS-Deployment**: `scripts/deploy-vps.ps1` (PowerShell).

## Was es bewusst nicht tut

- **Keine Web-Beschleunigung** (gzip/statisches Caching/FastCGI-Puffer/
  Upstream-Keepalive) — siehe die nativen Rust-Implementierungen von
  `open-runo`/`poem-cosmo-tauri`.
- **Keinerlei Datenbankanbindung**.
- Keine Authentifizierung, Paginierung oder automatischen Fehler-Retry.
- Kein natives App-Erlebnis wie Tauri (nur Browser-WASM).
- **Kein echter Domainkauf/DNS-Eintrag** (Registrar-Vorgang) oder
  VPS-Vertragsabschluss — das übernimmt der Nutzer selbst; dieses Repo
  automatisiert nur vhost-Erzeugung und TLS-Lebenszyklus für eine
  bereits registrierte Domain.

## Build

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.126
cargo build --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir pkg \
  target/wasm32-unknown-unknown/debug/open_easy_web.wasm
python -m http.server 8080
```

## In diesem Durchlauf verifiziert

`cargo check`/`build`/`clippy --target wasm32-unknown-unknown` alle
ohne Warnungen erfolgreich. `gen-vhost.sh` für alle 5 Stacks verifiziert.
Da diese Entwicklungsumgebung Windows ist und kein nginx/apache-Binary
verfügbar ist, wurde die echte Syntaxprüfung via `nginx -t`/
`apache2ctl configtest` in diesem Durchlauf **nicht** durchgeführt — die
Templates sind eine strikte Subtraktion (nur entfernte Direktiven) der
bereits verifizierten aruaru-web-Templates, ohne neue Syntax. Siehe
`CLAUDE.md` für den vollständigen ehrlichen Verifikationsstatus.

## Verwandte Projekte

- **aruaru-web**: https://github.com/aon-co-jp/aruaru-web
- **open-runo**: https://github.com/aon-co-jp/open-runo
- **poem-cosmo-tauri**: https://github.com/aon-co-jp/poem-cosmo-tauri
- **aruaru-db**: https://github.com/aon-co-jp/aruaru-db
- **open-web-server**: https://github.com/aon-co-jp/open-web-server
- **open-raid-z**: https://github.com/aon-co-jp/open-raid-z
- **rs-to-readme**: https://github.com/aon-co-jp/rs-to-readme

## License

Apache-2.0
