# open-easy-web FAQ de autohospedaje (configuración de cuenta y 2FA)

📖 Otros idiomas: [日本語](manual-JAPAN.md) / [English](manual-ENGLISH.md) /
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

## P1. Si descargo esto y lo ejecuto en mi propio VPS, PC, teléfono o tableta, ¿puedo registrar mi propia dirección de correo electrónico y número de teléfono?

**Sí.** No hay un formulario de registro autoservicio en el navegador (el registro público se deshabilitó intencionalmente el 2026-07-15 por razones de seguridad). En su lugar, configura **su propia** dirección de correo y número de teléfono como la única cuenta de acceso mediante **variables de entorno** al iniciar el servidor.

| Variable de entorno | Obligatorio/Opcional | Significado |
|---|---|---|
| `OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL` | Obligatorio | Su propia dirección de correo |
| `OPEN_EASYWEB_FIXED_ACCOUNT_PHONE` | Opcional | Su propio número de teléfono |
| `OPEN_EASYWEB_FIXED_ACCOUNT_BACKUP_EMAIL` | Opcional | Un correo de respaldo |

Si no configura un número de teléfono, se requiere un correo de respaldo (al menos uno de los dos debe estar configurado).

**Cómo configurarlo según la plataforma:**
- **Windows / Linux (VPS, etc.)**: configúrelo como variable de entorno al instalar, o en el archivo de servicio systemd.
- **Android**: introduzca su dirección de correo en la pantalla "Configuración de cuenta fija" de la app (la app se niega a iniciar si esto no está configurado, una medida de seguridad deliberada).

En resumen: su propia instancia autohospedada usa exactamente el mismo mecanismo que la instancia de producción (easy-web.tokyo), que a su vez funciona con la propia dirección del propietario.

## P2. Si solo tengo un teléfono básico (no inteligente), ¿puedo confirmar la autenticación de dos factores (2FA) en mi PC?

**Sí.** La pantalla de configuración de 2FA (TOTP mediante app autenticadora) no muestra una imagen de código QR para escanear con la cámara del teléfono — muestra directamente la **cadena secreta en texto plano**.

Esa cadena funciona con cualquier app TOTP que permita introducir un secreto manualmente, no solo con autenticadores de teléfono. Si solo tiene un teléfono básico, tiene dos opciones:

1. Use **OTP por correo electrónico** en su lugar (la opción más sencilla si su teléfono básico puede recibir correo del operador).
2. Introduzca el "secreto" mostrado durante la configuración de 2FA en una **app autenticadora de PC** (por ejemplo, WinAuth o una extensión de navegador autenticadora), y luego lea el código de 6 dígitos en la pantalla de su PC al iniciar sesión.

Ambas opciones funcionan de inmediato sin configuración especial.
