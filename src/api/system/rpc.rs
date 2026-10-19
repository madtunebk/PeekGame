//! Small RPCRT4 UUID conversion routines.
use std::ffi::c_char;
use std::io::Read;

const RPC_S_INVALID_STRING_UUID: u32 = 1705;
const RPC_S_INVALID_ARG: u32 = 87;

#[repr(C)]
pub(crate) struct Uuid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}
const _: () = assert!(std::mem::size_of::<Uuid>() == 16);

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn parse(bytes: &[u8; 36]) -> Option<Uuid> {
    if [8, 13, 18, 23].iter().any(|&i| bytes[i] != b'-') {
        return None;
    }
    let mut raw = [0u8; 16];
    let mut digits = bytes.iter().copied().filter(|&b| b != b'-');
    for byte in &mut raw {
        *byte = (hex(digits.next()?)? << 4) | hex(digits.next()?)?;
    }
    Some(Uuid {
        data1: u32::from_be_bytes(raw[..4].try_into().unwrap()),
        data2: u16::from_be_bytes(raw[4..6].try_into().unwrap()),
        data3: u16::from_be_bytes(raw[6..8].try_into().unwrap()),
        data4: raw[8..].try_into().unwrap(),
    })
}

/// `NULL` input denotes the nil UUID. The output is Windows' UUID layout.
pub unsafe extern "win64" fn uuid_from_string_a(input: *const c_char, output: *mut Uuid) -> u32 {
    if output.is_null() {
        return RPC_S_INVALID_ARG;
    }
    if input.is_null() {
        unsafe { output.write(Uuid { data1: 0, data2: 0, data3: 0, data4: [0; 8] }) };
        return 0;
    }
    let mut bytes = [0u8; 36];
    for (index, byte) in bytes.iter_mut().enumerate() {
        let value = unsafe { *input.add(index) as u8 };
        if value == 0 { return RPC_S_INVALID_STRING_UUID; }
        *byte = value;
    }
    if unsafe { *input.add(36) } != 0 { return RPC_S_INVALID_STRING_UUID; }
    let Some(uuid) = parse(&bytes) else { return RPC_S_INVALID_STRING_UUID; };
    unsafe { output.write(uuid) };
    0
}
pub unsafe extern "win64" fn uuid_create(output: *mut Uuid) -> u32 {
    if output.is_null() {
        return RPC_S_INVALID_ARG;
    }
    let mut raw = [0u8; 16];
    if std::fs::File::open("/dev/urandom")
        .and_then(|mut source| source.read_exact(&mut raw))
        .is_err()
    {
        return 14; // RPC_S_OUT_OF_MEMORY: no UUID returned.
    }
    raw[6] = (raw[6] & 0x0f) | 0x40;
    raw[8] = (raw[8] & 0x3f) | 0x80;
    unsafe {
        output.write(Uuid {
            data1: u32::from_be_bytes(raw[..4].try_into().unwrap()),
            data2: u16::from_be_bytes(raw[4..6].try_into().unwrap()),
            data3: u16::from_be_bytes(raw[6..8].try_into().unwrap()),
            data4: raw[8..].try_into().unwrap(),
        });
    }
    0
}
