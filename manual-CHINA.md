# open-easy-web 自托管常见问题(账户设置与双重验证)

📖 其他语言: [日本語](manual-JAPAN.md) / [English](manual-ENGLISH.md) /
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

## 问1. 如果我下载后在自己的VPS、电脑、手机或平板上运行,可以注册自己的邮箱地址和手机号码吗?

**可以。** 浏览器上没有自助注册表单(出于安全原因,已于2026-07-15停用公开注册)。取而代之的是,在启动服务器时通过**环境变量**设置您自己的邮箱和手机号码,作为唯一可登录的账户。

| 环境变量 | 必需/可选 | 内容 |
|---|---|---|
| `OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL` | 必需 | 您自己的邮箱地址 |
| `OPEN_EASYWEB_FIXED_ACCOUNT_PHONE` | 可选 | 您自己的手机号码 |
| `OPEN_EASYWEB_FIXED_ACCOUNT_BACKUP_EMAIL` | 可选 | 备用邮箱地址 |

如果不设置手机号码,则必须设置备用邮箱(两者至少设置一个)。

**各平台的设置方法:**
- **Windows / Linux(VPS等)**: 在安装时或systemd服务配置文件中设置环境变量。
- **Android**: 在应用内的"固定账户设置"界面输入邮箱地址(若未设置,应用会拒绝启动——这是刻意的安全设计)。

简而言之: 您自己搭建的实例,使用的机制与生产环境(easy-web.tokyo,同样使用所有者本人的地址运行)完全相同。

## 问2. 如果只有功能机(非智能手机),双重验证(2FA)可以在电脑上确认吗?

**可以。** 双重验证(基于身份验证器应用的TOTP)设置界面**不会**显示需要手机摄像头扫描的二维码图像,而是直接显示**纯文本形式的密钥字符串**。

该字符串适用于任何支持手动输入密钥的TOTP应用——不仅限于手机身份验证器。如果您只有功能机,可以使用以下两种方式之一:

1. 改用**邮箱一次性验证码**(如果您的功能机可以接收运营商邮件,这是最简单的方式)。
2. 将双重验证设置时显示的"密钥"手动输入到**电脑端身份验证器应用**(如WinAuth或浏览器扩展身份验证器)中,登录时查看电脑屏幕上显示的6位数字代码并输入。

以上两种方式均无需额外配置即可直接使用。
