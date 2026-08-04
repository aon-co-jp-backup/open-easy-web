# iOS/iPadOS対応 実現可能性メモ(2026-08-03、調査のみ・未着手)

ユーザー指示: 「端末間ドライブ同期+自動バックアップ」をWindows/Linux/
Mac/Android/iPhone/iPadへ展開する構想のうち、iOS/iPadOS部分は今回
実装に着手せず、実現可能性の調査結果をこのファイルに記録して次回へ
持ち越す(2026-08-03ユーザー指示「3の調査は、記録ファイルに調査内容を
記録して次回に」)。

## 現状(このエコシステム全体でiOS実績ゼロ)

`F:\runo`配下の全リポジトリを通じて、iOS/iPadOS向けのコード・ビルド
設定・実機/シミュレータでの検証実績は**一件も存在しない**。Android版は
`open-web-server`/`open-easy-web`/`aruaru-db`/`open-redmine`の4リポジトリで
`cargo ndk`によるクロスコンパイル+Kotlin製シェルアプリという確立した
パターンがあるが、iOSは全く別の技術スタックが必要。

## 技術的な選択肢と制約

1. **Rust本体のiOSクロスコンパイル自体は可能**:
   `rustup target add aarch64-apple-ios aarch64-apple-ios-sim
   x86_64-apple-ios`でターゲット追加は可能(Rust公式サポート対象)。
   `cargo-lipo`または`cargo build --target aarch64-apple-ios`で
   staticlib/dylibを生成できる。
   - **ただし、この開発環境(Windows)ではiOS向けクロスコンパイル自体が
     実行不可能**——AppleのCode Signing・iOS SDK(`Xcode`のみが提供、
     Apple製品上でしか入手できない)が無いと、たとえRustツールチェーン上
     ターゲットを追加できてもリンク・署名ができない。**実機/シミュレータ
     でのビルド確認にはmacOS環境(実機のMacまたはmacOSクラウドCI)が
     必須**——この既知の制約はopen-easy-web CLAUDE.mdの「将来のmacOS
     対応」節(2026-07-23付)でも既に明記されている。
2. **UI層**: 既存エコシステムの「Rust→WASM(wasm-bindgen)」パターンは
   iOSネイティブアプリには使えない(WKWebViewでWASMを動かすことは
   技術的に可能だが、ファイルシステムアクセス・バックグラウンド同期・
   プッシュ通知等のOS機能にはネイティブブリッジが別途必要になり、
   Android版の「Kotlinシェル+ProcessBuilderでバイナリ起動」ほど単純では
   ない)。iOSは**サンドボックス化されたアプリごとの専用ディレクトリ
   以外への直接ファイルシステムアクセスができない**——Android版が
   `/proc/meminfo`を直接読む・任意パスへ`ProcessBuilder`でバイナリを
   起動するような設計は、iOSでは同じ形では実現できない。
3. **バックグラウンド同期の制約(iOS特有、最重要)**:
   iOSはAndroidと異なり、アプリが恒常的にバックグラウンドで動き続ける
   ことを許さない(電池・リソース管理のOS側ポリシーが非常に厳格)。
   常時稼働のsyncデーモンのような設計は原理的に不可能で、
   `BGTaskScheduler`(Background Tasks framework)による短時間の定期実行
   か、ユーザーがアプリを前面で開いている間のみの同期、という制約付き
   設計にせざるを得ない。「常時電源接続で常駐」という他プラットフォーム
   版の設計思想はiOSには移植できない。
4. **配布**: App Store経由の配布には(a) Apple Developer Program
   登録(有償、年会費)、(b) App Store Review(審査、任意コードの
   ダウンロード実行〈他リポジトリのAndroid版が実際に行っている
   バイナリ埋め込み+ProcessBuilder起動のパターン〉は審査ガイドライン
   違反になる可能性が高い)、(c) TestFlight経由の限定配布、といった
   選択肢がある。**「インストーラー付きアプリ」という表現が意図する
   「野良ビルドの直接配布」は、iOSでは通常のiPhoneではApple公式の
   仕組み(App Store/TestFlight/Enterprise配布)を経由しない限り
   事実上不可能**(脱獄前提の配布は現実的な選択肢として扱わない)。
5. **WiFiルーター接続USB/外付けHDDへのアクセス**: iOS単体からは
   SMB/DLNAクライアント機能自体は`Files`アプリ経由で標準対応している
   ため、"Files provider extension"としてこのアプリを実装すれば
   `Files`アプリから見える形での統合は可能——ただし通常のアプリ内蔵
   バックグラウンド同期とは別の実装形態(File Provider Extension、
   Swift/Objective-Cでの実装が事実上必須)になる。

## 結論(正直な評価)

iOS/iPadOS対応は、他プラットフォーム版(Windows/Linux/Mac/Android)の
延長線上の作業ではなく、**(a) macOS環境の確保、(b) Swift/Xcodeでの
ネイティブシェル実装、(c) File Provider Extension等のiOS固有の統合
方式の設計、(d) Apple Developer Program登録・審査対応**という4つの
別プロジェクト規模の前提が必要な、実質的に新規の開発ラインである。
「まず記録して次回」というユーザー判断は妥当。

## 次回着手する場合の推奨手順

1. macOS環境(実機のMacまたはクラウドMac CI、例: GitHub Actionsの
   `macos-latest`ランナー)を確保する。
2. まず`cargo build --target aarch64-apple-ios-sim`がこの環境で通るかを
   確認する(Rustコア部分のクロスコンパイル可否の一次切り分け)。
3. Swift側のシェルアプリを最小構成(ヘルスチェック+ブラウザ起動、
   Android版の「リモートクライアント」設計と同じ最小スコープ)から
   着手し、いきなりバックグラウンド常駐同期を狙わない。
4. File Provider Extensionの要否は、「WiFiルーター接続USB/HDD/NASへの
   iOSからのアクセス」という要件が実際にどこまで必要かをユーザーに
   再確認してから設計する(通常のSMBクライアントアプリで足りるか、
   Files.app統合まで必要かで実装コストが大きく変わる)。
