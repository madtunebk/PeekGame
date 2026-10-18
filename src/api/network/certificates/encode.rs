//! CryptEncodeObjectEx. X509_NAME (CERT_NAME_INFO) is encoded to DER; every other
//! structure fails with ERROR_NOT_SUPPORTED instead of returning fabricated DER.
use super::fail;
use std::ffi::{c_char, c_void, CStr};

const X509_NAME: usize = 7;
const CRYPT_ENCODE_ALLOC_FLAG: u32 = 0x8000;
const ERROR_MORE_DATA: u32 = 234;

/// One DER TLV.
pub(super) fn tlv(tag: u8, body: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    let n = body.len();
    if n < 0x80 {
        out.push(n as u8);
    } else {
        let bytes = n.to_be_bytes();
        let skip = bytes.iter().take_while(|b| **b == 0).count();
        out.push(0x80 | (bytes.len() - skip) as u8);
        out.extend_from_slice(&bytes[skip..]);
    }
    out.extend_from_slice(body);
    out
}
/// A dotted OID ("2.5.4.3") as an OBJECT IDENTIFIER body.
fn oid(text: &str) -> Option<Vec<u8>> {
    let arcs: Vec<u64> = text.split('.').map(|a| a.parse().ok()).collect::<Option<_>>()?;
    if arcs.len() < 2 || arcs[0] > 2 || (arcs[0] < 2 && arcs[1] > 39) {
        return None;
    }
    let mut out = Vec::new();
    let mut push = |mut v: u64| {
        let mut groups = vec![(v & 0x7f) as u8];
        v >>= 7;
        while v != 0 {
            groups.push(0x80 | (v & 0x7f) as u8);
            v >>= 7;
        }
        out.extend(groups.iter().rev());
    };
    push(arcs[0] * 40 + arcs[1]);
    for &arc in &arcs[2..] {
        push(arc);
    }
    Some(out)
}
/// UTF-16 data of a CERT_RDN_VALUE_BLOB: `size` bytes, or NUL-terminated when 0.
unsafe fn wide(data: *const u8, size: u32) -> Vec<u16> {
    let data = data.cast::<u16>();
    let count = if size == 0 {
        let mut n = 0;
        while unsafe { data.add(n).read_unaligned() } != 0 {
            n += 1;
        }
        n
    } else {
        size as usize / 2
    };
    (0..count).map(|i| unsafe { data.add(i).read_unaligned() }).collect()
}
/// One CERT_RDN_ATTR value (dwValueType, Value) in X509_NAME form.
unsafe fn value(kind: u32, data: *const u8, size: u32) -> Result<Vec<u8>, u32> {
    if data.is_null() && size != 0 {
        return Err(87);
    }
    let bytes = || if size == 0 { &[][..] } else { unsafe { std::slice::from_raw_parts(data, size as usize) } };
    let tag = match kind {
        1 => return Ok(bytes().to_vec()), // CERT_RDN_ENCODED_BLOB: already DER
        2 => 0x04,                        // OCTET_STRING
        3 => 0x12,                        // NUMERIC_STRING
        4 => 0x13,                        // PRINTABLE_STRING
        5 => 0x14,                        // TELETEX_STRING
        6 => 0x15,                        // VIDEOTEX_STRING
        7 => 0x16,                        // IA5_STRING
        8 => 0x19,                        // GRAPHIC_STRING
        9 => 0x1a,                        // VISIBLE_STRING
        10 => 0x1b,                       // GENERAL_STRING
        12 => {
            // CERT_RDN_BMP_STRING: UTF-16 in, big-endian UCS-2 on the wire.
            let text = if data.is_null() { Vec::new() } else { unsafe { wide(data, size) } };
            let body: Vec<u8> = text.iter().flat_map(|c| c.to_be_bytes()).collect();
            return Ok(tlv(0x1e, &body));
        }
        13 => {
            // CERT_RDN_UTF8_STRING: UTF-16 in, UTF-8 on the wire.
            let text = if data.is_null() { Vec::new() } else { unsafe { wide(data, size) } };
            let text = String::from_utf16(&text).map_err(|_| 0x80092002u32)?;
            return Ok(tlv(0x0c, text.as_bytes()));
        }
        _ => return Err(50),
    };
    Ok(tlv(tag, bytes()))
}
/// CERT_NAME_INFO (x64): cRDN @0, rgRDN @8. CERT_RDN: cRDNAttr @0, rgRDNAttr @8 (16 bytes).
/// CERT_RDN_ATTR: pszObjId @0, dwValueType @8, Value.cbData @16, Value.pbData @24 (32 bytes).
pub(super) unsafe fn name(info: *const u8) -> Result<Vec<u8>, u32> {
    let read32 = |p: *const u8, at: usize| unsafe { p.add(at).cast::<u32>().read_unaligned() };
    let read_ptr = |p: *const u8, at: usize| unsafe { p.add(at).cast::<usize>().read_unaligned() } as *const u8;
    let (count, rdns) = (read32(info, 0), read_ptr(info, 8));
    if count != 0 && rdns.is_null() {
        return Err(87);
    }
    let mut sequence = Vec::new();
    for i in 0..count as usize {
        let rdn = unsafe { rdns.add(i * 16) };
        let (attr_count, attrs) = (read32(rdn, 0), read_ptr(rdn, 8));
        if attr_count != 0 && attrs.is_null() {
            return Err(87);
        }
        let mut set: Vec<Vec<u8>> = Vec::new();
        for j in 0..attr_count as usize {
            let attr = unsafe { attrs.add(j * 32) };
            let id = read_ptr(attr, 0);
            if id.is_null() {
                return Err(87);
            }
            let id = unsafe { CStr::from_ptr(id.cast::<c_char>()) }.to_str().map_err(|_| 87u32)?;
            let id = oid(id).ok_or(0x80092002u32)?;
            let value = unsafe { value(read32(attr, 8), read_ptr(attr, 24), read32(attr, 16)) }?;
            let mut body = tlv(0x06, &id);
            body.extend_from_slice(&value);
            set.push(tlv(0x30, &body));
        }
        // DER orders SET OF members by their encodings.
        set.sort();
        sequence.extend_from_slice(&tlv(0x31, &set.concat()));
    }
    Ok(tlv(0x30, &sequence))
}

pub unsafe extern "win64" fn encode_object_ex(
    encoding: u32, struct_type: *const c_char, info: *const c_void,
    flags: u32, params: *const c_void, encoded: *mut c_void, size: *mut u32,
) -> i32 {
    let ordinal = struct_type as usize;
    if ordinal != X509_NAME {
        let name = if ordinal < 65536 {
            format!("#{ordinal:#x}")
        } else {
            unsafe { CStr::from_ptr(struct_type) }.to_string_lossy().into_owned()
        };
        eprintln!("crypt32: CryptEncodeObjectEx encoding={encoding:#x} type={name:?} flags={flags:#x}: unsupported");
        if !size.is_null() {
            unsafe { *size = 0 };
        }
        return fail(50, 0);
    }
    if encoding & 0xffff != 1 || info.is_null() || size.is_null() {
        return fail(87, 0);
    }
    // Allocation through CRYPT_ENCODE_PARA callbacks and the other flags are not supported.
    if flags & !CRYPT_ENCODE_ALLOC_FLAG != 0 || !params.is_null() {
        eprintln!("crypt32: CryptEncodeObjectEx X509_NAME flags={flags:#x} params={params:?}: unsupported");
        return fail(50, 0);
    }
    let der = match unsafe { name(info.cast()) } {
        Ok(der) => der,
        Err(code) => return fail(code, 0),
    };
    if flags & CRYPT_ENCODE_ALLOC_FLAG != 0 {
        // pvEncoded receives a LocalAlloc block; the caller frees it with LocalFree.
        if encoded.is_null() {
            return fail(87, 0);
        }
        let block = crate::api::memory::local::allocate(0, der.len());
        if block == 0 {
            return fail(8, 0);
        }
        unsafe {
            std::ptr::copy_nonoverlapping(der.as_ptr(), block as *mut u8, der.len());
            *encoded.cast::<usize>() = block;
            *size = der.len() as u32;
        }
        return 1;
    }
    let capacity = unsafe { *size } as usize;
    unsafe { *size = der.len() as u32 };
    if encoded.is_null() {
        return 1;
    }
    if capacity < der.len() {
        return fail(ERROR_MORE_DATA, 0);
    }
    unsafe { std::ptr::copy_nonoverlapping(der.as_ptr(), encoded.cast::<u8>(), der.len()) };
    1
}

#[cfg(test)]
mod tests {
    #[test]
    fn x509_name_with_utf8_common_name() {
        // CN=Ab as CERT_RDN_UTF8_STRING (UTF-16 input).
        let text: Vec<u16> = "Ab".encode_utf16().collect();
        let oid = c"2.5.4.3";
        let mut attr = [0u8; 32];
        attr[0..8].copy_from_slice(&(oid.as_ptr() as usize).to_le_bytes());
        attr[8..12].copy_from_slice(&13u32.to_le_bytes());
        attr[16..20].copy_from_slice(&4u32.to_le_bytes());
        attr[24..32].copy_from_slice(&(text.as_ptr() as usize).to_le_bytes());
        let mut rdn = [0u8; 16];
        rdn[0..4].copy_from_slice(&1u32.to_le_bytes());
        rdn[8..16].copy_from_slice(&(attr.as_ptr() as usize).to_le_bytes());
        let mut info = [0u8; 16];
        info[0..4].copy_from_slice(&1u32.to_le_bytes());
        info[8..16].copy_from_slice(&(rdn.as_ptr() as usize).to_le_bytes());
        let der = unsafe { super::name(info.as_ptr()) }.unwrap();
        assert_eq!(der, [0x30, 0x0d, 0x31, 0x0b, 0x30, 0x09, 0x06, 0x03, 0x55, 0x04, 0x03, 0x0c, 0x02, b'A', b'b']);
    }
    #[test]
    fn oid_encoding() {
        assert_eq!(super::oid("1.2.840.113549.1.1.11").unwrap(), [0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b]);
        assert!(super::oid("3.1").is_none());
    }
}
