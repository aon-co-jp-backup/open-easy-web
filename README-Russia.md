# open-easy-web

**«Второй KUSANAGI» — запуск по IP-адресу после загрузки приложения, с
простой регистрацией домена и автоматическим HTTPS (Rust →
WebAssembly, без зависимости от фреймворков)**

Подобно набору ускорения для WordPress «KUSANAGI», `open-easy-web`
стремится провести вас от «загрузки приложения» до **запуска по
IP-адресу → упрощённой регистрации домена → автоматического HTTPS** в
едином потоке. Включает экран «управления сайтами» и генерирует базовую
конфигурацию vhost обратного прокси (Nginx/Apache) для WordPress,
PHP+Laravel, Python+FastAPI или любого бэкенда. **Не имеет подключения
к базе данных** (намеренно вне области применения).

**Разделение от 2026-07-13 из `aruaru-web`**: всё, что `aruaru-web`
разрабатывал под «простую регистрацию/удаление доменов/поддоменов»,
«автоматический мониторинг/выпуск/продление HTTPS» и «простую
эксплуатацию сайта после загрузки» — **всё, кроме функций ускорения
KUSANAGI** — перенесено сюда. Функции ускорения (gzip-сжатие,
долгосрочное кэширование статических активов, настройка буферов
FastCGI, пулинг keepalive upstream) больше не генерируются как
конфигурация Nginx/Apache; они консолидированы как **нативные
реализации на Rust (hyper-middleware) в `open-runo`/`poem-cosmo-tauri`**.

📖 Другие языки: [日本語](README-Japan.md) / [English](README-English.md) /
[中文](README-Chinese.md) / [한국어](README-Korea.md) / [Español](README-Spain.md) /
[Français](README-France.md) / [Deutsch](README-Germany.md) / [Italiano](README-Italy.md) /
[Русский](README-Russia.md) / [العربية](README-Arabic.md)

---

## Что работает сейчас

- **Экран управления сайтами**: регистрация/редактирование/удаление
  нескольких целей развёртывания, сохранённых в `localStorage`. Кнопка
  «проверка соединения» на карточке, валидация порта (1-65535),
  подтверждение перед удалением, экспорт/импорт JSON.
- **Запуск по IP-адресу**: `scripts/serve.sh <BIND_IP> <PORT>`.
- **Генерация vhost + автоматический HTTPS**: `scripts/gen-vhost.sh
  [--stack=STACK] <DOMAIN> <BIND_IP> [UPSTREAM] [WEBROOT]` для 5
  стеков: `static`, `proxy`, `wordpress`, `laravel`, `fastapi`.
  **Настройка ускорения здесь намеренно не включена** — см.
  `open-runo`/`poem-cosmo-tauri`.
- **Автоматический мониторинг/продление HTTPS**:
  `scripts/setup-tls.sh` (Let's Encrypt через certbot),
  `deploy/systemd/install-systemd-units.sh` устанавливает таймеры
  продления (2 раза в день) и мониторинга истечения (1 раз в день).
- **Развёртывание на VPS**: `scripts/deploy-vps.ps1` (PowerShell).

## Что намеренно не делает

- **Никакого ускорения веба** (gzip/статическое кэширование/буферы
  FastCGI/keepalive upstream) — см. нативные реализации на Rust в
  `open-runo`/`poem-cosmo-tauri`.
- **Никакого подключения к базе данных**.
- Нет аутентификации, пагинации, автоматического повтора при ошибках.
- Нет нативного опыта приложения как в Tauri (только WASM в браузере).
- **Не выполняет реальную покупку домена/регистрацию DNS** (операция
  регистратора) или заключение договора VPS — это делает пользователь
  самостоятельно; репозиторий автоматизирует только генерацию vhost и
  жизненный цикл TLS для уже зарегистрированного домена.

## Сборка

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.126
cargo build --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir pkg \
  target/wasm32-unknown-unknown/debug/open_easy_web.wasm
python -m http.server 8080
```

## Проверено в этом проходе

`cargo check`/`build`/`clippy --target wasm32-unknown-unknown` — все
успешны, без предупреждений. `gen-vhost.sh` проверен для всех 5
стеков. Поскольку эта среда разработки — Windows без доступного
бинарного файла nginx/apache, реальная синтаксическая проверка через
`nginx -t`/`apache2ctl configtest` в этом проходе **не** выполнялась —
шаблоны являются строгим вычитанием (только удалённые директивы) из уже
проверенных шаблонов aruaru-web, без добавления нового синтаксиса. См.
`CLAUDE.md` для полного честного статуса проверки.

## Связанные проекты

- **aruaru-web**: https://github.com/aon-co-jp/aruaru-web
- **open-runo**: https://github.com/aon-co-jp/open-runo
- **poem-cosmo-tauri**: https://github.com/aon-co-jp/poem-cosmo-tauri
- **aruaru-db**: https://github.com/aon-co-jp/aruaru-db
- **open-web-server**: https://github.com/aon-co-jp/open-web-server
- **open-raid-z**: https://github.com/aon-co-jp/open-raid-z
- **rs-to-readme**: https://github.com/aon-co-jp/rs-to-readme

## License

Apache-2.0
