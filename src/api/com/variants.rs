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

pub unsafe extern "win64" fn copy(dest: *mut u8, src: *const u8) -> i32 {
    unsafe { bstr::variant_copy(dest, src) }
}

pub unsafe extern "win64" fn to_bstr(propvar: *const u8, pbstr_out: *mut *mut u16) -> i32 {
    if pbstr_out.is_null() {
        return 0x80070057u32 as i32;
    }
    if propvar.is_null() {
        let empty = [0u16; 1];
        unsafe { *pbstr_out = bstr::alloc_string(empty.as_ptr()) };
        return 0;
    }
    let vt = unsafe { propvar.cast::<u16>().read_unaligned() };
    if vt == 8 {
        let s = unsafe { propvar.add(8).cast::<*const u16>().read_unaligned() };
        unsafe { *pbstr_out = bstr::alloc_string(s) };
        return 0;
    }
    if vt == 31 {
        // VT_LPWSTR
        let s = unsafe { propvar.add(8).cast::<*const u16>().read_unaligned() };
        unsafe { *pbstr_out = bstr::alloc_string(s) };
        return 0;
    }
    let empty = [0u16; 1];
    unsafe { *pbstr_out = bstr::alloc_string(empty.as_ptr()) };
    0
}

pub unsafe extern "win64" fn init_from_clsid(clsid: *const u8, ppropvar: *mut u8) -> i32 {
    if clsid.is_null() || ppropvar.is_null() {
        return 0x80070057u32 as i32;
    }
    let mem = apartment::allocate(16) as *mut u8;
    if mem.is_null() {
        return 0x8007000Eu32 as i32;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(clsid, mem, 16);
        ppropvar.write_bytes(0, 24);
        ppropvar.cast::<u16>().write_unaligned(72); // VT_CLSID
        ppropvar.add(8).cast::<*mut u8>().write_unaligned(mem);
    }
    0
}

pub unsafe extern "win64" fn get_property_key_from_name(
    _psz_name: *const u16,
    pkey: *mut u8,
) -> i32 {
    if !pkey.is_null() {
        unsafe { pkey.write_bytes(0, 20) };
    }
    0x8002802Bu32 as i32 // TYPE_E_ELEMENTNOTFOUND
}

pub unsafe extern "win64" fn variant_compare(_var1: *const u8, _var2: *const u8) -> i32 {
    0
}

pub unsafe extern "win64" fn create_free_threaded_marshaler(
    _punk_outer: usize,
    ppunk_marshal: *mut usize,
) -> i32 {
    if !ppunk_marshal.is_null() {
        unsafe { *ppunk_marshal = 0 };
    }
    0x80004001u32 as i32 // E_NOTIMPL
}

