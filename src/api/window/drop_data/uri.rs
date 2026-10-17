//! Strict local file:// URI lists. Remote hosts and malformed escapes fail.
pub(super) fn parse(bytes: &[u8]) -> Option<Vec<String>> {
    if bytes.len() > 4 * 1024 * 1024 {
        return None;
    }
    let text = std::str::from_utf8(bytes).ok()?;
    let mut files = Vec::new();
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let rest = line.strip_prefix("file://")?;
        let slash = rest.find('/')?;
        if !rest[..slash].is_empty() && !rest[..slash].eq_ignore_ascii_case("localhost") {
            return None;
        }
        let path = &rest[slash..];
        let b = path.as_bytes();
        let mut out = Vec::new();
        let mut i = 0;
        while i < b.len() {
            if b[i] == b'%' {
                if i + 2 >= b.len() {
                    return None;
                }
                let h = |x: u8| (x as char).to_digit(16).map(|n| n as u8);
                let n = h(b[i + 1])? * 16 + h(b[i + 2])?;
                if n == 0 {
                    return None;
                }
                out.push(n);
                i += 3;
            } else {
                if matches!(b[i], 0 | b'?' | b'#' | b'\\') {
                    return None;
                }
                out.push(b[i]);
                i += 1;
            }
        }
        let path = String::from_utf8(out).ok()?;
        if path.contains('\\') {
            return None;
        }
        files.push(crate::host::paths::windows(std::path::Path::new(&path)));
        if files.len() > 4096 {
            return None;
        }
    }
    if files.is_empty() {
        None
    } else {
        Some(files)
    }
}
