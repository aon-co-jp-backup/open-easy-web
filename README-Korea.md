# open-easy-web

**"제2의 KUSANAGI" — 앱 업로드 후 IP 주소로 실행하고 도메인 등록과
HTTPS 자동 적용을 간편하게(Rust → WebAssembly, 프레임워크 비의존)**

WordPress 고속화 서버 구축 키트 "KUSANAGI"처럼, 앱을 업로드한 후
**IP 주소로 실행 → 도메인 등록 간소화 → HTTPS 자동화**까지 한 번에
처리하는 것을 목표로 합니다. 여러 사이트의 접속 정보를 등록·전환·
연결 테스트할 수 있는 "사이트 관리" 화면과, WordPress·PHP+Laravel·
Python+FastAPI 등 임의의 백엔드 스택을 위한 기본 리버스 프록시 vhost
설정(Nginx/Apache) 자동 생성 기능을 제공합니다. **데이터베이스 연결
기능은 없습니다**(의도적으로 범위 밖).

**2026-07-13, `aruaru-web`에서 분리**: `aruaru-web`이 개발하던
"손쉬운 도메인/서브도메인 등록·삭제", "HTTPS 자동 모니터링/발급/
갱신", "업로드 후 손쉬운 사이트 운영" 기능——**KUSANAGI의 웹 고속화
기능을 제외한 전부**——이 이 저장소로 이관되었습니다. 고속화 기능
(gzip 압축, 정적 자산 장기 캐싱, FastCGI 버퍼 조정, upstream
keepalive 풀링)은 Nginx/Apache 설정 생성 방식이 아니라 **`open-runo`/
`poem-cosmo-tauri` 측의 네이티브 Rust(hyper 미들웨어) 구현**으로
통합되었습니다.

📖 다른 언어: [日本語](README-Japan.md) / [English](README-English.md) /
[中文](README-Chinese.md) / [한국어](README-Korea.md) / [Español](README-Spain.md) /
[Français](README-France.md) / [Deutsch](README-Germany.md) / [Italiano](README-Italy.md) /
[Русский](README-Russia.md) / [العربية](README-Arabic.md)

---

## 현재 가능한 것

- **사이트 관리 화면**: open-easy-web 자신·WordPress·Laravel·FastAPI 등
  임의의 배포 대상(이름/용도/프로토콜/호스트/포트/경로/백엔드 스택)을
  여러 개 등록·수정·삭제, `localStorage`에 저장. 카드별 "연결 테스트"
  버튼(`fetch(url, {mode:'no-cors'})` 기반 단순 도달성 확인), 포트
  번호 유효성 검사(1~65535), 삭제 전 확인 대화상자, 등록된 사이트
  목록의 JSON 내보내기/가져오기.
- **IP 주소로 실행**: `scripts/serve.sh <BIND_IP> <PORT>`.
- **vhost 생성 + HTTPS 자동 설정**: `scripts/gen-vhost.sh
  [--stack=STACK] <DOMAIN> <BIND_IP> [UPSTREAM] [WEBROOT]`로 `static`·
  `proxy`(범용 리버스 프록시)·`wordpress`·`laravel`·`fastapi` 5개
  스택의 Nginx/Apache vhost(HTTP→HTTPS 리다이렉트 포함)를 생성.
  **고속화 튜닝은 여기 포함되지 않음**——`open-runo`/`poem-cosmo-tauri`
  참조.
- **HTTPS 자동 모니터링/갱신**: `scripts/setup-tls.sh`(certbot로 Let's
  Encrypt 인증서 발급), `deploy/systemd/install-systemd-units.sh`로
  `easyweb-tls-renew.timer`(하루 2회 자동 갱신)·
  `easyweb-tls-monitor.timer`(하루 1회 만료 모니터링) 활성화.
- **VPS 배포**: `scripts/deploy-vps.ps1`(Windows PowerShell)로 빌드→
  업로드→실행 자동화.

## 현재 할 수 없는 것

- **웹 고속화 기능 없음**(gzip/정적 캐싱/FastCGI 버퍼 조정/upstream
  keepalive 풀링)——`open-runo`/`poem-cosmo-tauri`의 네이티브 Rust
  구현 참조.
- **데이터베이스 연결 기능 전혀 없음**.
- 인증·페이지네이션·오류 시 자동 재시도 미구현.
- Tauri와 같은 네이티브 앱 경험 미제공(브라우저 실행 WASM만).
- **실제 도메인 구매/DNS 레코드 등록(등록기관 작업)은 이 저장소에서
  수행하지 않음**——이는 사용자가 직접 수행하며, 이 저장소는 이미
  등록된 도메인에 대한 vhost 생성과 TLS 인증서 수명 주기 관리만
  자동화합니다.

## 빌드

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.126
cargo build --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir pkg \
  target/wasm32-unknown-unknown/debug/open_easy_web.wasm
python -m http.server 8080
```

## 이번 패스 검증 내용

`cargo check`/`build`/`clippy --target wasm32-unknown-unknown` 모두
경고 0건으로 성공. `gen-vhost.sh`는 5개 스택 전부에서 플레이스홀더
치환이 올바름을 확인. 이 개발 환경은 Windows이며 nginx/apache
바이너리가 없어 `nginx -t`/`apache2ctl configtest`를 통한 실제 구문
검증은 **이번 패스에서 수행하지 않음**——템플릿은 이미 검증된
aruaru-web 템플릿에서 지시어를 "제거만" 한 차분이며 새 구문을 추가하지
않았습니다. 자세한 내용은 `CLAUDE.md` 참조.

## 관련 프로젝트

- **aruaru-web**(분리 출처): https://github.com/aon-co-jp/aruaru-web
- **open-runo**: https://github.com/aon-co-jp/open-runo
- **poem-cosmo-tauri**: https://github.com/aon-co-jp/poem-cosmo-tauri
- **aruaru-db**: https://github.com/aon-co-jp/aruaru-db
- **open-web-server**: https://github.com/aon-co-jp/open-web-server
- **open-raid-z**(개발 규칙 정본): https://github.com/aon-co-jp/open-raid-z
- **rs-to-readme**: https://github.com/aon-co-jp/rs-to-readme

## License

Apache-2.0
