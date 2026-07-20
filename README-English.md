# open-easy-web

**A second KUSANAGI — launch by IP address after upload, and easily
apply domain registration + automatic HTTPS (Rust → WebAssembly, no
framework dependency)**

Like the WordPress speed-up server kit "KUSANAGI", `open-easy-web` aims
to take you from "upload the app" to **launch by IP address → easy
domain registration → automatic HTTPS** in one flow. It includes a
"site management" screen to register/switch/test multiple site
endpoints, and generates basic reverse-proxy vhost config (Nginx/Apache)
for WordPress, PHP+Laravel, Python+FastAPI, or any backend stack.
**It has no database connectivity** (intentionally out of scope).

**2026-07-13 split from `aruaru-web`**: everything `aruaru-web` was
developing under "easy domain/subdomain registration and deletion",
"automatic HTTPS monitor/issue/renew", and general "easy post-upload
site operations" — **everything except KUSANAGI's web speed-up
features** — has moved here. The speed-up features (gzip compression,
static-asset long-lived caching, FastCGI buffer tuning, upstream
keepalive pooling) are no longer generated as Nginx/Apache config;
they've instead been consolidated into **native Rust (hyper
middleware) implementations in `open-runo`/RPoem (formerly
poem-cosmo-tauri)** (gzip
response-compression middleware, static-asset Cache-Control
middleware, etc. — see those repos' CLAUDE.md for details).

📖 Other languages: [日本語](README-Japan.md) / [English](README-English.md) /
[中文](README-Chinese.md) / [한국어](README-Korea.md) / [Español](README-Spain.md) /
[Français](README-France.md) / [Deutsch](README-Germany.md) / [Italiano](README-Italy.md) /
[Русский](README-Russia.md) / [العربية](README-Arabic.md)

---

## What works today

- **Site management screen**: register/edit/delete multiple deploy
  targets (name/purpose/protocol/host/port/path/backend stack) for
  open-easy-web itself, WordPress, Laravel, FastAPI, or anything else,
  saved to `localStorage`. Per-card "connection test" button (plain
  HTTP reachability check via `fetch(url, {mode: 'no-cors'})`), port
  validation (1-65535), delete confirmation dialog, JSON export/import
  of the registered site list.
- **Launch by IP address**: `scripts/serve.sh <BIND_IP> <PORT>`.
- **vhost generation + automatic HTTPS**: `scripts/gen-vhost.sh
  [--stack=STACK] <DOMAIN> <BIND_IP> [UPSTREAM] [WEBROOT]` generates
  Nginx/Apache vhosts (HTTP→HTTPS redirect + ACME challenge path
  included) for 5 stacks: `static`, `proxy` (generic reverse proxy),
  `wordpress`, `laravel`, `fastapi`. **Speed-up tuning is deliberately
  not included here** — see `open-runo`/RPoem (formerly poem-cosmo-tauri).
- **Automatic HTTPS monitor/renew**: `scripts/setup-tls.sh` (Let's
  Encrypt via certbot), `deploy/systemd/install-systemd-units.sh`
  installs `easyweb-tls-renew.timer` (certbot renew, twice daily) and
  `easyweb-tls-monitor.timer` (expiry monitor, daily).
- **VPS deploy**: `scripts/deploy-vps.ps1` (Windows PowerShell)
  automates build → upload → launch.
- **Password-free account authentication**: no fixed passwords at all —
  log in via a one-time password (OTP) sent to whichever contact you
  registered (primary email, a second email, or a phone number).
  Authenticator-app 2FA (TOTP) can be enabled, and **either the email
  OTP or the authenticator code alone is enough to log in** (a
  dedicated login path lets a 2FA-enabled account skip the email OTP
  entirely and authenticate with just the 6-digit authenticator code).
  Contact-info changes are always confirmed via a link sent to the
  *current* primary email, never the new one (prevents account
  takeover). **As of 2026-07-15, public sign-up is disabled for security
  reasons — only a single fixed account seeded at startup can log in**
  (`FIXED_ACCOUNT_EMAIL` in `server/src/main.rs`). Running multiple
  accounts currently requires editing that fixed-account setup for your
  own deployment.
- **AI-driven automatic PHP detection**: uploading files to a site
  triggers a self-learning AI (no external LLM, no contract) that
  scores file-extension/`<?php` tag/`wp-config.php`/`composer.json`
  signatures to decide whether the site is PHP, and if so auto-
  generates and installs the matching nginx + PHP-FPM vhost. Detections
  can be manually corrected, and each correction nudges the AI's
  weights online (EWMA).
- **Dynamic registration with a shared backend ("bunshin no jutsu")**:
  instead of installing a separate `open-runo`/RPoem (formerly
  poem-cosmo-tauri)
  process per domain, a site's domain can be dynamically registered
  with an already-running shared backend instance.

## What it deliberately does not do

- **No web speed-up** (gzip/static caching/FastCGI buffer tuning/
  upstream keepalive pooling) — see `open-runo`/RPoem (formerly
  poem-cosmo-tauri)'s
  native Rust implementations instead.
- **No database connectivity** of any kind.
- Pagination and automatic error retry are not implemented.
- No native-app experience like Tauri (browser-run WASM only).
- **Does not perform actual domain purchase/DNS record registration**
  (a registrar operation) or VPS contracting — those are the user's
  responsibility; this repo only automates vhost generation and TLS
  cert lifecycle for an already-registered domain.

## Build

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.126
cargo build --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir pkg \
  target/wasm32-unknown-unknown/debug/open_easy_web.wasm
python -m http.server 8080
```

> ⚠️ **Build caveat (network-drive environments)**: if this repo lives on
> a network-mounted drive (e.g. an SMB share), reading/writing `cargo
> build`'s `target/` output or `wasm-bindgen`'s input/output directly on
> that drive can return stale content immediately after a write (a
> read-cache coherency issue actually hit on 2026-07-20). If a rebuild
> doesn't seem to take effect, point the build output at a local drive
> with `cargo build --target-dir <local-temp-dir>` and run `wasm-bindgen`
> against that local copy instead.

## Launch by IP

```bash
scripts/serve.sh 0.0.0.0 8080
```

## vhost generation

```bash
scripts/gen-vhost.sh --stack=static easyweb.example.com 203.0.113.10
scripts/gen-vhost.sh --stack=proxy tool.example.com 203.0.113.10 127.0.0.1:9000
scripts/gen-vhost.sh --stack=wordpress blog.example.com 203.0.113.10 \
  unix:/run/php/php8.3-fpm.sock /var/www/blog
scripts/gen-vhost.sh --stack=laravel app.example.com 203.0.113.10 \
  unix:/run/php/php8.3-fpm.sock /var/www/app/public
scripts/gen-vhost.sh --stack=fastapi api.example.com 203.0.113.10 127.0.0.1:8000

scripts/setup-tls.sh easyweb.example.com admin@example.com /var/www/easyweb.example.com
sudo deploy/systemd/install-systemd-units.sh
```

## Verified this pass

`cargo check`/`build`/`clippy --target wasm32-unknown-unknown` all
succeed with zero warnings. `gen-vhost.sh` verified for all 5 stacks
(placeholder substitution correct). `nginx -t`/`apache2ctl configtest`
against a real installed Nginx/Apache was **not** performed this pass
(Windows dev environment, no nginx/apache binary available) — the
templates are a strict subtraction (removed directives only) from
aruaru-web's templates, which *were* syntax-verified in a Linux
container in a prior pass; no syntax was added. See `CLAUDE.md` for the
full honest verification status.

## Related projects

- **aruaru-web** (split source): https://github.com/aon-co-jp/aruaru-web
- **open-runo**: https://github.com/aon-co-jp/open-runo
- **RPoem** (formerly poem-cosmo-tauri): https://github.com/aon-co-jp/RPoem
- **aruaru-db**: https://github.com/aon-co-jp/aruaru-db
- **open-web-server**: https://github.com/aon-co-jp/open-web-server
- **open-raid-z** (canonical dev rules): https://github.com/aon-co-jp/open-raid-z
- **rs-to-readme**: https://github.com/aon-co-jp/rs-to-readme

## License

Apache-2.0
