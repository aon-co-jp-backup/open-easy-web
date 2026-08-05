# open-easy-web Kendi Sunucunda Barındırma SSS (Hesap Ayarları ve 2FA)

📖 Diğer diller: [日本語](manual-JAPAN.md) / [English](manual-ENGLISH.md) /
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

## S1. Bunu indirip kendi VPS'imde, bilgisayarımda, telefonumda veya tabletimde çalıştırırsam, kendi e-posta adresimi ve telefon numaramı kaydedebilir miyim?

**Evet, edebilirsiniz.** Tarayıcıda self-servis kayıt formu bulunmamaktadır (güvenlik nedeniyle herkese açık kayıt 2026-07-15 tarihinde kasıtlı olarak devre dışı bırakılmıştır). Bunun yerine, sunucu başlatılırken **kendi** e-posta adresinizi ve telefon numaranızı tek giriş hesabı olarak **ortam değişkenleri** aracılığıyla ayarlarsınız.

| Ortam değişkeni | Zorunlu/İsteğe bağlı | Anlamı |
|---|---|---|
| `OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL` | Zorunlu | Kendi e-posta adresiniz |
| `OPEN_EASYWEB_FIXED_ACCOUNT_PHONE` | İsteğe bağlı | Kendi telefon numaranız |
| `OPEN_EASYWEB_FIXED_ACCOUNT_BACKUP_EMAIL` | İsteğe bağlı | Yedek bir e-posta adresi |

Telefon numarası ayarlamazsanız, yedek e-posta gereklidir (ikisinden en az biri ayarlanmalıdır).

**Platforma göre nasıl yapılandırılır:**
- **Windows / Linux (VPS vb.)**: kurulum sırasında ortam değişkeni olarak, veya systemd servis dosyasında ayarlayın.
- **Android**: uygulama içindeki "Sabit Hesap Ayarları" ekranında e-posta adresinizi girin (bu ayarlanmadıysa uygulama başlamayı reddeder — bilinçli bir güvenlik önlemidir).

Özetle: kendi kendine barındırılan örneğiniz, üretim örneğiyle (easy-web.tokyo) tamamen aynı mekanizmayı kullanır; bu da kendi sahibinin adresiyle çalışır.

## S2. Sadece basit bir cep telefonum varsa (akıllı telefon değil), iki faktörlü kimlik doğrulamayı (2FA) bilgisayarımda onaylayabilir miyim?

**Evet, onaylayabilirsiniz.** 2FA (kimlik doğrulama uygulaması TOTP) kurulum ekranı, akıllı telefon kamerasıyla taranacak bir QR kodu görüntüsü göstermez — doğrudan **düz metin gizli anahtar dizesini** gösterir.

Bu dize, gizli anahtarı manuel olarak girmeye izin veren herhangi bir TOTP uygulamasıyla çalışır — sadece akıllı telefon kimlik doğrulayıcılarıyla sınırlı değildir. Sadece basit bir telefonunuz varsa, iki seçeneğiniz vardır:

1. Bunun yerine **e-posta OTP** kullanın (basit telefonunuz operatör e-postası alabiliyorsa en basit seçenektir).
2. 2FA kurulumu sırasında gösterilen "gizli anahtarı" bir **bilgisayar kimlik doğrulama uygulamasına** (örneğin WinAuth veya bir tarayıcı uzantısı) manuel olarak girin, ardından giriş yaparken bilgisayar ekranınızda görüntülenen 6 haneli kodu okuyun.

Her iki yöntem de özel bir yapılandırma gerektirmeden hemen çalışır.
