# open-easy-web 셀프호스팅 FAQ(계정 설정 및 2단계 인증)

📖 다른 언어: [日本語](MANUAL.md) / [English](MANUAL-English.md) /
[中文](MANUAL-Chinese.md) / [한국어](MANUAL-Korea.md) /
[Español](MANUAL-Spain.md) / [Français](MANUAL-France.md) /
[Deutsch](MANUAL-Germany.md) / [Italiano](MANUAL-Italy.md) /
[Русский](MANUAL-Russia.md) / [العربية](MANUAL-Arabic.md) /
[Português](MANUAL-Portugal.md) / [Nederlands](MANUAL-Netherlands.md) /
[Türkçe](MANUAL-Turkey.md) / [Polski](MANUAL-Poland.md) /
[Tiếng Việt](MANUAL-Vietnam.md) / [ไทย](MANUAL-Thailand.md) /
[Bahasa Indonesia](MANUAL-Indonesia.md) / [हिन्दी](MANUAL-India.md)

---

## Q1. 다운로드하여 자신의 VPS, PC, 스마트폰, 태블릿에서 운영할 경우, 자신의 이메일 주소와 휴대폰 번호를 등록할 수 있나요?

**네, 가능합니다.** 브라우저상의 자체 회원가입 양식은 없습니다(보안상의 이유로 2026-07-15에 공개 등록이 폐지되었습니다). 대신 **서버 시작 시 환경 변수**를 통해 본인의 이메일 주소와 휴대폰 번호를 유일하게 로그인 가능한 계정으로 설정하는 방식입니다.

| 환경 변수 | 필수/선택 | 내용 |
|---|---|---|
| `OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL` | 필수 | 본인의 이메일 주소 |
| `OPEN_EASYWEB_FIXED_ACCOUNT_PHONE` | 선택 | 본인의 휴대폰 번호 |
| `OPEN_EASYWEB_FIXED_ACCOUNT_BACKUP_EMAIL` | 선택 | 예비 이메일 주소 |

휴대폰 번호를 설정하지 않는 경우, 예비 이메일 주소 설정이 필수입니다(둘 중 하나는 반드시 설정해야 합니다).

**플랫폼별 설정 방법:**
- **Windows / Linux(VPS 등)**: 설치 시 또는 systemd 서비스 설정 파일에 환경 변수로 기재합니다.
- **Android**: 앱 내 "고정 계정 설정" 화면에서 이메일 주소를 입력합니다(미설정 시 앱이 시작을 거부하는 안전 설계입니다).

즉, 프로덕션 환경(easy-web.tokyo)이 소유자 본인의 주소로 운영되는 것과 완전히 동일한 방식을, 직접 다운로드한 환경에서도 그대로 사용할 수 있습니다.

## Q2. 피처폰(일반 휴대폰)만 있는 경우, 2단계 인증(2FA)을 PC에서 확인할 수 있나요?

**네, 가능합니다.** 2FA(인증 앱 기반 TOTP) 설정 화면은 스마트폰 카메라로 스캔하는 QR 코드 이미지를 표시하지 않고, **텍스트 형태의 비밀 키 문자열**을 그대로 표시하는 방식입니다.

이 문자열은 비밀 키를 수동으로 입력할 수 있는 모든 TOTP 앱에서 사용할 수 있습니다——스마트폰 인증 앱에 국한되지 않습니다. 피처폰만 있는 경우 다음 두 가지 방법 중 하나를 사용할 수 있습니다.

1. **이메일 OTP**를 사용합니다(피처폰으로 통신사 이메일을 수신할 수 있다면 가장 간단한 방법입니다).
2. 2FA 설정 시 표시되는 "비밀 키"를 **PC용 인증 앱**(WinAuth 또는 브라우저 확장 인증 앱 등)에 수동으로 입력하고, 로그인 시 PC 화면에 표시되는 6자리 코드를 확인하여 입력합니다.

두 방법 모두 별도의 설정 없이 표준 기능만으로 사용할 수 있습니다.
