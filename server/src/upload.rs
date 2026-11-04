//! サイトへアップロードされるファイルの相対パスの検証。
//! (以前は `multipart/form-data` の手書きパーサーもあったが、アップロードは Connect の `Sites/Upload`
//! (POST、RS-JSON、ファイルは base64)に移したので削除した。2026-11-04)

/// アップロードされた相対パスを検証する。`..`セグメント・絶対パスは拒否
/// (パストラバーサル対策)。
pub fn safe_relative_path(raw: &str) -> Option<std::path::PathBuf> {
    let path = std::path::Path::new(raw);
    if path.is_absolute() {
        return None;
    }
    let mut clean = std::path::PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Normal(part) => clean.push(part),
            std::path::Component::CurDir => {}
            _ => return None,
        }
    }
    if clean.as_os_str().is_empty() {
        return None;
    }
    Some(clean)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_relative_path_rejects_traversal_and_absolute() {
        assert!(safe_relative_path("../etc/passwd").is_none());
        assert!(safe_relative_path("/etc/passwd").is_none());
        assert_eq!(
            safe_relative_path("sub/dir/index.php"),
            Some(std::path::PathBuf::from("sub/dir/index.php"))
        );
    }
}
