# open-easy-web

**« Le second KUSANAGI » — démarrez par adresse IP après l'upload, et
appliquez facilement l'enregistrement de domaine + HTTPS automatique
(Rust → WebAssembly, sans dépendance à un framework)**

Comme le kit d'accélération WordPress « KUSANAGI », `open-easy-web` vise
à vous emmener de « uploader l'app » à **démarrage par IP → domaine
simplifié → HTTPS automatique** en un seul flux. Comprend un écran de
« gestion de sites » et génère une config vhost de reverse-proxy basique
(Nginx/Apache) pour WordPress, PHP+Laravel, Python+FastAPI ou tout
backend. **Aucune connectivité base de données** (hors périmètre
volontairement).

**Scission du 2026-07-13 depuis `aruaru-web`**: tout ce que
`aruaru-web` développait sous « enregistrement/suppression facile de
domaines/sous-domaines », « surveillance/émission/renouvellement HTTPS
automatique » et « exploitation facile du site après upload » —**tout
sauf les fonctions d'accélération de KUSANAGI**— a été déplacé ici. Les
fonctions d'accélération (compression gzip, cache longue durée des
assets statiques, ajustement des buffers FastCGI, pooling keepalive
upstream) sont désormais des **implémentations Rust natives (middleware
hyper) dans `open-runo`/`poem-cosmo-tauri`**.

📖 Autres langues : [日本語](README-Japan.md) / [English](README-English.md) /
[中文](README-Chinese.md) / [한국어](README-Korea.md) / [Español](README-Spain.md) /
[Français](README-France.md) / [Deutsch](README-Germany.md) / [Italiano](README-Italy.md) /
[Русский](README-Russia.md) / [العربية](README-Arabic.md)

---

## Ce qui fonctionne aujourd'hui

- **Écran de gestion de sites**: enregistrer/modifier/supprimer
  plusieurs cibles de déploiement, sauvegardées dans `localStorage`.
  Bouton « test de connexion » par carte, validation de port
  (1-65535), confirmation avant suppression, export/import JSON.
- **Démarrage par IP**: `scripts/serve.sh <BIND_IP> <PORT>`.
- **Génération de vhost + HTTPS automatique**: `scripts/gen-vhost.sh
  [--stack=STACK] <DOMAIN> <BIND_IP> [UPSTREAM] [WEBROOT]` pour 5
  stacks : `static`, `proxy`, `wordpress`, `laravel`, `fastapi`.
  **Le tuning d'accélération est volontairement exclu ici** — voir
  `open-runo`/`poem-cosmo-tauri`.
- **Surveillance/renouvellement HTTPS automatique**:
  `scripts/setup-tls.sh` (Let's Encrypt via certbot),
  `deploy/systemd/install-systemd-units.sh` installe les timers de
  renouvellement (2x/jour) et de surveillance d'expiration (1x/jour).
- **Déploiement VPS**: `scripts/deploy-vps.ps1` (PowerShell).

## Ce qu'il ne fait volontairement pas

- **Pas d'accélération web** (gzip/cache statique/buffers FastCGI/
  keepalive upstream) — voir les implémentations Rust natives de
  `open-runo`/`poem-cosmo-tauri`.
- **Aucune connectivité base de données**.
- Pas d'authentification, pagination, ni retry automatique d'erreur.
- Pas d'expérience app native façon Tauri (WASM navigateur uniquement).
- **N'effectue pas l'achat réel de domaine/l'enregistrement DNS** ni la
  location de VPS — l'utilisateur s'en charge; ce dépôt automatise
  uniquement la génération de vhost et le cycle de vie TLS pour un
  domaine déjà enregistré.

## Compilation

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.126
cargo build --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir pkg \
  target/wasm32-unknown-unknown/debug/open_easy_web.wasm
python -m http.server 8080
```

> ⚠️ **Avertissement de build (environnements sur lecteur réseau)** : si
> ce dépôt se trouve sur un lecteur monté en réseau (ex. partage SMB),
> lire/écrire la sortie `target/` de `cargo build` ou les entrées/sorties
> de `wasm-bindgen` directement sur ce lecteur **peut renvoyer un contenu
> obsolète juste après une écriture** (incohérence de cache de lecture,
> réellement rencontrée le 2026-07-20). Si une recompilation ne semble pas
> prise en compte, redirigez la sortie du build vers un lecteur local
> avec `cargo build --target-dir <répertoire-temporaire-local>` et
> exécutez `wasm-bindgen` sur cette copie locale.

## Vérifié dans ce passage

`cargo check`/`build`/`clippy --target wasm32-unknown-unknown` réussis
sans avertissement. `gen-vhost.sh` vérifié sur les 5 stacks. Cet
environnement de dev étant Windows sans binaire nginx/apache
disponible, la vérification syntaxique réelle via `nginx -t`/
`apache2ctl configtest` n'a **pas** été effectuée dans ce passage — les
templates ne sont qu'une soustraction stricte (directives supprimées
uniquement) des templates déjà vérifiés d'aruaru-web, sans nouvelle
syntaxe ajoutée. Voir `CLAUDE.md` pour l'état de vérification complet.

## Projets liés

- **aruaru-web**: https://github.com/aon-co-jp/aruaru-web
- **open-runo**: https://github.com/aon-co-jp/open-runo
- **poem-cosmo-tauri**: https://github.com/aon-co-jp/poem-cosmo-tauri
- **aruaru-db**: https://github.com/aon-co-jp/aruaru-db
- **open-web-server**: https://github.com/aon-co-jp/open-web-server
- **open-raid-z**: https://github.com/aon-co-jp/open-raid-z
- **rs-to-readme**: https://github.com/aon-co-jp/rs-to-readme

## License

Apache-2.0
