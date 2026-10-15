//! Read the host's trust bundle; do not synthesize trusted roots.
use super::der::Certificate;
use std::{path::PathBuf, sync::Arc};
fn decode(text: &str) -> Result<Vec<u8>, ()> {
    let bytes = text
        .bytes()
        .filter(|b| !b.is_ascii_whitespace())
        .collect::<Vec<_>>();
    if bytes.is_empty() || bytes.len() % 4 != 0 {
        return Err(());
    }
    let mut out = Vec::new();
    for (index, quad) in bytes.chunks_exact(4).enumerate() {
        let mut values = [0u32; 4];
        let mut padding = 0;
        for i in 0..4 {
            values[i] = match quad[i] {
                b'A'..=b'Z' => (quad[i] - b'A') as u32,
                b'a'..=b'z' => (quad[i] - b'a' + 26) as u32,
                b'0'..=b'9' => (quad[i] - b'0' + 52) as u32,
                b'+' => 62,
                b'/' => 63,
                b'=' if i >= 2 && index + 1 == bytes.len() / 4 => {
                    padding += 1;
                    0
                }
                _ => return Err(()),
            };
            if padding != 0 && quad[i] != b'=' {
                return Err(());
            }
        }
        let value = values[0] << 18 | values[1] << 12 | values[2] << 6 | values[3];
        if padding == 2 && values[1] & 15 != 0 || padding == 1 && values[2] & 3 != 0 {
            return Err(());
        }
        out.push((value >> 16) as u8);
        if padding < 2 {
            out.push((value >> 8) as u8);
        }
        if padding == 0 {
            out.push(value as u8);
        }
    }
    Ok(out)
}
pub fn roots() -> Result<Vec<Arc<Certificate>>, u32> {
    let path = if let Some(path) = std::env::var_os("SSL_CERT_FILE") {
        PathBuf::from(path)
    } else {
        [
            "/etc/ssl/certs/ca-certificates.crt",
            "/etc/pki/tls/certs/ca-bundle.crt",
            "/etc/ssl/ca-bundle.pem",
            "/etc/ssl/cert.pem",
        ]
        .iter()
        .map(PathBuf::from)
        .find(|p| p.is_file())
        .ok_or(2u32)?
    };
    let text = std::fs::read_to_string(path).map_err(|_| 2u32)?;
    let mut rest = text.as_str();
    let mut result = Vec::new();
    while let Some((_, body)) = rest.split_once("-----BEGIN CERTIFICATE-----") {
        let (encoded, next) = body
            .split_once("-----END CERTIFICATE-----")
            .ok_or(0x8009310bu32)?;
        let certificate = Certificate::parse(decode(encoded).map_err(|_| 0x8009310bu32)?)
            .map_err(|_| 0x8009310bu32)?;
        result.push(Arc::new(certificate));
        rest = next;
    }
    if result.is_empty() {
        return Err(0x80092004);
    }
    Ok(result)
}
