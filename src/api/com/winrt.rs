//! WinRT (combase.dll, api-ms-win-core-winrt-*), a few COM helpers and UI
//! Automation without clients (ported from peek's ole32.inc). There is no Windows
//! Runtime: activation fails as for a class that is not installed.
//!
//! HSTRING uses Windows' header layout: flags @0, length @4 (UTF-16 units), text
//! pointer @16. A reference string (WindowsCreateStringReference, flags 1) lives in
//! the caller's 24-byte header; an owned string is one allocation with a reference
//! count @24 and the NUL-terminated text @32. The empty string is NULL.
use super::apartment;
use std::{
    ffi::c_void,
    sync::atomic::{AtomicU32, Ordering},
};
const S_OK: i32 = 0;
const E_INVALIDARG: i32 = 0x80070057u32 as i32;
const E_POINTER: i32 = 0x80004003u32 as i32;
const E_OUTOFMEMORY: i32 = 0x8007000Eu32 as i32;
const E_NOTIMPL: i32 = 0x80004001u32 as i32;
const E_FAIL: i32 = 0x80004005u32 as i32;
const REGDB_E_CLASSNOTREG: i32 = 0x80040154u32 as i32;

#[repr(C)]
pub(crate) struct Header {
    flags: u32,
    length: u32,
    _reserved: [u32; 2],
    text: *const u16,
}
#[repr(C)]
pub(crate) struct Owned {
    header: Header,
    refs: AtomicU32,
    _padding: u32,
    text: [u16; 0],
}
unsafe extern "C" {
    fn malloc(n: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn getrandom(buffer: *mut u8, length: usize, flags: u32) -> isize;
}

pub unsafe extern "win64" fn create_string(
    text: *const u16,
    length: u32,
    out: *mut *mut Header,
) -> i32 {
    if out.is_null() {
        return E_INVALIDARG;
    }
    unsafe { *out = std::ptr::null_mut() };
    if length == 0 {
        return S_OK;
    }
    if text.is_null() {
        return E_POINTER;
    }
    let owned = unsafe { malloc(size_of::<Owned>() + (length as usize + 1) * 2) }.cast::<Owned>();
    if owned.is_null() {
        return E_OUTOFMEMORY;
    }
    unsafe {
        let data = (owned as *mut u8).add(size_of::<Owned>()).cast::<u16>();
        std::ptr::copy_nonoverlapping(text, data, length as usize);
        *data.add(length as usize) = 0;
        owned.write(Owned {
            header: Header {
                flags: 0,
                length,
                _reserved: [0; 2],
                text: data,
            },
            refs: AtomicU32::new(1),
            _padding: 0,
            text: [],
        });
        *out = owned.cast();
    }
    S_OK
}
pub unsafe extern "win64" fn create_string_reference(
    text: *const u16,
    length: u32,
    header: *mut Header,
    out: *mut *mut Header,
) -> i32 {
    if out.is_null() || header.is_null() {
        return E_INVALIDARG;
    }
    unsafe { *out = std::ptr::null_mut() };
    if length == 0 {
        return S_OK;
    }
    // The text must be NUL-terminated at `length`.
    if text.is_null() || unsafe { *text.add(length as usize) } != 0 {
        return E_INVALIDARG;
    }
    unsafe {
        header.write(Header {
            flags: 1,
            length,
            _reserved: [0; 2],
            text,
        });
        *out = header;
    }
    S_OK
}
pub unsafe extern "win64" fn delete_string(h: *mut Header) -> i32 {
    if h.is_null() || unsafe { (*h).flags } & 1 != 0 {
        return S_OK;
    }
    let owned = h.cast::<Owned>();
    if unsafe { &(*owned).refs }.fetch_sub(1, Ordering::AcqRel) == 1 {
        unsafe { free(owned.cast()) };
    }
    S_OK
}
pub unsafe extern "win64" fn duplicate_string(h: *mut Header, out: *mut *mut Header) -> i32 {
    if out.is_null() {
        return E_INVALIDARG;
    }
    unsafe { *out = std::ptr::null_mut() };
    if h.is_null() {
        return S_OK;
    }
    if unsafe { (*h).flags } & 1 != 0 {
        return unsafe { create_string((*h).text, (*h).length, out) };
    }
    unsafe {
        (*h.cast::<Owned>()).refs.fetch_add(1, Ordering::Relaxed);
        *out = h;
    }
    S_OK
}
static EMPTY: [u16; 1] = [0];
pub unsafe extern "win64" fn get_string_raw_buffer(
    h: *const Header,
    length: *mut u32,
) -> *const u16 {
    if !length.is_null() {
        unsafe { *length = if h.is_null() { 0 } else { (*h).length } };
    }
    if h.is_null() {
        EMPTY.as_ptr()
    } else {
        unsafe { (*h).text }
    }
}
pub unsafe extern "win64" fn get_string_len(h: *const Header) -> u32 {
    if h.is_null() {
        0
    } else {
        unsafe { (*h).length }
    }
}
pub unsafe extern "win64" fn is_string_empty(h: *const Header) -> i32 {
    (h.is_null() || unsafe { (*h).length } == 0) as i32
}
unsafe fn units<'a>(h: *const Header) -> &'a [u16] {
    let mut length = 0;
    let text = unsafe { get_string_raw_buffer(h, &mut length) };
    unsafe { std::slice::from_raw_parts(text, length as usize) }
}
pub unsafe extern "win64" fn compare_string_ordinal(
    a: *const Header,
    b: *const Header,
    result: *mut i32,
) -> i32 {
    if result.is_null() {
        return E_INVALIDARG;
    }
    unsafe { *result = units(a).cmp(units(b)) as i32 };
    S_OK
}
pub unsafe extern "win64" fn ro_get_activation_factory(
    class: *const Header,
    _iid: *const u8,
    factory: *mut *mut c_void,
) -> i32 {
    if !factory.is_null() {
        unsafe { *factory = std::ptr::null_mut() };
    }
    let name = String::from_utf16_lossy(unsafe { units(class) });
    eprintln!("RoGetActivationFactory({name}): no Windows Runtime, REGDB_E_CLASSNOTREG");
    REGDB_E_CLASSNOTREG
}
pub unsafe extern "win64" fn ro_activate_instance(
    class: *const Header,
    instance: *mut *mut c_void,
) -> i32 {
    unsafe { ro_get_activation_factory(class, std::ptr::null(), instance) }
}
/// RoInitialize: RO_INIT_SINGLETHREADED (0) is an STA, RO_INIT_MULTITHREADED (1) the MTA.
pub extern "win64" fn ro_initialize(kind: u32) -> i32 {
    if kind > 1 {
        return E_INVALIDARG;
    }
    apartment::initialize(std::ptr::null_mut(), if kind == 1 { 0 } else { 2 })
}
pub extern "win64" fn co_initialize(reserved: *mut c_void) -> i32 {
    apartment::initialize(reserved, 2)
}
/// StringFromGUID2: "{XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}", 39 units with the NUL.
pub unsafe extern "win64" fn string_from_guid2(
    guid: *const u8,
    out: *mut u16,
    capacity: i32,
) -> i32 {
    if guid.is_null() || out.is_null() || capacity < 39 {
        return 0;
    }
    let g = unsafe { std::slice::from_raw_parts(guid, 16) };
    let text = format!(
        "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{}}}",
        u32::from_le_bytes(g[0..4].try_into().unwrap()),
        u16::from_le_bytes(g[4..6].try_into().unwrap()),
        u16::from_le_bytes(g[6..8].try_into().unwrap()),
        g[8],
        g[9],
        g[10..]
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<String>()
    );
    for (i, u) in text.encode_utf16().chain(Some(0)).enumerate() {
        unsafe { *out.add(i) = u };
    }
    39
}
/// CoCreateGuid: a random (version 4) GUID.
pub unsafe extern "win64" fn co_create_guid(guid: *mut u8) -> i32 {
    if guid.is_null() {
        return E_INVALIDARG;
    }
    if unsafe { getrandom(guid, 16, 0) } != 16 {
        return E_FAIL;
    }
    unsafe {
        *guid.add(7) = (*guid.add(7) & 0x0f) | 0x40;
        *guid.add(8) = (*guid.add(8) & 0x3f) | 0x80;
    }
    S_OK
}

// UI Automation without clients: no screen reader can connect, so raising events is
// the documented no-listener no-op and nothing is ever listening.
pub extern "win64" fn uia_host_provider_from_hwnd(_hwnd: usize, out: *mut usize) -> i32 {
    if !out.is_null() {
        unsafe { *out = 0 };
    }
    E_NOTIMPL
}
pub extern "win64" fn uia_raise_event(_provider: usize, _id: i32) -> i32 {
    S_OK
}
pub extern "win64" fn uia_raise_property_changed(
    _provider: usize,
    _id: i32,
    _old: usize,
    _new: usize,
) -> i32 {
    S_OK
}
pub extern "win64" fn uia_raise_structure_changed(
    _provider: usize,
    _kind: i32,
    _id: usize,
    _count: i32,
) -> i32 {
    S_OK
}
pub extern "win64" fn uia_raise_notification(
    _provider: usize,
    _kind: i32,
    _processing: i32,
    _text: usize,
    _id: usize,
) -> i32 {
    S_OK
}
pub extern "win64" fn uia_clients_are_listening() -> i32 {
    0
}
pub extern "win64" fn uia_return_raw_element_provider(
    _hwnd: usize,
    _wp: usize,
    _lp: isize,
    _provider: usize,
) -> isize {
    0
}
