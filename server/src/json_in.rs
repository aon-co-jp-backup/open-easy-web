//! 外から入ってくる JSON(HTTP リクエスト本文、他サービスの応答、ディスク上の状態ファイル)を読む入口(2026-11-04、
//! ユーザー指示「Connect、RS-JSON に例外なく統一」)。読み込みは RS-JSON の `from_str_secure`
//! (サイズ・深さ・重複キーの上限つき)だけを使い、`serde_json::from_*` / `sonic_rs::from_*` を外部入力に直接使わない。
//! 書き出しは `rust_json::fast`。

use serde::de::DeserializeOwned;

/// リクエスト本文の既定の上限(64 KiB)。
pub const DEFAULT_MAX: usize = 64 * 1024;

pub fn parse_with_limit<T: DeserializeOwned>(bytes: &[u8], max_bytes: usize) -> Result<T, String> {
    let text = std::str::from_utf8(bytes).map_err(|e| format!("本文が UTF-8 ではありません: {e}"))?;
    let limits = rust_json::Limits { max_bytes, max_depth: 32, reject_duplicate_keys: true };
    rust_json::from_str_secure(text, &limits).map_err(|e| format!("invalid JSON: {e}"))
}

/// 上限 64 KiB で読む(HTTP リクエスト本文)。
pub fn parse<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    parse_with_limit(bytes, DEFAULT_MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_duplicate_keys_oversize_and_bad_utf8() {
        #[derive(serde::Deserialize, Debug)]
        struct P {
            #[allow(dead_code)]
            a: u32,
        }
        assert!(parse::<P>(br#"{"a":1}"#).is_ok());
        assert!(parse::<P>(br#"{"a":1,"a":2}"#).is_err());
        assert!(parse_with_limit::<P>(br#"{"a":1}"#, 3).is_err());
        assert!(parse::<P>(&[0xff, 0xfe]).is_err());
    }
}
