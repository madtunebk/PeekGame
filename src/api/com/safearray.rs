//! OLEAUT32 1D SafeArray vector operations.
use std::ffi::c_void;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SafeArrayBound {
    pub c_elements: u32,
    pub l_lbound: i32,
}

#[repr(C)]
pub struct SafeArray {
    pub c_dims: u16,
    pub f_features: u16,
    pub cb_elements: u32,
    pub c_locks: u32,
    pub pv_data: *mut u8,
    pub rgsabound: [SafeArrayBound; 1],
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

fn element_size(vt: u16) -> u32 {
    match vt {
        16 | 17 => 1,       // VT_I1, VT_UI1
        2 | 18 | 11 => 2,    // VT_I2, VT_UI2, VT_BOOL
        3 | 19 | 4 | 22 | 23 | 10 => 4, // VT_I4, VT_UI4, VT_R4, VT_INT, VT_UINT, VT_ERROR
        20 | 21 | 5 | 6 | 7 => 8,       // VT_I8, VT_UI8, VT_R8, VT_CY, VT_DATE
        8 | 9 | 13 => 8,    // VT_BSTR, VT_DISPATCH, VT_UNKNOWN
        12 => 24,           // VT_VARIANT (24 bytes on x64)
        _ => 8,
    }
}

pub extern "win64" fn create_vector(vt: u16, l_bound: i32, c_elements: u32) -> *mut SafeArray {
    let cb = element_size(vt);
    let data_len = cb as usize * c_elements as usize;
    let total = size_of::<SafeArray>() + data_len;
    let ptr = unsafe { malloc(total) as *mut u8 };
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        ptr.write_bytes(0, total);
        let psa = ptr as *mut SafeArray;
        (*psa).c_dims = 1;
        (*psa).cb_elements = cb;
        (*psa).pv_data = ptr.add(size_of::<SafeArray>());
        (*psa).rgsabound[0] = SafeArrayBound {
            c_elements,
            l_lbound: l_bound,
        };
        psa
    }
}

pub unsafe extern "win64" fn destroy_vector(psa: *mut SafeArray) -> i32 {
    if psa.is_null() {
        return 0x80070057u32 as i32; // E_INVALIDARG
    }
    unsafe { free(psa as *mut c_void) };
    0
}

pub unsafe extern "win64" fn put_element(
    psa: *mut SafeArray,
    indices: *const i32,
    pv: *const c_void,
) -> i32 {
    if psa.is_null() || indices.is_null() || pv.is_null() {
        return 0x80070057u32 as i32; // E_INVALIDARG
    }
    let (c_elements, l_lbound) = unsafe {
        ((*psa).rgsabound[0].c_elements, (*psa).rgsabound[0].l_lbound)
    };
    let idx = unsafe { *indices };
    if idx < l_lbound || (idx - l_lbound) as u32 >= c_elements {
        return 0x8002000Bu32 as i32; // DISP_E_BADINDEX
    }
    let offset = (idx - l_lbound) as usize * unsafe { (*psa).cb_elements as usize };
    unsafe {
        std::ptr::copy_nonoverlapping(
            pv as *const u8,
            (*psa).pv_data.add(offset),
            (*psa).cb_elements as usize,
        );
    }
    0
}

pub unsafe extern "win64" fn access_data(psa: *mut SafeArray, ppv_data: *mut *mut c_void) -> i32 {
    if psa.is_null() || ppv_data.is_null() {
        return 0x80070057u32 as i32; // E_INVALIDARG
    }
    unsafe {
        (*psa).c_locks += 1;
        *ppv_data = (*psa).pv_data as *mut c_void;
    }
    0
}

pub unsafe extern "win64" fn unaccess_data(psa: *mut SafeArray) -> i32 {
    if psa.is_null() {
        return 0x80070057u32 as i32; // E_INVALIDARG
    }
    unsafe {
        if (*psa).c_locks > 0 {
            (*psa).c_locks -= 1;
        }
    }
    0
}

pub unsafe extern "win64" fn get_dim(psa: *mut SafeArray) -> u32 {
    if psa.is_null() {
        0
    } else {
        unsafe { (*psa).c_dims as u32 }
    }
}

pub unsafe extern "win64" fn get_elemsize(psa: *mut SafeArray) -> u32 {
    if psa.is_null() {
        0
    } else {
        unsafe { (*psa).cb_elements }
    }
}

pub unsafe extern "win64" fn get_lbound(psa: *mut SafeArray, _dim: u32, lbound: *mut i32) -> i32 {
    if psa.is_null() || lbound.is_null() {
        return 0x80070057u32 as i32;
    }
    unsafe { *lbound = (*psa).rgsabound[0].l_lbound };
    0
}

pub unsafe extern "win64" fn get_ubound(psa: *mut SafeArray, _dim: u32, ubound: *mut i32) -> i32 {
    if psa.is_null() || ubound.is_null() {
        return 0x80070057u32 as i32;
    }
    unsafe {
        *ubound = (*psa).rgsabound[0].l_lbound + (*psa).rgsabound[0].c_elements as i32 - 1;
    }
    0
}

pub unsafe extern "win64" fn lock(psa: *mut SafeArray) -> i32 {
    if psa.is_null() {
        return 0x80070057u32 as i32;
    }
    unsafe { (*psa).c_locks += 1 };
    0
}

pub unsafe extern "win64" fn unlock(psa: *mut SafeArray) -> i32 {
    if psa.is_null() {
        return 0x80070057u32 as i32;
    }
    unsafe {
        if (*psa).c_locks > 0 {
            (*psa).c_locks -= 1;
        }
    }
    0
}

pub unsafe extern "win64" fn get_element(
    psa: *mut SafeArray,
    indices: *const i32,
    pv: *mut c_void,
) -> i32 {
    if psa.is_null() || indices.is_null() || pv.is_null() {
        return 0x80070057u32 as i32;
    }
    let (c_elements, l_lbound) = unsafe {
        ((*psa).rgsabound[0].c_elements, (*psa).rgsabound[0].l_lbound)
    };
    let idx = unsafe { *indices };
    if idx < l_lbound || (idx - l_lbound) as u32 >= c_elements {
        return 0x8002000Bu32 as i32; // DISP_E_BADINDEX
    }
    let offset = (idx - l_lbound) as usize * unsafe { (*psa).cb_elements as usize };
    unsafe {
        std::ptr::copy_nonoverlapping(
            (*psa).pv_data.add(offset),
            pv as *mut u8,
            (*psa).cb_elements as usize,
        );
    }
    0
}

pub unsafe extern "win64" fn create(
    vt: u16,
    dims: u32,
    rgsabound: *const SafeArrayBound,
) -> *mut SafeArray {
    if dims != 1 || rgsabound.is_null() {
        return std::ptr::null_mut();
    }
    let bound = unsafe { *rgsabound };
    create_vector(vt, bound.l_lbound, bound.c_elements)
}

pub unsafe extern "win64" fn copy(psa: *mut SafeArray, ppsa_out: *mut *mut SafeArray) -> i32 {
    if ppsa_out.is_null() {
        return 0x80070057u32 as i32;
    }
    if psa.is_null() {
        unsafe { *ppsa_out = std::ptr::null_mut() };
        return 0;
    }
    unsafe {
        let cb = (*psa).cb_elements;
        let bound = (*psa).rgsabound[0];
        let data_len = cb as usize * bound.c_elements as usize;
        let total = size_of::<SafeArray>() + data_len;
        let ptr = malloc(total) as *mut u8;
        if ptr.is_null() {
            return 0x8007000Eu32 as i32; // E_OUTOFMEMORY
        }
        std::ptr::copy_nonoverlapping(psa as *const u8, ptr, total);
        let out = ptr as *mut SafeArray;
        (*out).c_locks = 0;
        (*out).pv_data = ptr.add(size_of::<SafeArray>());
        *ppsa_out = out;
    }
    0
}

pub unsafe extern "win64" fn get_vartype(psa: *mut SafeArray, pvt: *mut u16) -> i32 {
    if psa.is_null() || pvt.is_null() {
        return 0x80070057u32 as i32;
    }
    let vt = match unsafe { (*psa).cb_elements } {
        1 => 17, // VT_UI1
        2 => 2,  // VT_I2
        4 => 3,  // VT_I4
        24 => 12, // VT_VARIANT
        _ => 8,  // VT_BSTR / 8-byte
    };
    unsafe { *pvt = vt };
    0
}

