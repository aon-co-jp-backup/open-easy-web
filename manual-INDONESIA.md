# open-easy-web FAQ Self-Hosting (Pengaturan Akun & 2FA)

📖 Bahasa lain: [日本語](manual-JAPAN.md) / [English](manual-ENGLISH.md) /
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

## T1. Jika saya mengunduh ini dan menjalankannya di VPS, PC, ponsel, atau tablet saya sendiri, dapatkah saya mendaftarkan alamat email dan nomor telepon saya sendiri?

**Ya, bisa.** Tidak ada formulir pendaftaran mandiri di peramban (pendaftaran publik sengaja dinonaktifkan pada 2026-07-15 karena alasan keamanan). Sebagai gantinya, Anda mengatur alamat email dan nomor telepon **Anda sendiri** sebagai satu-satunya akun login melalui **variabel lingkungan (environment variables)** saat server dimulai.

| Variabel lingkungan | Wajib/Opsional | Arti |
|---|---|---|
| `OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL` | Wajib | Alamat email Anda sendiri |
| `OPEN_EASYWEB_FIXED_ACCOUNT_PHONE` | Opsional | Nomor telepon Anda sendiri |
| `OPEN_EASYWEB_FIXED_ACCOUNT_BACKUP_EMAIL` | Opsional | Alamat email cadangan |

Jika Anda tidak mengatur nomor telepon, email cadangan diperlukan (setidaknya salah satu dari keduanya harus diatur).

**Cara mengonfigurasi per platform:**
- **Windows / Linux (VPS, dll.)**: atur sebagai variabel lingkungan saat instalasi, atau di file layanan systemd.
- **Android**: masukkan alamat email Anda di layar "Pengaturan Akun Tetap" pada aplikasi (aplikasi akan menolak untuk memulai jika ini belum diatur — ini adalah tindakan keamanan yang disengaja).

Singkatnya: instance self-hosted Anda sendiri menggunakan mekanisme yang persis sama dengan instance produksi (easy-web.tokyo), yang juga berjalan dengan alamat milik pemiliknya sendiri.

## T2. Jika saya hanya memiliki ponsel fitur (bukan smartphone), dapatkah saya mengonfirmasi autentikasi dua faktor (2FA) di PC saya?

**Ya, bisa.** Layar pengaturan 2FA (TOTP melalui aplikasi autentikator) tidak menampilkan gambar kode QR untuk dipindai dengan kamera smartphone — melainkan langsung menampilkan **string rahasia dalam bentuk teks biasa**.

String ini berfungsi dengan aplikasi TOTP apa pun yang memungkinkan Anda memasukkan rahasia secara manual — tidak hanya autentikator smartphone. Jika Anda hanya memiliki ponsel fitur, Anda memiliki dua opsi:

1. Gunakan **OTP email** sebagai gantinya (opsi paling sederhana jika ponsel fitur Anda dapat menerima email operator).
2. Masukkan "rahasia" yang ditampilkan selama pengaturan 2FA secara manual ke **aplikasi autentikator PC** (misalnya WinAuth atau ekstensi peramban), lalu baca kode 6 digit yang ditampilkan di layar PC Anda saat masuk.

Kedua metode ini langsung berfungsi tanpa konfigurasi khusus.
