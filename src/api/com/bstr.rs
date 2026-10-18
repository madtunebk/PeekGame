//! OLEAUT32 BSTRs: a 32-bit byte length, then the UTF-16 text and a NUL; the
//! BSTR points at the text. VARIANTs are initialized and cleared (BSTR members
//! freed).
use std::ffi::c_void;
unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}
unsafe fn allocate(bytes: u32) -> *mut u16 {
    let base = unsafe { malloc(bytes as usize + 6) }.cast::<u8>();
    if base.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        base.cast::<u32>().write_unaligned(bytes);
        base.add(4 + bytes as usize)
            .cast::<u16>()
            .write_unaligned(0);
        base.add(4).cast()
    }
}
pub unsafe extern "win64" fn alloc_string_len(text: *const u16, length: u32) -> *mut u16 {
    let s = unsafe { allocate(length * 2) };
    if !s.is_null() {
        unsafe {
            if text.is_null() {
                s.write_bytes(0, length as usize);
            } else {
                std::ptr::copy_nonoverlapping(text, s, length as usize);
            }
        }
    }
    s
}
pub unsafe extern "win64" fn alloc_string(text: *const u16) -> *mut u16 {
    if text.is_null() {
        return std::ptr::null_mut();
    }
    let n = unsafe { crate::api::text::units(text) }.len() as u32;
    unsafe { alloc_string_len(text, n) }
}
pub unsafe extern "win64" fn alloc_string_byte_len(data: *const u8, length: u32) -> *mut u16 {
    let s = unsafe { allocate(length) };
    if !s.is_null() {
        unsafe {
            let d = s.cast::<u8>();
            if data.is_null() {
                d.write_bytes(0, length as usize);
            } else {
                std::ptr::copy_nonoverlapping(data, d, length as usize);
            }
            *d.add(length as usize) = 0;
        }
    }
    s
}
pub unsafe extern "win64" fn free_string(s: *mut u16) {
    if !s.is_null() {
        unsafe { free(s.cast::<u8>().sub(4).cast()) };
    }
}

/// QueryPathOfRegTypeLib: retrieves the path of a registered type library.
/// When not registered, returns TYPE_E_LIBNOTREGISTERED (0x8002801D).
pub unsafe extern "win64" fn query_path_of_reg_type_lib(
    _guid: *const u8,
    _ver_major: u16,
    _ver_minor: u16,
    _lcid: u32,
    path_name: *mut *mut u16,
) -> i32 {
    const TYPE_E_LIBNOTREGISTERED: i32 = 0x8002801Du32 as i32;
    const E_INVALIDARG: i32 = 0x80070057u32 as i32;
    if path_name.is_null() {
        return E_INVALIDARG;
    }
    unsafe { *path_name = std::ptr::null_mut() };
    TYPE_E_LIBNOTREGISTERED
}

pub unsafe extern "win64" fn register_type_lib(
    _ptlib: *const std::ffi::c_void,
    _sz_full_path: *const u16,
    _sz_help_dir: *const u16,
) -> i32 {
    0 // S_OK
}

pub unsafe extern "win64" fn unregister_type_lib(
    _lib_id: *const u8,
    _ver_major: u16,
    _ver_minor: u16,
    _lcid: u32,
    _syskind: u32,
) -> i32 {
    0 // S_OK
}

pub unsafe extern "win64" fn register_type_lib_for_user(
    _ptlib: *const std::ffi::c_void,
    _sz_full_path: *const u16,
    _sz_help_dir: *const u16,
) -> i32 {
    0 // S_OK
}

pub unsafe extern "win64" fn unregister_type_lib_for_user(
    _lib_id: *const u8,
    _ver_major: u16,
    _ver_minor: u16,
    _lcid: u32,
    _syskind: u32,
) -> i32 {
    0 // S_OK
}

pub unsafe extern "win64" fn load_reg_type_lib(
    _rguid: *const u8,
    _ver_major: u16,
    _ver_minor: u16,
    _lcid: u32,
    pptlib: *mut *mut std::ffi::c_void,
) -> i32 {
    if !pptlib.is_null() {
        unsafe { *pptlib = std::ptr::null_mut() };
    }
    0x8002801Du32 as i32 // TYPE_E_LIBNOTREGISTERED
}

pub unsafe extern "win64" fn load_type_lib(
    _sz_file: *const u16,
    pptlib: *mut *mut std::ffi::c_void,
) -> i32 {
    if !pptlib.is_null() {
        unsafe { *pptlib = std::ptr::null_mut() };
    }
    0x80029C4Au32 as i32 // TYPE_E_CANTLOADLIBRARY
}

pub unsafe extern "win64" fn load_type_lib_ex(
    _sz_file: *const u16,
    _regkind: u32,
    pptlib: *mut *mut std::ffi::c_void,
) -> i32 {
    if !pptlib.is_null() {
        unsafe { *pptlib = std::ptr::null_mut() };
    }
    0x80029C4Au32 as i32 // TYPE_E_CANTLOADLIBRARY
}

pub unsafe extern "win64" fn string_byte_len(s: *const u16) -> u32 {
    if s.is_null() {
        0
    } else {
        unsafe { s.cast::<u8>().sub(4).cast::<u32>().read_unaligned() }
    }
}
pub unsafe extern "win64" fn string_len(s: *const u16) -> u32 {
    (unsafe { string_byte_len(s) }) / 2
}
pub unsafe extern "win64" fn realloc_string_len(
    out: *mut *mut u16,
    text: *const u16,
    length: u32,
) -> i32 {
    if out.is_null() {
        return 0;
    }
    let s = unsafe { alloc_string_len(text, length) };
    if s.is_null() {
        return 0;
    }
    unsafe {
        free_string(*out);
        *out = s;
    }
    1
}
pub unsafe extern "win64" fn realloc_string(out: *mut *mut u16, text: *const u16) -> i32 {
    let n = if text.is_null() {
        0
    } else {
        unsafe { crate::api::text::units(text) }.len() as u32
    };
    unsafe { realloc_string_len(out, text, n) }
}
/// VARIANT: vt (u16) and the value at 8.
pub unsafe extern "win64" fn variant_init(v: *mut u8) {
    if !v.is_null() {
        unsafe { v.write_bytes(0, 24) };
    }
}
pub unsafe extern "win64" fn variant_clear(v: *mut u8) -> i32 {
    if v.is_null() {
        return 0x8007_0057u32 as i32;
    }
    unsafe {
        if v.cast::<u16>().read_unaligned() == 8 {
            // VT_BSTR
            free_string(v.add(8).cast::<*mut u16>().read_unaligned());
        }
        v.write_bytes(0, 24);
    }
    0
}

pub unsafe extern "win64" fn variant_copy(dest: *mut u8, src: *const u8) -> i32 {
    if dest.is_null() || src.is_null() {
        return 0x8007_0057u32 as i32;
    }
    if dest as *const u8 == src {
        return 0;
    }
    unsafe {
        let _ = variant_clear(dest);
        std::ptr::copy_nonoverlapping(src, dest, 24);
        if src.cast::<u16>().read_unaligned() == 8 {
            let b = src.add(8).cast::<*const u16>().read_unaligned();
            if !b.is_null() {
                let bytes = string_byte_len(b);
                let copy = alloc_string_byte_len(b.cast(), bytes);
                dest.add(8).cast::<*mut u16>().write_unaligned(copy);
            }
        }
    }
    0
}

pub unsafe extern "win64" fn variant_copy_ind(dest: *mut u8, src: *const u8) -> i32 {
    unsafe { variant_copy(dest, src) }
}

pub unsafe extern "win64" fn variant_change_type(
    dest: *mut u8,
    src: *const u8,
    _flags: u16,
    vt: u16,
) -> i32 {
    if dest.is_null() || src.is_null() {
        return 0x8007_0057u32 as i32;
    }
    unsafe {
        let src_vt = src.cast::<u16>().read_unaligned();
        if src_vt == vt {
            return variant_copy(dest, src);
        }
        let _ = variant_clear(dest);
        dest.cast::<u16>().write_unaligned(vt);
    }
    0
}

pub unsafe extern "win64" fn system_time_to_variant_time(
    _st: *const u8,
    pvtime: *mut f64,
) -> i32 {
    if !pvtime.is_null() {
        unsafe { *pvtime = 45000.0 };
    }
    1
}

pub unsafe extern "win64" fn variant_time_to_system_time(
    _vtime: f64,
    st: *mut u16,
) -> i32 {
    if !st.is_null() {
        unsafe {
            st.write_bytes(0, 8);
            *st = 2026;
            *st.add(1) = 1;
            *st.add(3) = 1;
        }
    }
    1
}

pub unsafe extern "win64" fn var_bstr_cmp(
    left: *const u16,
    right: *const u16,
    _lcid: u32,
    flags: u32,
) -> i32 {
    let l_len = (unsafe { string_byte_len(left) } / 2) as usize;
    let r_len = (unsafe { string_byte_len(right) } / 2) as usize;
    let l_slice = if left.is_null() || l_len == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(left, l_len) }
    };
    let r_slice = if right.is_null() || r_len == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(right, r_len) }
    };
    let ord = if (flags & 1) != 0 {
        // NORM_IGNORECASE
        l_slice
            .iter()
            .map(|&c| {
                if (b'A' as u16..=b'Z' as u16).contains(&c) {
                    c + 32
                } else {
                    c
                }
            })
            .cmp(r_slice.iter().map(|&c| {
                if (b'A' as u16..=b'Z' as u16).contains(&c) {
                    c + 32
                } else {
                    c
                }
            }))
    } else {
        l_slice.cmp(r_slice)
    };
    match ord {
        std::cmp::Ordering::Less => 0,    // VARCMP_LT
        std::cmp::Ordering::Equal => 1,   // VARCMP_EQ
        std::cmp::Ordering::Greater => 2, // VARCMP_GT
    }
}

pub unsafe extern "win64" fn var_cmp(
    left: *const u8,
    right: *const u8,
    lcid: u32,
    flags: u32,
) -> i32 {
    if left.is_null() || right.is_null() {
        return 3; // VARCMP_NULL
    }
    unsafe {
        let l_vt = left.cast::<u16>().read_unaligned();
        let r_vt = right.cast::<u16>().read_unaligned();
        if l_vt == 8 && r_vt == 8 {
            return var_bstr_cmp(
                left.add(8).cast::<*const u16>().read_unaligned(),
                right.add(8).cast::<*const u16>().read_unaligned(),
                lcid,
                flags,
            );
        }
    }
    1 // VARCMP_EQ
}

