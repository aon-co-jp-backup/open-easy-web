# open-easy-web

**"Il secondo KUSANAGI" — avvia tramite indirizzo IP dopo l'upload, e
applica facilmente registrazione dominio + HTTPS automatico (Rust →
WebAssembly, senza dipendenza da framework)**

Come il kit di accelerazione per WordPress "KUSANAGI", `open-easy-web`
mira a portarti da "carica l'app" ad **avvio tramite IP → registrazione
dominio semplificata → HTTPS automatico** in un unico flusso. Include
una schermata di "gestione siti" e genera una configurazione vhost di
reverse-proxy di base (Nginx/Apache) per WordPress, PHP+Laravel,
Python+FastAPI o qualsiasi backend. **Nessuna connettività al
database** (deliberatamente fuori ambito).

**Scissione del 2026-07-13 da `aruaru-web`**: tutto ciò che
`aruaru-web` sviluppava sotto "facile registrazione/eliminazione di
domini/sottodomini", "monitoraggio/emissione/rinnovo HTTPS automatico"
e "facile gestione del sito dopo l'upload" —**tutto tranne le funzioni
di accelerazione di KUSANAGI**— è stato spostato qui. Le funzioni di
accelerazione (compressione gzip, cache di lunga durata degli asset
statici, tuning dei buffer FastCGI, pooling keepalive upstream) non
sono più generate come configurazione Nginx/Apache; sono state
consolidate come **implementazioni Rust native (middleware hyper) in
`open-runo`/`poem-cosmo-tauri`**.

📖 Altre lingue: [日本語](README-Japan.md) / [English](README-English.md) /
[中文](README-Chinese.md) / [한국어](README-Korea.md) / [Español](README-Spain.md) /
[Français](README-France.md) / [Deutsch](README-Germany.md) / [Italiano](README-Italy.md) /
[Русский](README-Russia.md) / [العربية](README-Arabic.md)

---

## Cosa funziona oggi

- **Schermata di gestione siti**: registra/modifica/elimina più
  destinazioni di deploy, salvate in `localStorage`. Pulsante "test di
  connessione" per scheda, validazione porta (1-65535), conferma prima
  dell'eliminazione, export/import JSON.
- **Avvio tramite IP**: `scripts/serve.sh <BIND_IP> <PORT>`.
- **Generazione vhost + HTTPS automatico**: `scripts/gen-vhost.sh
  [--stack=STACK] <DOMAIN> <BIND_IP> [UPSTREAM] [WEBROOT]` per 5
  stack: `static`, `proxy`, `wordpress`, `laravel`, `fastapi`. **Il
  tuning di accelerazione è deliberatamente escluso qui** — vedi
  `open-runo`/`poem-cosmo-tauri`.
- **Monitoraggio/rinnovo HTTPS automatico**: `scripts/setup-tls.sh`
  (Let's Encrypt via certbot), `deploy/systemd/
  install-systemd-units.sh` installa i timer di rinnovo (2x/giorno) e
  monitoraggio scadenza (1x/giorno).
- **Deploy VPS**: `scripts/deploy-vps.ps1` (PowerShell).

## Cosa deliberatamente non fa

- **Nessuna accelerazione web** (gzip/cache statica/buffer FastCGI/
  keepalive upstream) — vedi le implementazioni Rust native di
  `open-runo`/`poem-cosmo-tauri`.
- **Nessuna connettività al database di alcun tipo**.
- Nessuna autenticazione, paginazione o retry automatico degli errori.
- Nessuna esperienza app nativa come Tauri (solo WASM da browser).
- **Non esegue l'acquisto reale del dominio/registrazione DNS**
  (operazione del registrar) né la contrattazione VPS — spetta
  all'utente; questo repo automatizza solo la generazione vhost e il
  ciclo di vita TLS per un dominio già registrato.

## Build

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.126
cargo build --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir pkg \
  target/wasm32-unknown-unknown/debug/open_easy_web.wasm
python -m http.server 8080
```

> ⚠️ **Avviso di build (ambienti su unità di rete)**: se questo repo si
> trova su un'unità montata in rete (es. condivisione SMB), leggere/
> scrivere l'output `target/` di `cargo build` o l'input/output di
> `wasm-bindgen` direttamente su quell'unità **può restituire contenuti
> obsoleti subito dopo una scrittura** (incoerenza della cache di
> lettura, verificatasi realmente il 2026-07-20). Se una ricompilazione
> non sembra avere effetto, reindirizzare l'output della build su
> un'unità locale con `cargo build --target-dir <directory-temp-locale>`
> ed eseguire `wasm-bindgen` su quella copia locale.

## Verificato in questo passaggio

`cargo check`/`build`/`clippy --target wasm32-unknown-unknown` tutti
riusciti senza avvisi. `gen-vhost.sh` verificato per tutti i 5 stack.
Poiché questo ambiente di sviluppo è Windows senza binario nginx/apache
disponibile, la verifica sintattica reale tramite `nginx -t`/
`apache2ctl configtest` **non** è stata eseguita in questo passaggio —
i template sono una sottrazione rigorosa (solo direttive rimosse) dai
template già verificati di aruaru-web, senza nuova sintassi aggiunta.
Vedi `CLAUDE.md` per lo stato di verifica completo e onesto.

## Progetti correlati

- **aruaru-web**: https://github.com/aon-co-jp/aruaru-web
- **open-runo**: https://github.com/aon-co-jp/open-runo
- **poem-cosmo-tauri**: https://github.com/aon-co-jp/poem-cosmo-tauri
- **aruaru-db**: https://github.com/aon-co-jp/aruaru-db
- **open-web-server**: https://github.com/aon-co-jp/open-web-server
- **open-raid-z**: https://github.com/aon-co-jp/open-raid-z
- **rs-to-readme**: https://github.com/aon-co-jp/rs-to-readme

## License

Apache-2.0
