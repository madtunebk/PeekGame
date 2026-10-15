//! Bounds-checked DER projection into Windows CERT_INFO. This parses metadata;
//! it does not decide whether a certificate is trusted or verify its signature.
use std::ffi::{c_char, CString};
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Blob {
    pub size: u32,
    pub data: *const u8,
}
impl Blob {
    fn new(data: &[u8]) -> Self {
        Self {
            size: data.len() as u32,
            data: data.as_ptr(),
        }
    }
}
#[repr(C)]
pub struct Algorithm {
    pub oid: *const c_char,
    pub parameters: Blob,
}
#[repr(C)]
pub struct Bits {
    pub size: u32,
    pub data: *const u8,
    pub unused: u32,
}
impl Bits {
    fn empty() -> Self {
        Self {
            size: 0,
            data: std::ptr::null(),
            unused: 0,
        }
    }
}
#[repr(C)]
pub struct PublicKey {
    pub algorithm: Algorithm,
    pub bits: Bits,
}
#[repr(C)]
pub struct Extension {
    pub oid: *const c_char,
    pub critical: i32,
    pub value: Blob,
}
#[repr(C)]
pub struct Info {
    pub version: u32,
    pub serial: Blob,
    pub signature: Algorithm,
    pub issuer: Blob,
    pub not_before: [u32; 2],
    pub not_after: [u32; 2],
    pub subject: Blob,
    pub key: PublicKey,
    pub issuer_id: Bits,
    pub subject_id: Bits,
    pub extension_count: u32,
    pub extensions: *const Extension,
}
pub struct Certificate {
    pub encoded: Box<[u8]>,
    pub info: Box<Info>,
    _serial: Box<[u8]>,
    _oids: Vec<CString>,
    _extensions: Box<[Extension]>,
}
// Pointers above refer only to this object's immutable owned allocations.
unsafe impl Send for Certificate {}
unsafe impl Sync for Certificate {}
struct Value<'a> {
    tag: u8,
    full: &'a [u8],
    body: &'a [u8],
}
fn take<'a>(input: &mut &'a [u8]) -> Result<Value<'a>, ()> {
    let all = *input;
    if all.len() < 2 {
        return Err(());
    }
    let tag = all[0];
    let mut head = 2usize;
    let mut len = all[1] as usize;
    if len & 128 != 0 {
        let n = len & 127;
        if n == 0 || n > 4 || all.len() < head + n || all[head] == 0 {
            return Err(());
        }
        len = 0;
        for byte in &all[head..head + n] {
            len = len.checked_mul(256).ok_or(())? + *byte as usize;
        }
        head += n;
        if len < 128 {
            return Err(());
        }
    }
    let end = head.checked_add(len).ok_or(())?;
    if end > all.len() {
        return Err(());
    }
    *input = &all[end..];
    Ok(Value {
        tag,
        full: &all[..end],
        body: &all[head..end],
    })
}
fn tagged<'a>(input: &mut &'a [u8], tag: u8) -> Result<Value<'a>, ()> {
    let value = take(input)?;
    if value.tag != tag {
        Err(())
    } else {
        Ok(value)
    }
}
fn oid(data: &[u8]) -> Result<CString, ()> {
    if data.is_empty() {
        return Err(());
    }
    let mut components = Vec::new();
    let mut value = 0u64;
    let mut start = true;
    for byte in data {
        if start && *byte == 128 {
            return Err(());
        }
        value = value
            .checked_mul(128)
            .and_then(|v| v.checked_add((byte & 127) as u64))
            .ok_or(())?;
        if byte & 128 == 0 {
            components.push(value);
            value = 0;
            start = true;
        } else {
            start = false;
        }
    }
    if !start {
        return Err(());
    }
    let first = components[0];
    let prefix = if first < 40 {
        (0, first)
    } else if first < 80 {
        (1, first - 40)
    } else {
        (2, first - 80)
    };
    let mut text = format!("{}.{}", prefix.0, prefix.1);
    for part in &components[1..] {
        text.push_str(&format!(".{part}"));
    }
    CString::new(text).map_err(|_| ())
}
fn algorithm(input: &mut &[u8], oids: &mut Vec<CString>) -> Result<Algorithm, ()> {
    let mut body = tagged(input, 0x30)?.body;
    oids.push(oid(tagged(&mut body, 6)?.body)?);
    let result = Algorithm {
        oid: oids.last().unwrap().as_ptr(),
        parameters: Blob::new(body),
    };
    if !body.is_empty() {
        take(&mut body)?;
        if !body.is_empty() {
            return Err(());
        }
    }
    Ok(result)
}
fn bits(value: &[u8]) -> Result<Bits, ()> {
    if value.is_empty() || value[0] > 7 {
        return Err(());
    }
    Ok(Bits {
        size: (value.len() - 1) as u32,
        data: value[1..].as_ptr(),
        unused: value[0] as u32,
    })
}
fn time(value: Value<'_>) -> Result<[u32; 2], ()> {
    let digits = |s: &[u8]| -> Result<i64, ()> {
        s.iter().try_fold(0i64, |n, b| {
            if b.is_ascii_digit() {
                Ok(n * 10 + (b - b'0') as i64)
            } else {
                Err(())
            }
        })
    };
    let (year, rest) = match (value.tag, value.body.len()) {
        (0x17, 13) => {
            let n = digits(&value.body[..2])?;
            (if n >= 50 { 1900 + n } else { 2000 + n }, &value.body[2..])
        }
        (0x18, 15) => (digits(&value.body[..4])?, &value.body[4..]),
        _ => return Err(()),
    };
    if rest[10] != b'Z' || year < 1601 {
        return Err(());
    }
    let month = digits(&rest[..2])?;
    let day = digits(&rest[2..4])?;
    let hour = digits(&rest[4..6])?;
    let minute = digits(&rest[6..8])?;
    let second = digits(&rest[8..10])?;
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        _ => return Err(()),
    };
    if day < 1 || day > max || hour > 23 || minute > 59 || second > 59 {
        return Err(());
    }
    let y = year - if month <= 2 { 1 } else { 0 };
    let era = y / 400;
    let yoe = y - era * 400;
    let m = month + if month > 2 { -3 } else { 9 };
    let days =
        era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + (153 * m + 2) / 5 + day - 1 - 719468;
    let ticks = ((days * 86400 + hour * 3600 + minute * 60 + second + 11644473600) as u64)
        .checked_mul(10000000)
        .ok_or(())?;
    Ok([ticks as u32, (ticks >> 32) as u32])
}
impl Certificate {
    pub fn parse(encoded: Vec<u8>) -> Result<Self, ()> {
        let encoded = encoded.into_boxed_slice();
        let mut input = &*encoded;
        let mut certificate = tagged(&mut input, 0x30)?.body;
        if !input.is_empty() {
            return Err(());
        }
        let mut tbs = tagged(&mut certificate, 0x30)?.body;
        let mut oids = Vec::new();
        let version = if tbs.first() == Some(&0xa0) {
            let mut explicit = take(&mut tbs)?.body;
            let integer = tagged(&mut explicit, 2)?;
            if integer.body.len() != 1 || integer.body[0] > 2 || !explicit.is_empty() {
                return Err(());
            }
            integer.body[0] as u32
        } else {
            0
        };
        let serial_value = tagged(&mut tbs, 2)?.body;
        if serial_value.is_empty() {
            return Err(());
        }
        let serial = serial_value
            .iter()
            .rev()
            .copied()
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let signature = algorithm(&mut tbs, &mut oids)?;
        let issuer = Blob::new(tagged(&mut tbs, 0x30)?.full);
        let mut validity = tagged(&mut tbs, 0x30)?.body;
        let not_before = time(take(&mut validity)?)?;
        let not_after = time(take(&mut validity)?)?;
        if !validity.is_empty() {
            return Err(());
        }
        let subject = Blob::new(tagged(&mut tbs, 0x30)?.full);
        let mut public = tagged(&mut tbs, 0x30)?.body;
        let key = PublicKey {
            algorithm: algorithm(&mut public, &mut oids)?,
            bits: bits(tagged(&mut public, 3)?.body)?,
        };
        if !public.is_empty() {
            return Err(());
        }
        let mut issuer_id = Bits::empty();
        let mut subject_id = Bits::empty();
        let mut extensions = Vec::new();
        while !tbs.is_empty() {
            let value = take(&mut tbs)?;
            match value.tag {
                0x81 => issuer_id = bits(value.body)?,
                0x82 => subject_id = bits(value.body)?,
                0xa3 => {
                    let mut explicit = value.body;
                    let mut list = tagged(&mut explicit, 0x30)?.body;
                    if !explicit.is_empty() {
                        return Err(());
                    }
                    while !list.is_empty() {
                        let mut ext = tagged(&mut list, 0x30)?.body;
                        oids.push(oid(tagged(&mut ext, 6)?.body)?);
                        let name = oids.last().unwrap().as_ptr();
                        let critical = if ext.first() == Some(&1) {
                            let v = take(&mut ext)?;
                            if v.body.len() != 1 {
                                return Err(());
                            }
                            i32::from(v.body[0] != 0)
                        } else {
                            0
                        };
                        let value = Blob::new(tagged(&mut ext, 4)?.body);
                        if !ext.is_empty() {
                            return Err(());
                        }
                        extensions.push(Extension {
                            oid: name,
                            critical,
                            value,
                        });
                    }
                }
                _ => return Err(()),
            }
        }
        algorithm(&mut certificate, &mut oids)?;
        bits(tagged(&mut certificate, 3)?.body)?;
        if !certificate.is_empty() {
            return Err(());
        }
        let extensions = extensions.into_boxed_slice();
        let info = Box::new(Info {
            version,
            serial: Blob::new(&serial),
            signature,
            issuer,
            not_before,
            not_after,
            subject,
            key,
            issuer_id,
            subject_id,
            extension_count: extensions.len() as u32,
            extensions: if extensions.is_empty() {
                std::ptr::null()
            } else {
                extensions.as_ptr()
            },
        });
        Ok(Self {
            encoded,
            info,
            _serial: serial,
            _oids: oids,
            _extensions: extensions,
        })
    }
}
