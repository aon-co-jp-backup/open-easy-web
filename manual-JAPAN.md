# open-easy-web セルフホストFAQ(アカウント設定・2FA)

📖 他の言語: [日本語](manual-JAPAN.md) / [English](manual-ENGLISH.md) /
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

## Q1. ダウンロードして自分のVPS・PC・スマホ・タブレットで運用する場合、自分のe-mailアドレスと携帯電話番号を登録できますか?

**はい、できます。** ただし「Webブラウザ上のセルフサービス新規登録フォーム」はありません(セキュリティ上の理由で2026-07-15に廃止済み)。代わりに、**サーバー起動時の環境変数**で、ご自身のメールアドレス・電話番号を「唯一ログイン可能なアカウント」として設定する方式です。

| 環境変数 | 必須/任意 | 内容 |
|---|---|---|
| `OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL` | 必須 | ご自身のメールアドレス |
| `OPEN_EASYWEB_FIXED_ACCOUNT_PHONE` | 任意 | ご自身の携帯電話番号 |
| `OPEN_EASYWEB_FIXED_ACCOUNT_BACKUP_EMAIL` | 任意 | 予備のメールアドレス |

※電話番号を設定しない場合は、予備メールアドレスの設定が必須です(どちらか一方は必要)。

**プラットフォームごとの設定方法:**
- **Windows / Linux(VPS等)**: インストール時、またはsystemdサービスの設定ファイルに環境変数として記述します。
- **Android**: アプリ内の「固定アカウント設定」画面でメールアドレスを入力します(未設定のままだと起動を拒否する安全設計です)。

つまり、本番環境(easy-web.tokyo)がユーザー自身のアドレスで動いているのと全く同じ仕組みを、ご自身でダウンロードした環境でもそのまま使えます。

## Q2. フィーチャーフォン(ガラケー)の場合、2段階認証(2FA)はPCで確認できますか?

**はい、できます。** 2FA(認証アプリによるTOTP)のセットアップ画面は、スマホカメラでのQRコード読み取りを前提とした画像表示ではなく、**テキストのシークレット文字列**をそのまま表示する設計です。

この文字列は、シークレットを手入力できるTOTPアプリであれば何でも使えます——スマホの認証アプリに限りません。ガラケーをお使いの場合は、以下のいずれかの方法が使えます。

1. **メールOTP**を使う(ガラケーでキャリアメール等が受信できれば、こちらが最も簡単です)。
2. 2FAセットアップ時に表示される「シークレット」を、**PC用の認証アプリ**(WinAuthやブラウザ拡張の認証アプリ等)に手入力し、ログイン時はPC画面に表示される6桁コードを見て入力する。

どちらの方法も、専用の説明無しに標準の設計だけで対応できます。
