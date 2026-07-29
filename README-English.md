# open-easy-web

> **Updated 2026-07-25**: The dev-policy file (`CLAUDE.md`) heading was
> renamed from "Development Policy & Dev Environment Rules" to
> "Design Philosophy & Development Policy & Dev Environment Rules",
> to more clearly separate the project's design philosophy (what we
> value), development policy (how we work), and dev environment rules
> (concrete operational conventions). See `CLAUDE.md` for details.


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
  > ⚠️ **Watch out for deploy-directory drift (discovered 2026-07-28)**:
  > confirm the VPS deploy directory really is a `git clone` of
  > `aon-co-jp/open-easy-web`. A past incident had files manually placed,
  > uncommitted, inside a checkout of a different meta-repo
  > (`aon-co-jp/RUNO`) — GitHub-side updates never reached production for
  > a long time, on both the frontend and the backend. Before declaring a
  > deploy done, diff `ls <deploy dir>/src` against the local `src/` /
  > `server/src/` and confirm the module count matches. See `PORTING.md`
  > for the full incident writeup and prevention steps.
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
- **Distributed sync clone DB + disaster recovery (added 2026-07-25)**:
  Step 5 of the First-time Setup Guide lets you register/remove one or
  more "distributed sync targets" (other VPS instances this file
  server's site data continuously replicates to, over SFTP) and,
  optionally, a disaster fallback destination (email or Google Drive,
  with automatic compression before upload / decompression on
  restore). This reuses the sister repository `open-raid-z`'s
  disconnect-tolerant journal, disaster-recovery orchestration, and
  offsite-backup targets (`open_raid_z_core`) as-is — it is not
  reimplemented here. **Configuration is optional** — skipping it does
  not block normal use of the file server. The admin API
  (`POST`/`GET`/`DELETE /admin/dist-sync/targets`,
  `POST /admin/dist-sync/disaster-fallback`,
  `POST /admin/dist-sync/first-time-setup`) is disabled unless the
  `OPEN_EASYWEB_DIST_SYNC_ADMIN_TOKEN` environment variable is set.
  **Honest disclosure**: GPU/NPU (DirectX) acceleration for
  compression always safely falls back to the CPU implementation as of
  2026-07, same as `open_raid_z_core::accel` (not claimed as
  implemented when it isn't). No integration test against a real VPS
  or a real cloud account has been run — only local mocks (unreachable
  addresses, no real SMTP/cloud connection). See the 2026-07-25 HANDOFF
  entry in `CLAUDE.md` for details.
  **2026-07-25 follow-up — real file writes now actually replicate**:
  the gap above (registry/admin-API/wizard scaffolding only, no real
  write-path wiring) has been closed. Uploading a site file
  (`POST /api/sites/:name/upload`) now triggers a non-blocking
  replication to every registered distributed-sync target — the HTTP
  response to the uploading user never waits on replication, so a
  slow or unreachable target does not delay the upload. When no sync
  targets are registered, no replication work is scheduled at all
  (zero behavior change from before this change). Wiring the disaster
  fallback (email/Google Drive) destinations and integrating with the
  disconnect-tolerant journal remain follow-up work. See the same-day
  follow-up HANDOFF entry in `CLAUDE.md` for details.
- **Easy free-domain wizard (DuckDNS, up to 20 domains, added 2026-07-23)**:
  for non-static-IP DDNS environments, a single-screen wizard drives
  `open-web-server`'s new admin API end to end: (a) an external link to
  create a DuckDNS (duckdns.org) account, (b) a registered-domain list
  (remaining-capacity count + per-domain remove button), (c) a
  subdomain-name + token form with an "Add & verify" button that
  registers the domain and checks connectivity immediately, (d) once
  verified, an example SFTP connection command (with a dropdown to pick
  which of several registered domains to use). Up to 20 domains can be
  registered and auto-renewed per instance. **Honest disclosure**:
  creating the DuckDNS account itself (OAuth login) is not automated.
  If `open-web-server` is on a different origin, set
  `OPEN_WEB_SERVER_CORS_ALLOWED_ORIGINS` on the `open-web-server` side
  (added 2026-07-23) to allow the call (see [PORTING.md](PORTING.md),
  Japanese only).

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

## Server-side (open-easy-web-server) install (added 2026-07-23)

The backend API binary is distributed separately via `install.sh` (Linux,
systemd) / `install.ps1` (Windows) / GitHub Releases (built by
`.github/workflows/release.yml` on tag push). This package does not
include the WASM frontend — build it separately via the steps above.

```
curl -fsSL https://github.com/aon-co-jp/open-easy-web/releases/latest/download/open-easy-web-server-linux-x86_64.tar.gz | tar xz
sudo ./install.sh
```

### Data portability on install/uninstall (added 2026-07-29)

`install.sh`/`install.ps1` now ask whether to restore existing data at
startup — from a local tar.gz, a GitHub repository, or an rclone remote
(Google Drive and other clouds). The matching `uninstall.sh`/`uninstall.ps1`
(new) ask whether to back up data the same three ways before removal.
Implemented in `scripts/data-portability.sh` (`.ps1`). This project never
handles Google Drive OAuth itself (same policy as elsewhere in this
ecosystem) — it relies on an `rclone` remote you configure yourself ahead
of time. Whether a GitHub backup repo is public or private is entirely up
to how you created that repo.

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
