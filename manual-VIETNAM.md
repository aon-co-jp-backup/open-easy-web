# open-easy-web Câu hỏi thường gặp về tự lưu trữ (thiết lập tài khoản & 2FA)

📖 Ngôn ngữ khác: [日本語](manual-JAPAN.md) / [English](manual-ENGLISH.md) /
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

## H1. Nếu tôi tải xuống và chạy trên VPS, PC, điện thoại hoặc máy tính bảng của riêng mình, tôi có thể đăng ký địa chỉ email và số điện thoại của riêng mình không?

**Có, bạn có thể.** Không có biểu mẫu đăng ký tự phục vụ trên trình duyệt (việc đăng ký công khai đã bị vô hiệu hóa có chủ đích vào ngày 2026-07-15 vì lý do bảo mật). Thay vào đó, bạn thiết lập địa chỉ email và số điện thoại **của riêng mình** làm tài khoản đăng nhập duy nhất thông qua **biến môi trường** khi khởi động máy chủ.

| Biến môi trường | Bắt buộc/Tùy chọn | Ý nghĩa |
|---|---|---|
| `OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL` | Bắt buộc | Địa chỉ email của riêng bạn |
| `OPEN_EASYWEB_FIXED_ACCOUNT_PHONE` | Tùy chọn | Số điện thoại của riêng bạn |
| `OPEN_EASYWEB_FIXED_ACCOUNT_BACKUP_EMAIL` | Tùy chọn | Một địa chỉ email dự phòng |

Nếu bạn không thiết lập số điện thoại, cần phải có email dự phòng (ít nhất một trong hai phải được thiết lập).

**Cách cấu hình theo từng nền tảng:**
- **Windows / Linux (VPS, v.v.)**: thiết lập dưới dạng biến môi trường khi cài đặt, hoặc trong tệp dịch vụ systemd.
- **Android**: nhập địa chỉ email của bạn vào màn hình "Thiết lập tài khoản cố định" trong ứng dụng (ứng dụng sẽ từ chối khởi động nếu chưa thiết lập — đây là biện pháp bảo mật có chủ đích).

Tóm lại: phiên bản tự lưu trữ của riêng bạn sử dụng chính xác cùng một cơ chế với phiên bản sản xuất (easy-web.tokyo), vốn cũng chạy bằng địa chỉ của chính chủ sở hữu.

## H2. Nếu tôi chỉ có điện thoại phổ thông (không phải smartphone), tôi có thể xác nhận xác thực hai yếu tố (2FA) trên PC của mình không?

**Có, bạn có thể.** Màn hình thiết lập 2FA (TOTP qua ứng dụng xác thực) không hiển thị hình ảnh mã QR để quét bằng camera smartphone — nó hiển thị trực tiếp **chuỗi bí mật dưới dạng văn bản thuần túy**.

Chuỗi này hoạt động với bất kỳ ứng dụng TOTP nào cho phép nhập thủ công một bí mật — không chỉ với các ứng dụng xác thực trên smartphone. Nếu bạn chỉ có điện thoại phổ thông, bạn có hai lựa chọn:

1. Sử dụng **OTP qua email** thay thế (lựa chọn đơn giản nhất nếu điện thoại phổ thông của bạn có thể nhận email từ nhà mạng).
2. Nhập thủ công "bí mật" được hiển thị trong quá trình thiết lập 2FA vào **ứng dụng xác thực trên PC** (ví dụ: WinAuth hoặc tiện ích mở rộng trình duyệt), sau đó đọc mã 6 chữ số hiển thị trên màn hình PC khi đăng nhập.

Cả hai cách đều hoạt động ngay lập tức mà không cần cấu hình đặc biệt.
