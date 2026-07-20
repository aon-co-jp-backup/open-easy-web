# open-easy-web

**"El segundo KUSANAGI" — arranca por dirección IP tras subir la app, y
aplica fácilmente registro de dominio + HTTPS automático (Rust →
WebAssembly, sin dependencia de frameworks)**

Al igual que el kit de aceleración para WordPress "KUSANAGI",
`open-easy-web` busca llevarte de "subir la app" a **arrancar por IP →
registro de dominio simplificado → HTTPS automático** en un solo flujo.
Incluye una pantalla de "gestión de sitios" para registrar/cambiar/
probar múltiples destinos, y genera configuración básica de vhost de
proxy inverso (Nginx/Apache) para WordPress, PHP+Laravel, Python+FastAPI
o cualquier backend. **No tiene conectividad a base de datos**
(deliberadamente fuera de alcance).

**División del 2026-07-13 desde `aruaru-web`**: todo lo que
`aruaru-web` desarrollaba bajo "registro/eliminación fácil de dominios/
subdominios", "monitor/emisión/renovación automática de HTTPS" y
"operación fácil del sitio tras la subida" —**todo excepto las
funciones de aceleración de KUSANAGI**— se ha movido aquí. Las
funciones de aceleración (compresión gzip, caché de larga duración de
activos estáticos, ajuste de buffers FastCGI, pooling keepalive de
upstream) ya no se generan como configuración Nginx/Apache; se han
consolidado como **implementaciones nativas en Rust (middleware hyper)
en `open-runo`/`poem-cosmo-tauri`**.

📖 Otros idiomas: [日本語](README-Japan.md) / [English](README-English.md) /
[中文](README-Chinese.md) / [한국어](README-Korea.md) / [Español](README-Spain.md) /
[Français](README-France.md) / [Deutsch](README-Germany.md) / [Italiano](README-Italy.md) /
[Русский](README-Russia.md) / [العربية](README-Arabic.md)

---

## Qué funciona hoy

- **Pantalla de gestión de sitios**: registra/edita/elimina múltiples
  destinos de despliegue, guardados en `localStorage`. Botón de
  "prueba de conexión" por tarjeta, validación de puerto (1-65535),
  confirmación antes de eliminar, exportación/importación JSON.
- **Arranque por IP**: `scripts/serve.sh <BIND_IP> <PORT>`.
- **Generación de vhost + HTTPS automático**:
  `scripts/gen-vhost.sh [--stack=STACK] <DOMAIN> <BIND_IP> [UPSTREAM]
  [WEBROOT]` para 5 stacks: `static`, `proxy`, `wordpress`, `laravel`,
  `fastapi`. **El ajuste de aceleración se excluye deliberadamente
  aquí** — ver `open-runo`/`poem-cosmo-tauri`.
- **Monitor/renovación automática de HTTPS**: `scripts/setup-tls.sh`
  (Let's Encrypt vía certbot), `deploy/systemd/
  install-systemd-units.sh` instala los timers de renovación (2x/día)
  y monitoreo de expiración (1x/día).
- **Despliegue en VPS**: `scripts/deploy-vps.ps1` (PowerShell).

## Qué deliberadamente no hace

- **Sin aceleración web** (gzip/caché estático/buffers FastCGI/
  keepalive upstream) — ver las implementaciones nativas en Rust de
  `open-runo`/`poem-cosmo-tauri`.
- **Sin conectividad a base de datos alguna**.
- Sin autenticación, paginación ni reintento automático de errores.
- Sin experiencia de app nativa tipo Tauri (solo WASM en navegador).
- **No realiza compra real de dominio/registro DNS** ni contratación de
  VPS — solo automatiza la generación de vhost y el ciclo de vida TLS
  para un dominio ya registrado.

## Compilación

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.126
cargo build --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir pkg \
  target/wasm32-unknown-unknown/debug/open_easy_web.wasm
python -m http.server 8080
```

> ⚠️ **Advertencia de compilación (entornos de unidad de red)**: si este
> repositorio está en una unidad montada por red (p. ej. SMB), leer/
> escribir la salida `target/` de `cargo build` o la entrada/salida de
> `wasm-bindgen` directamente en esa unidad **puede devolver contenido
> obsoleto justo después de escribir** (incoherencia de caché de lectura,
> ocurrió realmente el 2026-07-20). Si una recompilación no parece
> aplicarse, apunta la salida a una unidad local con `cargo build
> --target-dir <directorio-temporal-local>` y ejecuta `wasm-bindgen`
> sobre esa copia local.

## Verificado en este pase

`cargo check`/`build`/`clippy --target wasm32-unknown-unknown` exitosos
sin advertencias. `gen-vhost.sh` verificado en los 5 stacks. Debido a
que este entorno de desarrollo es Windows sin binario nginx/apache
disponible, **no** se ejecutó `nginx -t`/`apache2ctl configtest` real en
este pase — las plantillas son una resta estricta (solo directivas
eliminadas) de las plantillas ya verificadas de aruaru-web, sin
sintaxis nueva añadida. Ver `CLAUDE.md` para el estado honesto completo.

## Proyectos relacionados

- **aruaru-web**: https://github.com/aon-co-jp/aruaru-web
- **open-runo**: https://github.com/aon-co-jp/open-runo
- **poem-cosmo-tauri**: https://github.com/aon-co-jp/poem-cosmo-tauri
- **aruaru-db**: https://github.com/aon-co-jp/aruaru-db
- **open-web-server**: https://github.com/aon-co-jp/open-web-server
- **open-raid-z**: https://github.com/aon-co-jp/open-raid-z
- **rs-to-readme**: https://github.com/aon-co-jp/rs-to-readme

## License

Apache-2.0
