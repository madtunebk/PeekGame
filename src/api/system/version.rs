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
