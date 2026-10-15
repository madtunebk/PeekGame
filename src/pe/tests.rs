use super::*;
fn put16(b: &mut [u8], p: usize, v: u16) {
    b[p..p + 2].copy_from_slice(&v.to_le_bytes());
}
fn put32(b: &mut [u8], p: usize, v: u32) {
    b[p..p + 4].copy_from_slice(&v.to_le_bytes());
}
fn put64(b: &mut [u8], p: usize, v: u64) {
    b[p..p + 8].copy_from_slice(&v.to_le_bytes());
}
fn fixture(delayed: bool) -> Vec<u8> {
    let mut b = vec![0; 1024];
    b[..2].copy_from_slice(b"MZ");
    put32(&mut b, 60, 64);
    b[64..68].copy_from_slice(b"PE\0\0");
    put16(&mut b, 68, 0x8664);
    put16(&mut b, 70, 1);
    put16(&mut b, 84, 240);
    put16(&mut b, 88, 0x20b);
    put32(&mut b, 148, 512);
    put32(&mut b, 196, 16);
    put32(&mut b, 336, 512);
    put32(&mut b, 340, 4096);
    put32(&mut b, 344, 512);
    put32(&mut b, 348, 512);
    let d = 200 + if delayed { 13 * 8 } else { 8 };
    put32(&mut b, d, 4096);
    put32(&mut b, d + 4, if delayed { 64 } else { 40 });
    if delayed {
        put32(&mut b, 512, 1);
        put32(&mut b, 516, 4200);
        put32(&mut b, 524, 4224);
        put32(&mut b, 528, 4224);
    } else {
        put32(&mut b, 512, 4224);
        put32(&mut b, 524, 4200);
        put32(&mut b, 528, 4224);
    }
    b[616..629].copy_from_slice(b"KERNEL32.dll\0");
    put64(&mut b, 640, 1u64 << 63 | 42);
    put64(&mut b, 648, 4256);
    put16(&mut b, 672, 7);
    b[674..686].copy_from_slice(b"ExitProcess\0");
    b
}
#[test]
fn named_and_ordinal_imports() {
    let i = parse(&fixture(false)).unwrap();
    assert_eq!(i.imports.len(), 2);
    assert_eq!(i.imports[0].ordinal, Some(42));
    assert_eq!(i.imports[1].name.as_deref(), Some("ExitProcess"));
    assert_eq!(i.imports[1].hint, Some(7));
    assert_eq!(i.imports[1].iat_rva, 4232);
}
#[test]
fn delay_imports() {
    assert!(parse(&fixture(true))
        .unwrap()
        .imports
        .iter()
        .all(|i| i.delayed));
}
#[test]
fn truncated_input_never_panics() {
    let b = fixture(false);
    for n in 0..b.len() {
        let _ = parse(&b[..n]);
    }
}
#[test]
fn unmapped_thunk_is_an_error() {
    let mut b = fixture(false);
    put64(&mut b, 648, 0x9000);
    assert!(parse(&b).is_err());
}
#[test]
fn descriptor_needs_terminator() {
    let mut b = fixture(false);
    put32(&mut b, 212, 20);
    assert!(parse(&b).is_err());
}
