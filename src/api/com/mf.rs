//! Windows Media Foundation implementation (MFPlat.dll, MFReadWrite.dll).
//! Provides IMFAttributes, IMFMediaType, IMFByteStream, and IMFSourceReader.

use super::apartment;
use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};

const S_OK: i32 = 0;
const E_NOTIMPL: i32 = 0x80004001u32 as i32;
const E_NOINTERFACE: i32 = 0x80004002u32 as i32;
const E_POINTER: i32 = 0x80004003u32 as i32;
const E_INVALIDARG: i32 = 0x80070057u32 as i32;
const E_OUTOFMEMORY: i32 = 0x8007000Eu32 as i32;
const HRESULT_ERROR_INSUFFICIENT_BUFFER: i32 = 0x8007007Au32 as i32;

const MF_E_ATTRIBUTENOTFOUND: i32 = 0xC00D36E6u32 as i32;
const MF_E_INVALIDTYPE: i32 = 0xC00D36E5u32 as i32;
const MF_E_NO_MORE_TYPES: i32 = 0xC00D36B9u32 as i32;

const IID_IUNKNOWN: [u8; 16] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
];

// {2cd2d927-90e5-4ea6-a4cd-44b4e125ac40}
const IID_IMFATTRIBUTES: [u8; 16] = [
    0x27, 0xd9, 0xd2, 0x2c, 0xe5, 0x90, 0xa6, 0x4e,
    0xa4, 0xcd, 0x44, 0xb4, 0xe1, 0x25, 0xac, 0x40,
];

// {44ae0fa8-ea31-4109-8d2e-4cae4997c555}
const IID_IMFMEDIATYPE: [u8; 16] = [
    0xa8, 0x0f, 0xae, 0x44, 0x31, 0xea, 0x09, 0x41,
    0x8d, 0x2e, 0x4c, 0xae, 0x49, 0x97, 0xc5, 0x55,
];

// {ad4c1b00-4bf7-422f-9175-756693d9130d}
const IID_IMFBYTESTREAM: [u8; 16] = [
    0x00, 0x1b, 0x4c, 0xad, 0xf7, 0x4b, 0x2f, 0x42,
    0x91, 0x75, 0x75, 0x66, 0x93, 0xd9, 0x13, 0x0d,
];

// {70ae66f2-c809-4e4f-aa91-4ac6422b03f1}
const IID_IMFSOURCEREADER: [u8; 16] = [
    0xf2, 0x66, 0xae, 0x70, 0x09, 0xc8, 0x4f, 0x4a,
    0xaa, 0x91, 0x4a, 0xc6, 0x42, 0x2b, 0x03, 0xf1,
];

// MF_MT_MAJOR_TYPE: {48eba18e-f827-49ed-85de-9e1f52ab163e}
pub const MF_MT_MAJOR_TYPE: [u8; 16] = [
    0x8e, 0xa1, 0xeb, 0x48, 0x27, 0xf8, 0xed, 0x49,
    0x85, 0xde, 0x9e, 0x1f, 0x52, 0xab, 0x16, 0x3e,
];

// MFMediaType_Video: {73646976-0000-0010-8000-00aa00389b71}
pub const MF_MEDIA_TYPE_VIDEO: [u8; 16] = [
    0x76, 0x69, 0x64, 0x73, 0x00, 0x00, 0x10, 0x00,
    0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71,
];

#[derive(Clone, Debug, PartialEq)]
pub enum MFAttributeValue {
    Uint32(u32),
    Uint64(u64),
    Double(f64),
    Guid([u8; 16]),
    String(Vec<u16>),
    Blob(Vec<u8>),
    Unknown(*mut c_void),
}

unsafe fn write_propvariant(val: &MFAttributeValue, out: *mut u8) {
    unsafe { out.write_bytes(0, 24) };
    match val {
        MFAttributeValue::Uint32(v) => unsafe {
            (out as *mut u16).write(19); // VT_UI4
            (out.add(8) as *mut u32).write(*v);
        },
        MFAttributeValue::Uint64(v) => unsafe {
            (out as *mut u16).write(21); // VT_UI8
            (out.add(8) as *mut u64).write(*v);
        },
        MFAttributeValue::Double(v) => unsafe {
            (out as *mut u16).write(5); // VT_R8
            (out.add(8) as *mut f64).write(*v);
        },
        MFAttributeValue::Guid(g) => unsafe {
            (out as *mut u16).write(72); // VT_CLSID
            let mem = apartment::allocate(16) as *mut [u8; 16];
            if !mem.is_null() {
                mem.write(*g);
                (out.add(8) as *mut *mut [u8; 16]).write(mem);
            }
        },
        MFAttributeValue::String(s) => unsafe {
            (out as *mut u16).write(31); // VT_LPWSTR
            let bytes = (s.len() + 1) * 2;
            let mem = apartment::allocate(bytes) as *mut u16;
            if !mem.is_null() {
                std::ptr::copy_nonoverlapping(s.as_ptr(), mem, s.len());
                *mem.add(s.len()) = 0;
                (out.add(8) as *mut *mut u16).write(mem);
            }
        },
        MFAttributeValue::Blob(b) => unsafe {
            (out as *mut u16).write(0x1011); // VT_VECTOR | VT_UI1
            (out.add(8) as *mut u32).write(b.len() as u32);
            let mem = apartment::allocate(b.len().max(1)) as *mut u8;
            if !mem.is_null() {
                std::ptr::copy_nonoverlapping(b.as_ptr(), mem, b.len());
                (out.add(16) as *mut *mut u8).write(mem);
            }
        },
        MFAttributeValue::Unknown(p) => unsafe {
            (out as *mut u16).write(13); // VT_UNKNOWN
            (out.add(8) as *mut *mut c_void).write(*p);
            if !p.is_null() {
                let vt = *(p.cast::<*const *const usize>());
                let add_ref: extern "win64" fn(*mut c_void) -> u32 =
                    std::mem::transmute(*vt.add(1));
                add_ref(*p);
            }
        },
    }
}

unsafe fn read_propvariant(val: *const u8) -> Option<MFAttributeValue> {
    let vt = unsafe { (val as *const u16).read() };
    match vt {
        19 => {
            let v = unsafe { (val.add(8) as *const u32).read() };
            Some(MFAttributeValue::Uint32(v))
        }
        21 => {
            let v = unsafe { (val.add(8) as *const u64).read() };
            Some(MFAttributeValue::Uint64(v))
        }
        5 => {
            let v = unsafe { (val.add(8) as *const f64).read() };
            Some(MFAttributeValue::Double(v))
        }
        72 => {
            let ptr = unsafe { (val.add(8) as *const *const [u8; 16]).read() };
            if !ptr.is_null() {
                Some(MFAttributeValue::Guid(unsafe { *ptr }))
            } else {
                None
            }
        }
        31 => {
            let ptr = unsafe { (val.add(8) as *const *const u16).read() };
            if !ptr.is_null() {
                let mut len = 0;
                while unsafe { *ptr.add(len) } != 0 {
                    len += 1;
                }
                let mut s = Vec::with_capacity(len);
                for i in 0..len {
                    s.push(unsafe { *ptr.add(i) });
                }
                Some(MFAttributeValue::String(s))
            } else {
                None
            }
        }
        0x1011 => {
            let count = unsafe { (val.add(8) as *const u32).read() } as usize;
            let ptr = unsafe { (val.add(16) as *const *const u8).read() };
            if !ptr.is_null() {
                let slice = unsafe { std::slice::from_raw_parts(ptr, count) };
                Some(MFAttributeValue::Blob(slice.to_vec()))
            } else {
                Some(MFAttributeValue::Blob(Vec::new()))
            }
        }
        13 => {
            let ptr = unsafe { (val.add(8) as *const *mut c_void).read() };
            Some(MFAttributeValue::Unknown(ptr))
        }
        _ => None,
    }
}

#[repr(C)]
pub struct MFAttributes {
    pub vtable: *const usize,
    pub refs: AtomicU32,
    pub items: Mutex<Vec<([u8; 16], MFAttributeValue)>>,
    pub is_media_type: bool,
}

impl MFAttributes {
    pub fn new(initial_capacity: usize) -> Self {
        Self {
            vtable: attributes_vtable().as_ptr(),
            refs: AtomicU32::new(1),
            items: Mutex::new(Vec::with_capacity(initial_capacity)),
            is_media_type: false,
        }
    }

    pub fn new_media_type() -> Self {
        Self {
            vtable: media_type_vtable().as_ptr(),
            refs: AtomicU32::new(1),
            items: Mutex::new(Vec::new()),
            is_media_type: true,
        }
    }

    pub fn get_item_val(&self, guid: &[u8; 16]) -> Option<MFAttributeValue> {
        let items = self.items.lock().unwrap();
        items.iter().find(|(k, _)| k == guid).map(|(_, v)| v.clone())
    }

    pub fn set_item_val(&self, guid: [u8; 16], val: MFAttributeValue) {
        let mut items = self.items.lock().unwrap();
        if let Some(entry) = items.iter_mut().find(|(k, _)| *k == guid) {
            entry.1 = val;
        } else {
            items.push((guid, val));
        }
    }

    pub fn delete_item_val(&self, guid: &[u8; 16]) -> bool {
        let mut items = self.items.lock().unwrap();
        if let Some(pos) = items.iter().position(|(k, _)| k == guid) {
            items.remove(pos);
            true
        } else {
            false
        }
    }
}

// ---------------------------------------------------------------------------
// IMFAttributes vtable implementations
// ---------------------------------------------------------------------------

unsafe extern "win64" fn mf_query_interface(
    this: *mut MFAttributes,
    riid: *const [u8; 16],
    out: *mut *mut c_void,
) -> i32 {
    if out.is_null() || riid.is_null() {
        return E_POINTER;
    }
    unsafe { *out = std::ptr::null_mut() };
    let iid = unsafe { *riid };
    let is_mt = unsafe { (*this).is_media_type };
    if iid == IID_IUNKNOWN
        || iid == IID_IMFATTRIBUTES
        || (is_mt && iid == IID_IMFMEDIATYPE)
    {
        unsafe {
            *out = this.cast();
            mf_add_ref(this);
        }
        return S_OK;
    }
    E_NOINTERFACE
}

unsafe extern "win64" fn mf_add_ref(this: *mut MFAttributes) -> u32 {
    unsafe { (*this).refs.fetch_add(1, Ordering::Relaxed) + 1 }
}

unsafe extern "win64" fn mf_release(this: *mut MFAttributes) -> u32 {
    let prev = unsafe { (*this).refs.fetch_sub(1, Ordering::Release) };
    if prev == 1 {
        std::sync::atomic::fence(Ordering::Acquire);
        unsafe { drop(Box::from_raw(this)) };
        0
    } else {
        prev - 1
    }
}

unsafe extern "win64" fn mf_get_item(
    this: *mut MFAttributes,
    guid_key: *const [u8; 16],
    p_value: *mut u8,
) -> i32 {
    if guid_key.is_null() || p_value.is_null() {
        return E_POINTER;
    }
    let key = unsafe { *guid_key };
    if let Some(val) = unsafe { (*this).get_item_val(&key) } {
        unsafe { write_propvariant(&val, p_value) };
        S_OK
    } else {
        MF_E_ATTRIBUTENOTFOUND
    }
}

unsafe extern "win64" fn mf_get_item_type(
    this: *mut MFAttributes,
    guid_key: *const [u8; 16],
    p_type: *mut u32,
) -> i32 {
    if guid_key.is_null() || p_type.is_null() {
        return E_POINTER;
    }
    let key = unsafe { *guid_key };
    if let Some(val) = unsafe { (*this).get_item_val(&key) } {
        let t = match val {
            MFAttributeValue::Uint32(_) => 19,
            MFAttributeValue::Uint64(_) => 21,
            MFAttributeValue::Double(_) => 5,
            MFAttributeValue::Guid(_) => 72,
            MFAttributeValue::String(_) => 31,
            MFAttributeValue::Blob(_) => 0x1011,
            MFAttributeValue::Unknown(_) => 13,
        };
        unsafe { *p_type = t };
        S_OK
    } else {
        MF_E_ATTRIBUTENOTFOUND
    }
}

unsafe extern "win64" fn mf_compare_item(
    this: *mut MFAttributes,
    guid_key: *const [u8; 16],
    value: *const u8,
    pb_result: *mut i32,
) -> i32 {
    if guid_key.is_null() || value.is_null() || pb_result.is_null() {
        return E_POINTER;
    }
    let key = unsafe { *guid_key };
    let cur = unsafe { (*this).get_item_val(&key) };
    let target = unsafe { read_propvariant(value) };
    let matches = match (cur, target) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    };
    unsafe { *pb_result = if matches { 1 } else { 0 } };
    S_OK
}

unsafe extern "win64" fn mf_compare(
    _this: *mut MFAttributes,
    _theirs: *mut c_void,
    _match_type: u32,
    pb_result: *mut i32,
) -> i32 {
    if pb_result.is_null() {
        return E_POINTER;
    }
    unsafe { *pb_result = 1 };
    S_OK
}

unsafe extern "win64" fn mf_get_uint32(
    this: *mut MFAttributes,
    guid_key: *const [u8; 16],
    pun_value: *mut u32,
) -> i32 {
    if guid_key.is_null() || pun_value.is_null() {
        return E_POINTER;
    }
    let key = unsafe { *guid_key };
    match unsafe { (*this).get_item_val(&key) } {
        Some(MFAttributeValue::Uint32(v)) => {
            unsafe { *pun_value = v };
            S_OK
        }
        Some(_) => MF_E_INVALIDTYPE,
        None => MF_E_ATTRIBUTENOTFOUND,
    }
}

unsafe extern "win64" fn mf_get_uint64(
    this: *mut MFAttributes,
    guid_key: *const [u8; 16],
    pun_value: *mut u64,
) -> i32 {
    if guid_key.is_null() || pun_value.is_null() {
        return E_POINTER;
    }
    let key = unsafe { *guid_key };
    match unsafe { (*this).get_item_val(&key) } {
        Some(MFAttributeValue::Uint64(v)) => {
            unsafe { *pun_value = v };
            S_OK
        }
        Some(_) => MF_E_INVALIDTYPE,
        None => MF_E_ATTRIBUTENOTFOUND,
    }
}

unsafe extern "win64" fn mf_get_double(
    this: *mut MFAttributes,
    guid_key: *const [u8; 16],
    pf_value: *mut f64,
) -> i32 {
    if guid_key.is_null() || pf_value.is_null() {
        return E_POINTER;
    }
    let key = unsafe { *guid_key };
    match unsafe { (*this).get_item_val(&key) } {
        Some(MFAttributeValue::Double(v)) => {
            unsafe { *pf_value = v };
            S_OK
        }
        Some(_) => MF_E_INVALIDTYPE,
        None => MF_E_ATTRIBUTENOTFOUND,
    }
}

unsafe extern "win64" fn mf_get_guid(
    this: *mut MFAttributes,
    guid_key: *const [u8; 16],
    pguid_value: *mut [u8; 16],
) -> i32 {
    if guid_key.is_null() || pguid_value.is_null() {
        return E_POINTER;
    }
    let key = unsafe { *guid_key };
    match unsafe { (*this).get_item_val(&key) } {
        Some(MFAttributeValue::Guid(v)) => {
            unsafe { *pguid_value = v };
            S_OK
        }
        Some(_) => MF_E_INVALIDTYPE,
        None => MF_E_ATTRIBUTENOTFOUND,
    }
}

unsafe extern "win64" fn mf_get_string_length(
    this: *mut MFAttributes,
    guid_key: *const [u8; 16],
    pcch_length: *mut u32,
) -> i32 {
    if guid_key.is_null() || pcch_length.is_null() {
        return E_POINTER;
    }
    let key = unsafe { *guid_key };
    match unsafe { (*this).get_item_val(&key) } {
        Some(MFAttributeValue::String(s)) => {
            unsafe { *pcch_length = s.len() as u32 };
            S_OK
        }
        Some(_) => MF_E_INVALIDTYPE,
        None => MF_E_ATTRIBUTENOTFOUND,
    }
}

unsafe extern "win64" fn mf_get_string(
    this: *mut MFAttributes,
    guid_key: *const [u8; 16],
    pwsz_value: *mut u16,
    cch_buf_size: u32,
    pcch_length: *mut u32,
) -> i32 {
    if guid_key.is_null() {
        return E_POINTER;
    }
    let key = unsafe { *guid_key };
    match unsafe { (*this).get_item_val(&key) } {
        Some(MFAttributeValue::String(s)) => {
            if !pcch_length.is_null() {
                unsafe { *pcch_length = s.len() as u32 };
            }
            if pwsz_value.is_null() {
                return S_OK;
            }
            if cch_buf_size <= s.len() as u32 {
                return HRESULT_ERROR_INSUFFICIENT_BUFFER;
            }
            unsafe {
                std::ptr::copy_nonoverlapping(s.as_ptr(), pwsz_value, s.len());
                *pwsz_value.add(s.len()) = 0;
            }
            S_OK
        }
        Some(_) => MF_E_INVALIDTYPE,
        None => MF_E_ATTRIBUTENOTFOUND,
    }
}

unsafe extern "win64" fn mf_get_allocated_string(
    this: *mut MFAttributes,
    guid_key: *const [u8; 16],
    ppwsz_value: *mut *mut u16,
    pcch_length: *mut u32,
) -> i32 {
    if guid_key.is_null() || ppwsz_value.is_null() {
        return E_POINTER;
    }
    let key = unsafe { *guid_key };
    match unsafe { (*this).get_item_val(&key) } {
        Some(MFAttributeValue::String(s)) => {
            let mem = apartment::allocate((s.len() + 1) * 2) as *mut u16;
            if mem.is_null() {
                return E_OUTOFMEMORY;
            }
            unsafe {
                std::ptr::copy_nonoverlapping(s.as_ptr(), mem, s.len());
                *mem.add(s.len()) = 0;
                *ppwsz_value = mem;
                if !pcch_length.is_null() {
                    *pcch_length = s.len() as u32;
                }
            }
            S_OK
        }
        Some(_) => MF_E_INVALIDTYPE,
        None => MF_E_ATTRIBUTENOTFOUND,
    }
}

unsafe extern "win64" fn mf_get_blob_size(
    this: *mut MFAttributes,
    guid_key: *const [u8; 16],
    pcb_blob_size: *mut u32,
) -> i32 {
    if guid_key.is_null() || pcb_blob_size.is_null() {
        return E_POINTER;
    }
    let key = unsafe { *guid_key };
    match unsafe { (*this).get_item_val(&key) } {
        Some(MFAttributeValue::Blob(b)) => {
            unsafe { *pcb_blob_size = b.len() as u32 };
            S_OK
        }
        Some(_) => MF_E_INVALIDTYPE,
        None => MF_E_ATTRIBUTENOTFOUND,
    }
}

unsafe extern "win64" fn mf_get_blob(
    this: *mut MFAttributes,
    guid_key: *const [u8; 16],
    p_buf: *mut u8,
    cb_buf_size: u32,
    pcb_blob_size: *mut u32,
) -> i32 {
    if guid_key.is_null() {
        return E_POINTER;
    }
    let key = unsafe { *guid_key };
    match unsafe { (*this).get_item_val(&key) } {
        Some(MFAttributeValue::Blob(b)) => {
            if !pcb_blob_size.is_null() {
                unsafe { *pcb_blob_size = b.len() as u32 };
            }
            if p_buf.is_null() {
                return S_OK;
            }
            if cb_buf_size < b.len() as u32 {
                return HRESULT_ERROR_INSUFFICIENT_BUFFER;
            }
            unsafe {
                std::ptr::copy_nonoverlapping(b.as_ptr(), p_buf, b.len());
            }
            S_OK
        }
        Some(_) => MF_E_INVALIDTYPE,
        None => MF_E_ATTRIBUTENOTFOUND,
    }
}

unsafe extern "win64" fn mf_get_allocated_blob(
    this: *mut MFAttributes,
    guid_key: *const [u8; 16],
    pp_buf: *mut *mut u8,
    pcb_size: *mut u32,
) -> i32 {
    if guid_key.is_null() || pp_buf.is_null() {
        return E_POINTER;
    }
    let key = unsafe { *guid_key };
    match unsafe { (*this).get_item_val(&key) } {
        Some(MFAttributeValue::Blob(b)) => {
            let mem = apartment::allocate(b.len().max(1)) as *mut u8;
            if mem.is_null() {
                return E_OUTOFMEMORY;
            }
            unsafe {
                std::ptr::copy_nonoverlapping(b.as_ptr(), mem, b.len());
                *pp_buf = mem;
                if !pcb_size.is_null() {
                    *pcb_size = b.len() as u32;
                }
            }
            S_OK
        }
        Some(_) => MF_E_INVALIDTYPE,
        None => MF_E_ATTRIBUTENOTFOUND,
    }
}

unsafe extern "win64" fn mf_get_unknown(
    this: *mut MFAttributes,
    guid_key: *const [u8; 16],
    riid: *const [u8; 16],
    ppv: *mut *mut c_void,
) -> i32 {
    if guid_key.is_null() || ppv.is_null() || riid.is_null() {
        return E_POINTER;
    }
    let key = unsafe { *guid_key };
    match unsafe { (*this).get_item_val(&key) } {
        Some(MFAttributeValue::Unknown(ptr)) => {
            if ptr.is_null() {
                unsafe { *ppv = std::ptr::null_mut() };
                return S_OK;
            }
            let vt = unsafe { *(ptr.cast::<*const *const usize>()) };
            let qi: extern "win64" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32 =
                unsafe { std::mem::transmute(*vt.add(0)) };
            qi(ptr, riid, ppv)
        }
        Some(_) => MF_E_INVALIDTYPE,
        None => MF_E_ATTRIBUTENOTFOUND,
    }
}

unsafe extern "win64" fn mf_set_item(
    this: *mut MFAttributes,
    guid_key: *const [u8; 16],
    value: *const u8,
) -> i32 {
    if guid_key.is_null() || value.is_null() {
        return E_POINTER;
    }
    let key = unsafe { *guid_key };
    if let Some(val) = unsafe { read_propvariant(value) } {
        unsafe { (*this).set_item_val(key, val) };
        S_OK
    } else {
        E_INVALIDARG
    }
}

unsafe extern "win64" fn mf_delete_item(
    this: *mut MFAttributes,
    guid_key: *const [u8; 16],
) -> i32 {
    if guid_key.is_null() {
        return E_POINTER;
    }
    let key = unsafe { *guid_key };
    unsafe { (*this).delete_item_val(&key) };
    S_OK
}

unsafe extern "win64" fn mf_delete_all_items(this: *mut MFAttributes) -> i32 {
    let mut items = unsafe { (*this).items.lock().unwrap() };
    items.clear();
    S_OK
}

unsafe extern "win64" fn mf_set_uint32(
    this: *mut MFAttributes,
    guid_key: *const [u8; 16],
    un_value: u32,
) -> i32 {
    if guid_key.is_null() {
        return E_POINTER;
    }
    let key = unsafe { *guid_key };
    unsafe { (*this).set_item_val(key, MFAttributeValue::Uint32(un_value)) };
    S_OK
}

unsafe extern "win64" fn mf_set_uint64(
    this: *mut MFAttributes,
    guid_key: *const [u8; 16],
    un_value: u64,
) -> i32 {
    if guid_key.is_null() {
        return E_POINTER;
    }
    let key = unsafe { *guid_key };
    unsafe { (*this).set_item_val(key, MFAttributeValue::Uint64(un_value)) };
    S_OK
}

unsafe extern "win64" fn mf_set_double(
    this: *mut MFAttributes,
    guid_key: *const [u8; 16],
    f_value: f64,
) -> i32 {
    if guid_key.is_null() {
        return E_POINTER;
    }
    let key = unsafe { *guid_key };
    unsafe { (*this).set_item_val(key, MFAttributeValue::Double(f_value)) };
    S_OK
}

unsafe extern "win64" fn mf_set_guid(
    this: *mut MFAttributes,
    guid_key: *const [u8; 16],
    pguid_value: *const [u8; 16],
) -> i32 {
    if guid_key.is_null() || pguid_value.is_null() {
        return E_POINTER;
    }
    let key = unsafe { *guid_key };
    let val = unsafe { *pguid_value };
    unsafe { (*this).set_item_val(key, MFAttributeValue::Guid(val)) };
    S_OK
}

unsafe extern "win64" fn mf_set_string(
    this: *mut MFAttributes,
    guid_key: *const [u8; 16],
    wsz_value: *const u16,
) -> i32 {
    if guid_key.is_null() || wsz_value.is_null() {
        return E_POINTER;
    }
    let key = unsafe { *guid_key };
    let mut len = 0;
    while unsafe { *wsz_value.add(len) } != 0 {
        len += 1;
    }
    let mut s = Vec::with_capacity(len);
    for i in 0..len {
        s.push(unsafe { *wsz_value.add(i) });
    }
    unsafe { (*this).set_item_val(key, MFAttributeValue::String(s)) };
    S_OK
}

unsafe extern "win64" fn mf_set_blob(
    this: *mut MFAttributes,
    guid_key: *const [u8; 16],
    p_buf: *const u8,
    cb_buf_size: u32,
) -> i32 {
    if guid_key.is_null() {
        return E_POINTER;
    }
    if cb_buf_size > 0 && p_buf.is_null() {
        return E_POINTER;
    }
    let key = unsafe { *guid_key };
    let b = if cb_buf_size > 0 {
        unsafe { std::slice::from_raw_parts(p_buf, cb_buf_size as usize).to_vec() }
    } else {
        Vec::new()
    };
    unsafe { (*this).set_item_val(key, MFAttributeValue::Blob(b)) };
    S_OK
}

unsafe extern "win64" fn mf_set_unknown(
    this: *mut MFAttributes,
    guid_key: *const [u8; 16],
    p_unknown: *mut c_void,
) -> i32 {
    if guid_key.is_null() {
        return E_POINTER;
    }
    let key = unsafe { *guid_key };
    if !p_unknown.is_null() {
        let vt = unsafe { *(p_unknown.cast::<*const *const usize>()) };
        let add_ref: extern "win64" fn(*mut c_void) -> u32 =
            unsafe { std::mem::transmute(*vt.add(1)) };
        add_ref(p_unknown);
    }
    unsafe { (*this).set_item_val(key, MFAttributeValue::Unknown(p_unknown)) };
    S_OK
}

unsafe extern "win64" fn mf_lock_store(_this: *mut MFAttributes) -> i32 {
    S_OK
}

unsafe extern "win64" fn mf_unlock_store(_this: *mut MFAttributes) -> i32 {
    S_OK
}

unsafe extern "win64" fn mf_get_count(
    this: *mut MFAttributes,
    pc_items: *mut u32,
) -> i32 {
    if pc_items.is_null() {
        return E_POINTER;
    }
    let items = unsafe { (*this).items.lock().unwrap() };
    unsafe { *pc_items = items.len() as u32 };
    S_OK
}

unsafe extern "win64" fn mf_get_item_by_index(
    this: *mut MFAttributes,
    un_index: u32,
    pguid_key: *mut [u8; 16],
    p_value: *mut u8,
) -> i32 {
    let items = unsafe { (*this).items.lock().unwrap() };
    if (un_index as usize) >= items.len() {
        return E_INVALIDARG;
    }
    let (k, v) = &items[un_index as usize];
    if !pguid_key.is_null() {
        unsafe { *pguid_key = *k };
    }
    if !p_value.is_null() {
        unsafe { write_propvariant(v, p_value) };
    }
    S_OK
}

unsafe extern "win64" fn mf_copy_all_items(
    this: *mut MFAttributes,
    p_dest: *mut c_void,
) -> i32 {
    if p_dest.is_null() {
        return E_POINTER;
    }
    let items = unsafe { (*this).items.lock().unwrap().clone() };
    let vt = unsafe { *(p_dest.cast::<*const *const usize>()) };
    let set_item: extern "win64" fn(*mut c_void, *const [u8; 16], *const u8) -> i32 =
        unsafe { std::mem::transmute(*vt.add(18)) };
    for (k, v) in items {
        let mut pv = [0u8; 24];
        unsafe { write_propvariant(&v, pv.as_mut_ptr()) };
        set_item(p_dest, &k, pv.as_ptr());
        unsafe { super::variants::clear(pv.as_mut_ptr()) };
    }
    S_OK
}

// ---------------------------------------------------------------------------
// IMFMediaType extra methods (slots 33..37)
// ---------------------------------------------------------------------------

unsafe extern "win64" fn mf_media_type_get_major_type(
    this: *mut MFAttributes,
    pguid_major_type: *mut [u8; 16],
) -> i32 {
    if pguid_major_type.is_null() {
        return E_POINTER;
    }
    match unsafe { (*this).get_item_val(&MF_MT_MAJOR_TYPE) } {
        Some(MFAttributeValue::Guid(g)) => {
            unsafe { *pguid_major_type = g };
            S_OK
        }
        _ => {
            unsafe { *pguid_major_type = MF_MEDIA_TYPE_VIDEO };
            S_OK
        }
    }
}

unsafe extern "win64" fn mf_media_type_is_compressed_format(
    _this: *mut MFAttributes,
    pf_compressed: *mut i32,
) -> i32 {
    if !pf_compressed.is_null() {
        unsafe { *pf_compressed = 0 };
    }
    S_OK
}

unsafe extern "win64" fn mf_media_type_is_equal(
    _this: *mut MFAttributes,
    _p_type: *mut c_void,
    pdw_flags: *mut u32,
) -> i32 {
    if !pdw_flags.is_null() {
        unsafe { *pdw_flags = 0 };
    }
    S_OK
}

unsafe extern "win64" fn mf_media_type_get_representation(
    _this: *mut MFAttributes,
    _guid_rep: *const [u8; 16],
    _ppv: *mut *mut c_void,
) -> i32 {
    E_NOTIMPL
}

unsafe extern "win64" fn mf_media_type_free_representation(
    _this: *mut MFAttributes,
    _guid_rep: *const [u8; 16],
    _pv: *mut c_void,
) -> i32 {
    E_NOTIMPL
}

fn attributes_vtable() -> &'static [usize; 33] {
    static VTABLE: OnceLock<[usize; 33]> = OnceLock::new();
    VTABLE.get_or_init(|| [
        mf_query_interface as *const () as usize,
        mf_add_ref as *const () as usize,
        mf_release as *const () as usize,
        mf_get_item as *const () as usize,
        mf_get_item_type as *const () as usize,
        mf_compare_item as *const () as usize,
        mf_compare as *const () as usize,
        mf_get_uint32 as *const () as usize,
        mf_get_uint64 as *const () as usize,
        mf_get_double as *const () as usize,
        mf_get_guid as *const () as usize,
        mf_get_string_length as *const () as usize,
        mf_get_string as *const () as usize,
        mf_get_allocated_string as *const () as usize,
        mf_get_blob_size as *const () as usize,
        mf_get_blob as *const () as usize,
        mf_get_allocated_blob as *const () as usize,
        mf_get_unknown as *const () as usize,
        mf_set_item as *const () as usize,
        mf_delete_item as *const () as usize,
        mf_delete_all_items as *const () as usize,
        mf_set_uint32 as *const () as usize,
        mf_set_uint64 as *const () as usize,
        mf_set_double as *const () as usize,
        mf_set_guid as *const () as usize,
        mf_set_string as *const () as usize,
        mf_set_blob as *const () as usize,
        mf_set_unknown as *const () as usize,
        mf_lock_store as *const () as usize,
        mf_unlock_store as *const () as usize,
        mf_get_count as *const () as usize,
        mf_get_item_by_index as *const () as usize,
        mf_copy_all_items as *const () as usize,
    ])
}

fn media_type_vtable() -> &'static [usize; 38] {
    static VTABLE: OnceLock<[usize; 38]> = OnceLock::new();
    VTABLE.get_or_init(|| [
        mf_query_interface as *const () as usize,
        mf_add_ref as *const () as usize,
        mf_release as *const () as usize,
        mf_get_item as *const () as usize,
        mf_get_item_type as *const () as usize,
        mf_compare_item as *const () as usize,
        mf_compare as *const () as usize,
        mf_get_uint32 as *const () as usize,
        mf_get_uint64 as *const () as usize,
        mf_get_double as *const () as usize,
        mf_get_guid as *const () as usize,
        mf_get_string_length as *const () as usize,
        mf_get_string as *const () as usize,
        mf_get_allocated_string as *const () as usize,
        mf_get_blob_size as *const () as usize,
        mf_get_blob as *const () as usize,
        mf_get_allocated_blob as *const () as usize,
        mf_get_unknown as *const () as usize,
        mf_set_item as *const () as usize,
        mf_delete_item as *const () as usize,
        mf_delete_all_items as *const () as usize,
        mf_set_uint32 as *const () as usize,
        mf_set_uint64 as *const () as usize,
        mf_set_double as *const () as usize,
        mf_set_guid as *const () as usize,
        mf_set_string as *const () as usize,
        mf_set_blob as *const () as usize,
        mf_set_unknown as *const () as usize,
        mf_lock_store as *const () as usize,
        mf_unlock_store as *const () as usize,
        mf_get_count as *const () as usize,
        mf_get_item_by_index as *const () as usize,
        mf_copy_all_items as *const () as usize,
        // IMFMediaType extra slots:
        mf_media_type_get_major_type as *const () as usize,
        mf_media_type_is_compressed_format as *const () as usize,
        mf_media_type_is_equal as *const () as usize,
        mf_media_type_get_representation as *const () as usize,
        mf_media_type_free_representation as *const () as usize,
    ])
}

// ---------------------------------------------------------------------------
// IMFByteStream
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct MFByteStream {
    pub vtable: *const usize,
    pub refs: AtomicU32,
    pub stream: *mut c_void,
}

unsafe extern "win64" fn mf_bytestream_query_interface(
    this: *mut MFByteStream,
    riid: *const [u8; 16],
    out: *mut *mut c_void,
) -> i32 {
    if out.is_null() || riid.is_null() {
        return E_POINTER;
    }
    unsafe { *out = std::ptr::null_mut() };
    let iid = unsafe { *riid };
    if iid == IID_IUNKNOWN || iid == IID_IMFBYTESTREAM {
        unsafe {
            *out = this.cast();
            mf_bytestream_add_ref(this);
        }
        return S_OK;
    }
    E_NOINTERFACE
}

unsafe extern "win64" fn mf_bytestream_add_ref(this: *mut MFByteStream) -> u32 {
    unsafe { (*this).refs.fetch_add(1, Ordering::Relaxed) + 1 }
}

unsafe extern "win64" fn mf_bytestream_release(this: *mut MFByteStream) -> u32 {
    let prev = unsafe { (*this).refs.fetch_sub(1, Ordering::Release) };
    if prev == 1 {
        std::sync::atomic::fence(Ordering::Acquire);
        let stream = unsafe { (*this).stream };
        if !stream.is_null() {
            let vt = unsafe { *(stream.cast::<*const *const usize>()) };
            let rel: extern "win64" fn(*mut c_void) -> u32 =
                unsafe { std::mem::transmute(*vt.add(2)) };
            rel(stream);
        }
        unsafe { drop(Box::from_raw(this)) };
        0
    } else {
        prev - 1
    }
}

unsafe extern "win64" fn mf_bytestream_get_capabilities(
    _this: *mut MFByteStream,
    pdw_capabilities: *mut u32,
) -> i32 {
    if !pdw_capabilities.is_null() {
        // MFBYTESTREAM_IS_READABLE = 0x1, MFBYTESTREAM_IS_SEEKABLE = 0x2
        unsafe { *pdw_capabilities = 0x00000001 | 0x00000002 };
    }
    S_OK
}

unsafe extern "win64" fn mf_bytestream_get_length(
    this: *mut MFByteStream,
    pqw_length: *mut u64,
) -> i32 {
    if pqw_length.is_null() {
        return E_POINTER;
    }
    let stream = unsafe { (*this).stream };
    if !stream.is_null() {
        let mut statstg = [0u8; 80];
        let vt = unsafe { *(stream.cast::<*const *const usize>()) };
        let stat_fn: extern "win64" fn(*mut c_void, *mut u8, u32) -> i32 =
            unsafe { std::mem::transmute(*vt.add(12)) };
        let hr = stat_fn(stream, statstg.as_mut_ptr(), 1);
        if hr >= 0 {
            let size = unsafe { (statstg.as_ptr().add(48) as *const u64).read_unaligned() };
            unsafe { *pqw_length = size };
            return S_OK;
        }
    }
    unsafe { *pqw_length = 0 };
    S_OK
}

unsafe extern "win64" fn mf_bytestream_set_length(
    _this: *mut MFByteStream,
    _qw_length: u64,
) -> i32 {
    E_NOTIMPL
}

unsafe extern "win64" fn mf_bytestream_get_current_position(
    this: *mut MFByteStream,
    pqw_position: *mut u64,
) -> i32 {
    if pqw_position.is_null() {
        return E_POINTER;
    }
    let stream = unsafe { (*this).stream };
    if !stream.is_null() {
        let vt = unsafe { *(stream.cast::<*const *const usize>()) };
        let seek_fn: extern "win64" fn(*mut c_void, i64, u32, *mut u64) -> i32 =
            unsafe { std::mem::transmute(*vt.add(5)) };
        return seek_fn(stream, 0, 1 /* STREAM_SEEK_CUR */, pqw_position);
    }
    unsafe { *pqw_position = 0 };
    S_OK
}

unsafe extern "win64" fn mf_bytestream_set_current_position(
    this: *mut MFByteStream,
    qw_position: u64,
) -> i32 {
    let stream = unsafe { (*this).stream };
    if !stream.is_null() {
        let vt = unsafe { *(stream.cast::<*const *const usize>()) };
        let seek_fn: extern "win64" fn(*mut c_void, i64, u32, *mut u64) -> i32 =
            unsafe { std::mem::transmute(*vt.add(5)) };
        return seek_fn(stream, qw_position as i64, 0 /* STREAM_SEEK_SET */, std::ptr::null_mut());
    }
    S_OK
}

unsafe extern "win64" fn mf_bytestream_is_end_of_stream(
    this: *mut MFByteStream,
    pf_end_of_stream: *mut i32,
) -> i32 {
    if !pf_end_of_stream.is_null() {
        let mut cur = 0u64;
        let mut len = 0u64;
        unsafe {
            mf_bytestream_get_current_position(this, &mut cur);
            mf_bytestream_get_length(this, &mut len);
            *pf_end_of_stream = if cur >= len { 1 } else { 0 };
        }
    }
    S_OK
}

unsafe extern "win64" fn mf_bytestream_read(
    this: *mut MFByteStream,
    pb: *mut u8,
    cb: u32,
    pcb_read: *mut u32,
) -> i32 {
    let stream = unsafe { (*this).stream };
    if !stream.is_null() {
        let vt = unsafe { *(stream.cast::<*const *const usize>()) };
        let read_fn: extern "win64" fn(*mut c_void, *mut u8, u32, *mut u32) -> i32 =
            unsafe { std::mem::transmute(*vt.add(3)) };
        return read_fn(stream, pb, cb, pcb_read);
    }
    if !pcb_read.is_null() {
        unsafe { *pcb_read = 0 };
    }
    S_OK
}

unsafe extern "win64" fn mf_bytestream_begin_read(
    _this: *mut MFByteStream,
    _pb: *mut u8,
    _cb: u32,
    _p_callback: *mut c_void,
    _punk_state: *mut c_void,
) -> i32 {
    E_NOTIMPL
}

unsafe extern "win64" fn mf_bytestream_end_read(
    _this: *mut MFByteStream,
    _p_result: *mut c_void,
    pcb_read: *mut u32,
) -> i32 {
    if !pcb_read.is_null() {
        unsafe { *pcb_read = 0 };
    }
    E_NOTIMPL
}

unsafe extern "win64" fn mf_bytestream_write(
    this: *mut MFByteStream,
    pb: *const u8,
    cb: u32,
    pcb_written: *mut u32,
) -> i32 {
    let stream = unsafe { (*this).stream };
    if !stream.is_null() {
        let vt = unsafe { *(stream.cast::<*const *const usize>()) };
        let write_fn: extern "win64" fn(*mut c_void, *const u8, u32, *mut u32) -> i32 =
            unsafe { std::mem::transmute(*vt.add(4)) };
        return write_fn(stream, pb, cb, pcb_written);
    }
    if !pcb_written.is_null() {
        unsafe { *pcb_written = 0 };
    }
    S_OK
}

unsafe extern "win64" fn mf_bytestream_begin_write(
    _this: *mut MFByteStream,
    _pb: *const u8,
    _cb: u32,
    _p_callback: *mut c_void,
    _punk_state: *mut c_void,
) -> i32 {
    E_NOTIMPL
}

unsafe extern "win64" fn mf_bytestream_end_write(
    _this: *mut MFByteStream,
    _p_result: *mut c_void,
    pcb_written: *mut u32,
) -> i32 {
    if !pcb_written.is_null() {
        unsafe { *pcb_written = 0 };
    }
    E_NOTIMPL
}

unsafe extern "win64" fn mf_bytestream_seek(
    this: *mut MFByteStream,
    seek_origin: u32,
    qw_seek_offset: i64,
    _dw_seek_flags: u32,
    pqw_current_position: *mut u64,
) -> i32 {
    let stream = unsafe { (*this).stream };
    if !stream.is_null() {
        let vt = unsafe { *(stream.cast::<*const *const usize>()) };
        let seek_fn: extern "win64" fn(*mut c_void, i64, u32, *mut u64) -> i32 =
            unsafe { std::mem::transmute(*vt.add(5)) };
        return seek_fn(stream, qw_seek_offset, seek_origin, pqw_current_position);
    }
    if !pqw_current_position.is_null() {
        unsafe { *pqw_current_position = 0 };
    }
    S_OK
}

unsafe extern "win64" fn mf_bytestream_flush(_this: *mut MFByteStream) -> i32 {
    S_OK
}

unsafe extern "win64" fn mf_bytestream_close(_this: *mut MFByteStream) -> i32 {
    S_OK
}

fn byte_stream_vtable() -> &'static [usize; 18] {
    static VTABLE: OnceLock<[usize; 18]> = OnceLock::new();
    VTABLE.get_or_init(|| [
        mf_bytestream_query_interface as *const () as usize,
        mf_bytestream_add_ref as *const () as usize,
        mf_bytestream_release as *const () as usize,
        mf_bytestream_get_capabilities as *const () as usize,
        mf_bytestream_get_length as *const () as usize,
        mf_bytestream_set_length as *const () as usize,
        mf_bytestream_get_current_position as *const () as usize,
        mf_bytestream_set_current_position as *const () as usize,
        mf_bytestream_is_end_of_stream as *const () as usize,
        mf_bytestream_read as *const () as usize,
        mf_bytestream_begin_read as *const () as usize,
        mf_bytestream_end_read as *const () as usize,
        mf_bytestream_write as *const () as usize,
        mf_bytestream_begin_write as *const () as usize,
        mf_bytestream_end_write as *const () as usize,
        mf_bytestream_seek as *const () as usize,
        mf_bytestream_flush as *const () as usize,
        mf_bytestream_close as *const () as usize,
    ])
}

// ---------------------------------------------------------------------------
// IMFSourceReader
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct MFSourceReader {
    pub vtable: *const usize,
    pub refs: AtomicU32,
    pub byte_stream: *mut c_void,
    pub attributes: *mut c_void,
}

unsafe extern "win64" fn mf_sourcereader_query_interface(
    this: *mut MFSourceReader,
    riid: *const [u8; 16],
    out: *mut *mut c_void,
) -> i32 {
    if out.is_null() || riid.is_null() {
        return E_POINTER;
    }
    unsafe { *out = std::ptr::null_mut() };
    let iid = unsafe { *riid };
    if iid == IID_IUNKNOWN || iid == IID_IMFSOURCEREADER {
        unsafe {
            *out = this.cast();
            mf_sourcereader_add_ref(this);
        }
        return S_OK;
    }
    E_NOINTERFACE
}

unsafe extern "win64" fn mf_sourcereader_add_ref(this: *mut MFSourceReader) -> u32 {
    unsafe { (*this).refs.fetch_add(1, Ordering::Relaxed) + 1 }
}

unsafe extern "win64" fn mf_sourcereader_release(this: *mut MFSourceReader) -> u32 {
    let prev = unsafe { (*this).refs.fetch_sub(1, Ordering::Release) };
    if prev == 1 {
        std::sync::atomic::fence(Ordering::Acquire);
        let bs = unsafe { (*this).byte_stream };
        if !bs.is_null() {
            let vt = unsafe { *(bs.cast::<*const *const usize>()) };
            let rel: extern "win64" fn(*mut c_void) -> u32 =
                unsafe { std::mem::transmute(*vt.add(2)) };
            rel(bs);
        }
        let attr = unsafe { (*this).attributes };
        if !attr.is_null() {
            let vt = unsafe { *(attr.cast::<*const *const usize>()) };
            let rel: extern "win64" fn(*mut c_void) -> u32 =
                unsafe { std::mem::transmute(*vt.add(2)) };
            rel(attr);
        }
        unsafe { drop(Box::from_raw(this)) };
        0
    } else {
        prev - 1
    }
}

unsafe extern "win64" fn mf_sourcereader_get_stream_selection(
    _this: *mut MFSourceReader,
    _dw_stream_index: u32,
    pf_selected: *mut i32,
) -> i32 {
    if !pf_selected.is_null() {
        unsafe { *pf_selected = 1 };
    }
    S_OK
}

unsafe extern "win64" fn mf_sourcereader_set_stream_selection(
    _this: *mut MFSourceReader,
    _dw_stream_index: u32,
    _f_selected: i32,
) -> i32 {
    S_OK
}

unsafe extern "win64" fn mf_sourcereader_get_native_media_type(
    _this: *mut MFSourceReader,
    _dw_stream_index: u32,
    dw_media_type_index: u32,
    pp_media_type: *mut *mut c_void,
) -> i32 {
    if pp_media_type.is_null() {
        return E_POINTER;
    }
    if dw_media_type_index > 0 {
        return MF_E_NO_MORE_TYPES;
    }
    let mt = Box::new(MFAttributes::new_media_type());
    mt.set_item_val(MF_MT_MAJOR_TYPE, MFAttributeValue::Guid(MF_MEDIA_TYPE_VIDEO));
    unsafe { *pp_media_type = Box::into_raw(mt) as *mut c_void };
    S_OK
}

unsafe extern "win64" fn mf_sourcereader_get_current_media_type(
    _this: *mut MFSourceReader,
    _dw_stream_index: u32,
    pp_media_type: *mut *mut c_void,
) -> i32 {
    if pp_media_type.is_null() {
        return E_POINTER;
    }
    let mt = Box::new(MFAttributes::new_media_type());
    mt.set_item_val(MF_MT_MAJOR_TYPE, MFAttributeValue::Guid(MF_MEDIA_TYPE_VIDEO));
    unsafe { *pp_media_type = Box::into_raw(mt) as *mut c_void };
    S_OK
}

unsafe extern "win64" fn mf_sourcereader_set_current_media_type(
    _this: *mut MFSourceReader,
    _dw_stream_index: u32,
    _pdw_reserved: *mut u32,
    _p_media_type: *mut c_void,
) -> i32 {
    S_OK
}

unsafe extern "win64" fn mf_sourcereader_set_stream_position(
    _this: *mut MFSourceReader,
    _guid_time_format: *const [u8; 16],
    _var_position: *const u8,
) -> i32 {
    S_OK
}

unsafe extern "win64" fn mf_sourcereader_read_sample(
    _this: *mut MFSourceReader,
    dw_stream_index: u32,
    _dw_control_flags: u32,
    pdw_actual_stream_index: *mut u32,
    pdw_stream_flags: *mut u32,
    pll_timestamp: *mut i64,
    pp_sample: *mut *mut c_void,
) -> i32 {
    if !pdw_actual_stream_index.is_null() {
        unsafe { *pdw_actual_stream_index = dw_stream_index };
    }
    if !pdw_stream_flags.is_null() {
        // MF_SOURCE_READERF_ENDOFSTREAM = 0x00000001
        unsafe { *pdw_stream_flags = 0x00000001 };
    }
    if !pll_timestamp.is_null() {
        unsafe { *pll_timestamp = 0 };
    }
    if !pp_sample.is_null() {
        unsafe { *pp_sample = std::ptr::null_mut() };
    }
    S_OK
}

unsafe extern "win64" fn mf_sourcereader_flush(
    _this: *mut MFSourceReader,
    _dw_stream_index: u32,
) -> i32 {
    S_OK
}

unsafe extern "win64" fn mf_sourcereader_get_service_for_stream(
    _this: *mut MFSourceReader,
    _dw_stream_index: u32,
    _guid_service: *const [u8; 16],
    _riid: *const [u8; 16],
    ppv_object: *mut *mut c_void,
) -> i32 {
    if !ppv_object.is_null() {
        unsafe { *ppv_object = std::ptr::null_mut() };
    }
    E_NOINTERFACE
}

unsafe extern "win64" fn mf_sourcereader_get_presentation_attribute(
    _this: *mut MFSourceReader,
    _dw_stream_index: u32,
    _guid_key: *const [u8; 16],
    _p_value: *mut u8,
) -> i32 {
    MF_E_ATTRIBUTENOTFOUND
}

fn source_reader_vtable() -> &'static [usize; 13] {
    static VTABLE: OnceLock<[usize; 13]> = OnceLock::new();
    VTABLE.get_or_init(|| [
        mf_sourcereader_query_interface as *const () as usize,
        mf_sourcereader_add_ref as *const () as usize,
        mf_sourcereader_release as *const () as usize,
        mf_sourcereader_get_stream_selection as *const () as usize,
        mf_sourcereader_set_stream_selection as *const () as usize,
        mf_sourcereader_get_native_media_type as *const () as usize,
        mf_sourcereader_get_current_media_type as *const () as usize,
        mf_sourcereader_set_current_media_type as *const () as usize,
        mf_sourcereader_set_stream_position as *const () as usize,
        mf_sourcereader_read_sample as *const () as usize,
        mf_sourcereader_flush as *const () as usize,
        mf_sourcereader_get_service_for_stream as *const () as usize,
        mf_sourcereader_get_presentation_attribute as *const () as usize,
    ])
}

// ---------------------------------------------------------------------------
// Public exported APIs
// ---------------------------------------------------------------------------

pub unsafe extern "win64" fn mf_create_attributes(
    out: *mut *mut c_void,
    initial_size: u32,
) -> i32 {
    if out.is_null() {
        return E_POINTER;
    }
    let obj = Box::new(MFAttributes::new(initial_size as usize));
    unsafe { *out = Box::into_raw(obj) as *mut c_void };
    S_OK
}

pub unsafe extern "win64" fn mf_create_media_type(out: *mut *mut c_void) -> i32 {
    if out.is_null() {
        return E_POINTER;
    }
    let obj = Box::new(MFAttributes::new_media_type());
    unsafe { *out = Box::into_raw(obj) as *mut c_void };
    S_OK
}

pub unsafe extern "win64" fn mf_create_mf_bytestream_on_stream(
    p_stream: *mut c_void,
    pp_byte_stream: *mut *mut c_void,
) -> i32 {
    if pp_byte_stream.is_null() {
        return E_POINTER;
    }
    if p_stream.is_null() {
        return E_INVALIDARG;
    }
    let vt = unsafe { *(p_stream.cast::<*const *const usize>()) };
    let add_ref: extern "win64" fn(*mut c_void) -> u32 = unsafe { std::mem::transmute(*vt.add(1)) };
    add_ref(p_stream);

    let obj = Box::new(MFByteStream {
        vtable: byte_stream_vtable().as_ptr(),
        refs: AtomicU32::new(1),
        stream: p_stream,
    });
    unsafe { *pp_byte_stream = Box::into_raw(obj) as *mut c_void };
    S_OK
}

pub unsafe extern "win64" fn mf_create_source_reader_from_byte_stream(
    p_byte_stream: *mut c_void,
    p_attributes: *mut c_void,
    pp_source_reader: *mut *mut c_void,
) -> i32 {
    if pp_source_reader.is_null() {
        return E_POINTER;
    }
    if !p_byte_stream.is_null() {
        let vt = unsafe { *(p_byte_stream.cast::<*const *const usize>()) };
        let add_ref: extern "win64" fn(*mut c_void) -> u32 =
            unsafe { std::mem::transmute(*vt.add(1)) };
        add_ref(p_byte_stream);
    }
    if !p_attributes.is_null() {
        let vt = unsafe { *(p_attributes.cast::<*const *const usize>()) };
        let add_ref: extern "win64" fn(*mut c_void) -> u32 =
            unsafe { std::mem::transmute(*vt.add(1)) };
        add_ref(p_attributes);
    }

    let obj = Box::new(MFSourceReader {
        vtable: source_reader_vtable().as_ptr(),
        refs: AtomicU32::new(1),
        byte_stream: p_byte_stream,
        attributes: p_attributes,
    });
    unsafe { *pp_source_reader = Box::into_raw(obj) as *mut c_void };
    S_OK
}

pub unsafe extern "win64" fn mf_create_source_reader_from_media_source(
    p_media_source: *mut c_void,
    p_attributes: *mut c_void,
    pp_source_reader: *mut *mut c_void,
) -> i32 {
    unsafe { mf_create_source_reader_from_byte_stream(p_media_source, p_attributes, pp_source_reader) }
}

pub unsafe extern "win64" fn mf_create_source_reader_from_url(
    _pwsz_url: *const u16,
    p_attributes: *mut c_void,
    pp_source_reader: *mut *mut c_void,
) -> i32 {
    unsafe { mf_create_source_reader_from_byte_stream(std::ptr::null_mut(), p_attributes, pp_source_reader) }
}
