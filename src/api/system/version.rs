//! The Windows version PeekGame reports, and the version checks against it.
//! Windows 10 22H2 (10.0.19045), a workstation: what current games expect, as an
//! application manifest declaring Windows 10 support would see it.
use super::error;
const MAJOR: u32 = 10;
const MINOR: u32 = 0;
const BUILD: u32 = 19045;
const PLATFORM: u32 = 2; // VER_PLATFORM_WIN32_NT
const PRODUCT_TYPE: u8 = 1; // VER_NT_WORKSTATION
const SUITE: u16 = 0x100; // VER_SUITE_SINGLEUSERTS
pub extern "win64" fn get_version() -> u32 {
    BUILD << 16 | MINOR << 8 | MAJOR
}
/// Report the Windows 10 Pro edition of the workstation exposed by this runtime.
pub unsafe extern "win64" fn get_product_info(
    major: u32,
    _minor: u32,
    _service_pack_major: u32,
    _service_pack_minor: u32,
    product: *mut u32,
) -> i32 {
    if major < 6 || product.is_null() {
        error::set(87); // ERROR_INVALID_PARAMETER
        return 0;
    }
    unsafe { product.write(0x30) }; // PRODUCT_PROFESSIONAL
    1
}
/// OSVERSIONINFO(EX)W: size, major, minor, build, platform, szCSDVersion[128]
/// (WCHAR) @20; EX adds service pack major/minor @276/278, suite @280, product
/// type @282. The A forms have a 128-byte CHAR szCSDVersion (EX fields @148).
unsafe fn fill(info: *mut u8, wide: bool) -> bool {
    if info.is_null() {
        return false;
    }
    let size = unsafe { info.cast::<u32>().read_unaligned() } as usize;
    let (plain, extended) = if wide { (276, 284) } else { (148, 156) };
    if size != plain && size != extended {
        return false;
    }
    unsafe {
        info.add(4).write_bytes(0, size - 4);
        for (i, v) in [MAJOR, MINOR, BUILD, PLATFORM].iter().enumerate() {
            info.add(4 + 4 * i).cast::<u32>().write_unaligned(*v);
        }
        if size == extended {
            let at = info.add(plain);
            at.add(4).cast::<u16>().write_unaligned(SUITE);
            *at.add(6) = PRODUCT_TYPE;
        }
    }
    true
}
pub unsafe extern "win64" fn get_version_ex_w(info: *mut u8) -> i32 {
    if unsafe { fill(info, true) } {
        1
    } else {
        error::set(122);
        0
    }
}
pub unsafe extern "win64" fn get_version_ex_a(info: *mut u8) -> i32 {
    if unsafe { fill(info, false) } {
        1
    } else {
        error::set(122);
        0
    }
}
/// RtlGetVersion: NTSTATUS.
pub unsafe extern "win64" fn rtl_get_version(info: *mut u8) -> i32 {
    if unsafe { fill(info, true) } {
        0
    } else {
        0xC000_000Du32 as i32
    }
}
const VER_MINORVERSION: u32 = 0x01;
const VER_MAJORVERSION: u32 = 0x02;
const VER_BUILDNUMBER: u32 = 0x04;
const VER_PLATFORMID: u32 = 0x08;
const VER_SERVICEPACKMINOR: u32 = 0x10;
const VER_SERVICEPACKMAJOR: u32 = 0x20;
const VER_SUITENAME: u32 = 0x40;
const VER_PRODUCT_TYPE: u32 = 0x80;
/// VerSetConditionMask: the condition goes to the highest field named in the mask
/// (three bits per field), as Windows does.
pub extern "win64" fn ver_set_condition_mask(mask: u64, fields: u32, condition: u8) -> u64 {
    let condition = (condition & 7) as u64;
    if fields == 0 || condition == 0 {
        return mask;
    }
    let index = 31 - (fields & 0xff).leading_zeros().min(31);
    mask | condition << (index * 3)
}
fn compare(have: u64, want: u64, condition: u64) -> bool {
    match condition {
        1 => have == want,
        2 => have > want,
        3 => have >= want,
        4 => have < want,
        5 => have <= want,
        _ => false,
    }
}
fn field_condition(mask: u64, field: u32) -> u64 {
    mask >> (field.trailing_zeros() * 3) & 7
}
/// VerifyVersionInfo against the reported version. Major, minor and service pack
/// compare together (with the condition of the most significant one named).
unsafe fn verify(info: *const u8, fields: u32, mask: u64, wide: bool) -> Result<bool, u32> {
    if info.is_null() || fields == 0 || fields & !0xff != 0 {
        return Err(87);
    }
    let read = |at: usize| unsafe { info.add(at).cast::<u32>().read_unaligned() };
    let base = if wide { 276 } else { 148 };
    let read16 = |at: usize| unsafe { info.add(base + at).cast::<u16>().read_unaligned() };
    let product = unsafe { *info.add(base + 6) };
    let mut ok = true;
    let parts = [
        (VER_MAJORVERSION, read(4) as u64, MAJOR as u64),
        (VER_MINORVERSION, read(8) as u64, MINOR as u64),
        (VER_SERVICEPACKMAJOR, read16(0) as u64, 0),
        (VER_SERVICEPACKMINOR, read16(2) as u64, 0),
    ];
    if let Some(&(first, _, _)) = parts.iter().find(|(f, _, _)| fields & f != 0) {
        let condition = field_condition(mask, first);
        let (mut have, mut want) = (0u64, 0u64);
        for (f, w, h) in parts.iter().filter(|(f, _, _)| fields & f != 0) {
            let _ = f;
            have = have << 16 | h;
            want = want << 16 | w;
        }
        ok &= compare(have, want, condition);
    }
    if fields & VER_BUILDNUMBER != 0 {
        ok &= compare(
            BUILD as u64,
            read(12) as u64,
            field_condition(mask, VER_BUILDNUMBER),
        );
    }
    if fields & VER_PLATFORMID != 0 {
        ok &= compare(
            PLATFORM as u64,
            read(16) as u64,
            field_condition(mask, VER_PLATFORMID),
        );
    }
    if fields & VER_PRODUCT_TYPE != 0 {
        ok &= compare(
            PRODUCT_TYPE as u64,
            product as u64,
            field_condition(mask, VER_PRODUCT_TYPE),
        );
    }
    if fields & VER_SUITENAME != 0 {
        let want = read16(4);
        ok &= match field_condition(mask, VER_SUITENAME) {
            6 => SUITE & want == want, // VER_AND
            7 => SUITE & want != 0,    // VER_OR
            _ => return Err(87),
        };
    }
    Ok(ok)
}
fn verdict(result: Result<bool, u32>) -> i32 {
    match result {
        Ok(true) => 1,
        Ok(false) => {
            error::set(1150); // ERROR_OLD_WIN_VERSION
            0
        }
        Err(code) => {
            error::set(code);
            0
        }
    }
}
pub unsafe extern "win64" fn verify_version_info_w(info: *const u8, fields: u32, mask: u64) -> i32 {
    verdict(unsafe { verify(info, fields, mask, true) })
}
pub unsafe extern "win64" fn verify_version_info_a(info: *const u8, fields: u32, mask: u64) -> i32 {
    verdict(unsafe { verify(info, fields, mask, false) })
}
/// RtlVerifyVersionInfo: NTSTATUS (STATUS_REVISION_MISMATCH when older).
pub unsafe extern "win64" fn rtl_verify_version_info(
    info: *const u8,
    fields: u32,
    mask: u64,
) -> i32 {
    match unsafe { verify(info, fields, mask, true) } {
        Ok(true) => 0,
        Ok(false) => 0xC000_0059u32 as i32,
        Err(_) => 0xC000_000Du32 as i32,
    }
}

/// The ExW variants need a real PE VERSION resource. Do not synthesize a
/// Windows 10 version for arbitrary files; report unsupported extraction.
pub unsafe extern "win64" fn get_file_version_info_size_ex_w(
    flags: u32, filename: *const u16, handle: *mut u32,
) -> u32 {
    if !handle.is_null() { unsafe { *handle = 0 }; }
    if filename.is_null() || flags & !0x7 != 0 {
        error::set(87); // ERROR_INVALID_PARAMETER
    } else {
        error::set(50); // ERROR_NOT_SUPPORTED
    }
    0
}
pub unsafe extern "win64" fn get_file_version_info_ex_w(
    flags: u32, filename: *const u16, _handle: u32, _len: u32, _data: *mut u8,
) -> i32 {
    if filename.is_null() || flags & !0x7 != 0 {
        error::set(87); // ERROR_INVALID_PARAMETER
    } else {
        error::set(50); // ERROR_NOT_SUPPORTED
    }
    0
}

pub unsafe extern "win64" fn get_file_version_info_size_w(
    filename: *const u16,
    handle: *mut u32,
) -> u32 {
    if filename.is_null() {
        error::set(87);
        return 0;
    }
    if !handle.is_null() {
        unsafe { *handle = 0 };
    }
    512
}

pub unsafe extern "win64" fn get_file_version_info_size_a(
    filename: *const std::ffi::c_char,
    handle: *mut u32,
) -> u32 {
    if filename.is_null() {
        error::set(87);
        return 0;
    }
    if !handle.is_null() {
        unsafe { *handle = 0 };
    }
    512
}

pub unsafe extern "win64" fn get_file_version_info_w(
    filename: *const u16,
    _handle: u32,
    len: u32,
    data: *mut u8,
) -> i32 {
    if filename.is_null() || data.is_null() || len < 52 {
        error::set(87);
        return 0;
    }
    unsafe {
        std::ptr::write_bytes(data, 0, len as usize);
        (data as *mut u32).write_unaligned(0xFEEF04BD); // dwSignature
        (data.add(4) as *mut u32).write_unaligned(0x00010000); // dwStrucVersion
        (data.add(8) as *mut u32).write_unaligned(0x000a0000); // dwFileVersionMS (10.0)
        (data.add(12) as *mut u32).write_unaligned(0x00004a65); // dwFileVersionLS (19045)
        (data.add(16) as *mut u32).write_unaligned(0x000a0000); // dwProductVersionMS
        (data.add(20) as *mut u32).write_unaligned(0x00004a65); // dwProductVersionLS
        (data.add(24) as *mut u32).write_unaligned(0x3F); // dwFileFlagsMask
        (data.add(28) as *mut u32).write_unaligned(0); // dwFileFlags
        (data.add(32) as *mut u32).write_unaligned(0x00040004); // dwFileOS (VOS_NT_WINDOWS32)
        (data.add(36) as *mut u32).write_unaligned(0x00000001); // dwFileType (VFT_APP)
        (data.add(40) as *mut u32).write_unaligned(0); // dwFileSubtype
        (data.add(44) as *mut u32).write_unaligned(0); // dwFileDateMS
        (data.add(48) as *mut u32).write_unaligned(0); // dwFileDateLS
    }
    1
}

pub unsafe extern "win64" fn get_file_version_info_a(
    filename: *const std::ffi::c_char,
    handle: u32,
    len: u32,
    data: *mut u8,
) -> i32 {
    let mut wide = Vec::new();
    if !filename.is_null() {
        if let Ok(s) = unsafe { std::ffi::CStr::from_ptr(filename) }.to_str() {
            wide = s.encode_utf16().chain(std::iter::once(0)).collect();
        }
    }
    unsafe { get_file_version_info_w(if wide.is_empty() { std::ptr::null() } else { wide.as_ptr() }, handle, len, data) }
}

static VERSION_STR_W: &[u16] = &[
    b'1' as u16, b'0' as u16, b'.' as u16, b'0' as u16, b'.' as u16,
    b'1' as u16, b'9' as u16, b'0' as u16, b'4' as u16, b'5' as u16,
    b'.' as u16, b'0' as u16, 0
];
static VERSION_STR_A: &[u8] = b"10.0.19045.0\0";

pub unsafe extern "win64" fn ver_query_value_w(
    block: *const u8,
    sub_block: *const u16,
    buffer: *mut *const std::ffi::c_void,
    len: *mut u32,
) -> i32 {
    if block.is_null() || sub_block.is_null() || buffer.is_null() || len.is_null() {
        error::set(87);
        return 0;
    }
    let sub = unsafe { crate::api::text::string(sub_block) }.unwrap_or_default();
    if sub.is_empty() || sub == "\\" {
        unsafe {
            *buffer = block as *const std::ffi::c_void;
            *len = 52;
        }
        return 1;
    }
    unsafe {
        *buffer = VERSION_STR_W.as_ptr() as *const std::ffi::c_void;
        *len = VERSION_STR_W.len() as u32;
    }
    1
}

pub unsafe extern "win64" fn ver_query_value_a(
    block: *const u8,
    sub_block: *const std::ffi::c_char,
    buffer: *mut *const std::ffi::c_void,
    len: *mut u32,
) -> i32 {
    if block.is_null() || sub_block.is_null() || buffer.is_null() || len.is_null() {
        error::set(87);
        return 0;
    }
    let sub = if !sub_block.is_null() {
        unsafe { std::ffi::CStr::from_ptr(sub_block) }.to_str().unwrap_or("")
    } else {
        ""
    };
    if sub.is_empty() || sub == "\\" {
        unsafe {
            *buffer = block as *const std::ffi::c_void;
            *len = 52;
        }
        return 1;
    }
    unsafe {
        *buffer = VERSION_STR_A.as_ptr() as *const std::ffi::c_void;
        *len = VERSION_STR_A.len() as u32;
    }
    1
}

