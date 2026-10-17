//! PROPVARIANT ownership for scalar values, strings, blobs and COM interfaces.
use super::{apartment, bstr};
pub unsafe extern "win64" fn clear(v: *mut u8) -> i32 {
    if v.is_null() {
        return 0x80070057u32 as i32;
    }
    let tag = unsafe { v.cast::<u16>().read_unaligned() };
    let data = || unsafe { v.add(8).cast::<*mut std::ffi::c_void>().read_unaligned() };
    match tag {
        0..=7 | 10 | 11 | 14 | 16..=23 | 64 => {}
        8 => unsafe { bstr::free_string(data().cast()) },
        30 | 31 | 72 => unsafe { apartment::release(data()) },
        65 => unsafe {
            apartment::release(v.add(16).cast::<*mut std::ffi::c_void>().read_unaligned())
        },
        9 | 13 => {
            if !data().is_null() {
                let data = data();
                unsafe {
                    let vt = *(data as *const *const usize);
                    let release: extern "win64" fn(*mut std::ffi::c_void) -> u32 =
                        std::mem::transmute(*vt.add(2));
                    release(data);
                }
            }
        }
        _ => return 0x80020008u32 as i32, // DISP_E_BADVARTYPE: ownership not implemented
    }
    unsafe { v.write_bytes(0, 24) };
    0
}
