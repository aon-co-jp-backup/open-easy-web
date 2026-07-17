//! `multipart/form-data` の手書きパーサー(RFC 7578)。外部フレームワーク・
//! 外部multipart crateへは依存しない、という既存エコシステムの方針に
//! 揃えた自前実装(poem-cosmo-tauriの`hyper_compat::read_multipart_body`と
//! 同じアプローチ、このクレート単体で完結させるためコピー・簡略化)。

use bytes::Bytes;

pub struct MultipartField {
    #[allow(dead_code)] // フィールド名自体はfilenameのみ使用する既存のアップロード経路では未参照。
    pub name: String,
    pub filename: Option<String>,
    pub data: Vec<u8>,
}

pub fn multipart_boundary(content_type: &str) -> Option<String> {
    content_type.split(';').find_map(|part| {
        let part = part.trim();
        part.strip_prefix("boundary=")
            .map(|b| b.trim_matches('"').to_string())
    })
}

fn find_subslice(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if needle.is_empty() || from > haystack.len() {
        return None;
    }
    haystack[from..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + from)
}

fn parse_content_disposition(line: &str) -> (Option<String>, Option<String>) {
    let mut name = None;
    let mut filename = None;
    for piece in line.split(';').skip(1) {
        let piece = piece.trim();
        if let Some(v) = piece.strip_prefix("name=") {
            name = Some(v.trim_matches('"').to_string());
        } else if let Some(v) = piece.strip_prefix("filename=") {
            filename = Some(v.trim_matches('"').to_string());
        }
    }
    (name, filename)
}

/// `bytes` を `multipart/form-data` としてパースする。不正な場合は
/// `Err(理由)` を返す。
pub fn parse(bytes: &Bytes, boundary: &str) -> Result<Vec<MultipartField>, String> {
    let delimiter = format!("--{boundary}").into_bytes();
    let mut fields = Vec::new();

    let Some(first_boundary) = find_subslice(bytes, &delimiter, 0) else {
        return Err("malformed multipart body: boundary not found".into());
    };
    let mut pos = first_boundary + delimiter.len();

    loop {
        if bytes[pos..].starts_with(b"--") {
            break;
        }
        if bytes[pos..].starts_with(b"\r\n") {
            pos += 2;
        }
        let Some(header_end) = find_subslice(bytes, b"\r\n\r\n", pos) else {
            break;
        };
        let header_bytes = &bytes[pos..header_end];
        let headers_str = String::from_utf8_lossy(header_bytes);

        let mut name = None;
        let mut filename = None;
        for line in headers_str.split("\r\n") {
            let lower = line.to_ascii_lowercase();
            if lower.starts_with("content-disposition:") {
                let (n, f) = parse_content_disposition(
                    &line[line.find(':').map(|i| i + 1).unwrap_or(0)..],
                );
                name = n;
                filename = f;
            }
        }

        let body_start = header_end + 4;
        let Some(next_boundary) = find_subslice(bytes, &delimiter, body_start) else {
            break;
        };
        let mut body_end = next_boundary;
        if body_end >= body_start + 2 && &bytes[body_end - 2..body_end] == b"\r\n" {
            body_end -= 2;
        }
        let data = bytes[body_start..body_end].to_vec();

        if let Some(name) = name {
            fields.push(MultipartField { name, filename, data });
        }

        pos = next_boundary + delimiter.len();
    }

    Ok(fields)
}

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
    fn parses_two_file_fields() {
        let boundary = "X-BOUNDARY";
        let body = format!(
            "--{b}\r\nContent-Disposition: form-data; name=\"files\"; filename=\"index.php\"\r\n\r\n<?php ?>\r\n--{b}\r\nContent-Disposition: form-data; name=\"files\"; filename=\"style.css\"\r\n\r\nbody{{}}\r\n--{b}--\r\n",
            b = boundary
        );
        let fields = parse(&Bytes::from(body), boundary).unwrap();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].filename.as_deref(), Some("index.php"));
        assert_eq!(fields[0].data, b"<?php ?>");
        assert_eq!(fields[1].filename.as_deref(), Some("style.css"));
    }

    #[test]
    fn missing_boundary_marker_is_an_error() {
        let result = parse(&Bytes::from_static(b"not multipart"), "X");
        assert!(result.is_err());
    }

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
