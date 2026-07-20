# open-easy-web

**"كوساناجي الثاني" — التشغيل عبر عنوان IP بعد رفع التطبيق، مع تطبيق
سهل لتسجيل النطاق و HTTPS التلقائي (Rust ← WebAssembly، بدون اعتماد
على أي إطار عمل)**

مثل حزمة تسريع WordPress "KUSANAGI"، يهدف `open-easy-web` إلى نقلك من
"رفع التطبيق" إلى **التشغيل عبر IP ← تسجيل نطاق مبسّط ← HTTPS تلقائي**
في تدفق واحد. يتضمن شاشة "إدارة المواقع" لتسجيل/تبديل/اختبار عدة
وجهات، ويُنشئ إعدادات vhost أساسية لبروكسي عكسي (Nginx/Apache) لـ
WordPress وPHP+Laravel وPython+FastAPI أو أي خلفية أخرى. **لا يمتلك أي
اتصال بقاعدة بيانات** (خارج النطاق عمدًا).

**الانفصال بتاريخ 2026-07-13 عن `aruaru-web`**: كل ما كان `aruaru-web`
يطوّره ضمن "تسجيل/حذف سهل للنطاقات/النطاقات الفرعية"، و"مراقبة/إصدار/
تجديد HTTPS تلقائي"، و"تشغيل سهل للموقع بعد الرفع" — **كل شيء باستثناء
ميزات تسريع KUSANAGI** — تم نقله إلى هنا. ميزات التسريع (ضغط gzip،
تخزين مؤقت طويل الأمد للأصول الثابتة، ضبط مخازن FastCGI المؤقتة، تجميع
keepalive للخادم الأعلى) لم تعد تُولَّد كإعدادات Nginx/Apache؛ بل تم
دمجها كـ **تنفيذات Rust أصلية (وسيط hyper) في `open-runo`/
`poem-cosmo-tauri`**.

📖 لغات أخرى: [日本語](README-Japan.md) / [English](README-English.md) /
[中文](README-Chinese.md) / [한국어](README-Korea.md) / [Español](README-Spain.md) /
[Français](README-France.md) / [Deutsch](README-Germany.md) / [Italiano](README-Italy.md) /
[Русский](README-Russia.md) / [العربية](README-Arabic.md)

---

## ما يعمل حاليًا

- **شاشة إدارة المواقع**: تسجيل/تعديل/حذف عدة وجهات نشر، محفوظة في
  `localStorage`. زر "اختبار الاتصال" لكل بطاقة، التحقق من صحة المنفذ
  (1-65535)، تأكيد قبل الحذف، تصدير/استيراد JSON.
- **التشغيل عبر IP**: `scripts/serve.sh <BIND_IP> <PORT>`.
- **إنشاء vhost + HTTPS تلقائي**: `scripts/gen-vhost.sh
  [--stack=STACK] <DOMAIN> <BIND_IP> [UPSTREAM] [WEBROOT]` لخمسة أنواع:
  `static`، `proxy`، `wordpress`، `laravel`، `fastapi`. **ضبط التسريع
  مستبعد عمدًا هنا** — راجع `open-runo`/`poem-cosmo-tauri`.
- **مراقبة/تجديد HTTPS تلقائي**: `scripts/setup-tls.sh` (Let's Encrypt
  عبر certbot)، ويثبّت `deploy/systemd/install-systemd-units.sh` مؤقتات
  للتجديد (مرتين يوميًا) ومراقبة انتهاء الصلاحية (مرة يوميًا).
- **النشر على VPS**: `scripts/deploy-vps.ps1` (PowerShell).

## ما لا يفعله عمدًا

- **لا يوجد تسريع للويب** (gzip/تخزين مؤقت ثابت/مخازن FastCGI/keepalive
  للخادم الأعلى) — راجع تنفيذات Rust الأصلية في `open-runo`/
  `poem-cosmo-tauri`.
- **لا يوجد أي اتصال بقاعدة بيانات على الإطلاق**.
- لا يوجد مصادقة أو ترقيم صفحات أو إعادة محاولة تلقائية عند الأخطاء.
- لا يوفر تجربة تطبيق أصلي مثل Tauri (فقط WASM يعمل في المتصفح).
- **لا يقوم بشراء نطاق فعلي/تسجيل DNS** (عملية مسجل النطاقات) أو
  التعاقد على VPS — يقوم المستخدم بذلك بنفسه؛ يقوم هذا المستودع فقط
  بأتمتة إنشاء vhost ودورة حياة شهادة TLS لنطاق مسجَّل بالفعل.

## البناء

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.126
cargo build --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir pkg \
  target/wasm32-unknown-unknown/debug/open_easy_web.wasm
python -m http.server 8080
```

> ⚠️ **تحذير بخصوص البناء (بيئات محرك الشبكة)**: إذا كان هذا المستودع على
> محرك مُحمَّل عبر الشبكة (مثل مشاركة SMB)، فإن القراءة/الكتابة المباشرة
> لمخرجات `target/` الخاصة بـ `cargo build` أو مدخلات/مخرجات
> `wasm-bindgen` على ذلك المحرك **قد تُعيد محتوى قديمًا فور الكتابة**
> (عدم اتساق ذاكرة التخزين المؤقت للقراءة، حدث فعليًا في 2026-07-20). إذا
> لم تنعكس إعادة البناء، وجّه مخرجات البناء إلى محرك محلي باستخدام
> `cargo build --target-dir <مجلد-محلي-مؤقت>` ثم شغّل `wasm-bindgen` على
> تلك النسخة المحلية.

## تم التحقق منه في هذا المرور

نجح `cargo check`/`build`/`clippy --target wasm32-unknown-unknown` كلها
بدون تحذيرات. تم التحقق من `gen-vhost.sh` لجميع الأنواع الخمسة. نظرًا
لأن بيئة التطوير هذه هي Windows بدون ملف تنفيذي nginx/apache متاح، **لم
يتم** إجراء التحقق النحوي الفعلي عبر `nginx -t`/`apache2ctl configtest`
في هذا المرور — القوالب هي طرح صارم (إزالة توجيهات فقط) من قوالب
aruaru-web التي تم التحقق منها مسبقًا، دون إضافة أي صياغة جديدة. راجع
`CLAUDE.md` للحصول على حالة التحقق الكاملة والصادقة.

## مشاريع ذات صلة

- **aruaru-web**: https://github.com/aon-co-jp/aruaru-web
- **open-runo**: https://github.com/aon-co-jp/open-runo
- **poem-cosmo-tauri**: https://github.com/aon-co-jp/poem-cosmo-tauri
- **aruaru-db**: https://github.com/aon-co-jp/aruaru-db
- **open-web-server**: https://github.com/aon-co-jp/open-web-server
- **open-raid-z**: https://github.com/aon-co-jp/open-raid-z
- **rs-to-readme**: https://github.com/aon-co-jp/rs-to-readme

## License

Apache-2.0
