//! Shared-mode WASAPI render endpoint on the host PulseAudio/PipeWire server.
mod adjustment;
mod client;
mod endpoint;
mod format;
mod properties;
mod render;
use std::{
    ffi::c_void,
    ptr,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex,
    },
};
type H = i32;
const POINTER: H = 0x80004003u32 as H;
const ARG: H = 0x80070057u32 as H;
const NO_INTERFACE: H = 0x80004002u32 as H;
const NOT_IMPL: H = 0x80004001u32 as H;
const INVALIDATED: H = 0x88890004u32 as H;
const NOT_INITIALIZED: H = 0x88890001u32 as H;
const UNKNOWN: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 0xc0, 0, 0, 0, 0, 0, 0, 0x46];
pub(crate) const CLASS: [u8; 16] = [
    0x95, 3, 0xde, 0xbc, 0x2f, 0xe5, 0x7c, 0x46, 0x8e, 0x3d, 0xc4, 0x57, 0x92, 0x91, 0x69, 0x2e,
];
const ENUM: [u8; 16] = [
    0xd2, 0x64, 0x56, 0xa9, 0x14, 0x96, 0x35, 0x4f, 0xa7, 0x46, 0xde, 0x8d, 0xb6, 0x36, 0x17, 0xe6,
];
const DEVICE: [u8; 16] = [
    0x3f, 6, 0x66, 0xd6, 0x87, 0x15, 0x43, 0x4e, 0x81, 0xf1, 0xb9, 0x48, 0xe8, 7, 0x36, 0x3f,
];
const ENDPOINT: [u8; 16] = [
    0x88, 0x97, 0xe0, 0x1b, 0x94, 0x68, 0x89, 0x40, 0x85, 0x86, 0x9a, 0x2a, 0x6c, 0x26, 0x5a, 0xc5,
];
const COLLECTION: [u8; 16] = [
    0xbe, 0xa1, 0xd7, 0x0b, 0x1a, 0x7a, 0xdb, 0x44, 0x83, 0x97, 0xcc, 0x53, 0x92, 0x38, 0x7b, 0x5e,
];
const CLIENT: [u8; 16] = [
    0x4c, 0xad, 0xb9, 0x1c, 0xfa, 0xdb, 0x32, 0x4c, 0xb1, 0x78, 0xc2, 0xf5, 0x68, 0xa7, 3, 0xb2,
];
const RENDER: [u8; 16] = [
    0xfc, 0xac, 0x94, 0xf2, 0x46, 0x31, 0x83, 0x44, 0xa7, 0xbf, 0xad, 0xdc, 0xa7, 0xc2, 0x60, 0xe2,
];
const ADJUSTMENT: [u8; 16] = [
    0xa0, 0xc0, 0xe4, 0xf6, 0xd9, 0x46, 0xb8, 0x4f, 0xbe, 0x21, 0x57, 0xa3, 0xef, 0x2b, 0x62, 0x6c,
];
const CLOCK: [u8; 16] = [
    0x4f, 0x31, 0x63, 0xcd, 0xba, 0x3f, 0x1b, 0x4a, 0x81, 0x2c, 0xef, 0x96, 0x35, 0x87, 0x28, 0xe7,
];
const PROPERTY: [u8; 16] = [
    0xeb, 0x8e, 0x6d, 0x88, 0xf2, 0x8c, 0x46, 0x44, 0x8d, 2, 0xcd, 0xba, 0x1d, 0xbd, 0xcf, 0x99,
];
#[repr(transparent)]
struct Table<const N: usize>([*const c_void; N]);
unsafe impl<const N: usize> Sync for Table<N> {}
macro_rules! table {($name:ident, $($f:path),+ $(,)?)=>{pub(super) static $name:Table<{[$(stringify!($f)),+].len()}>=Table([$($f as *const c_void),+]);}}
use table;
#[repr(C)]
struct Object {
    vt: *const c_void,
    endpoint: *const c_void,
    refs: AtomicU32,
    kind: Kind,
}
enum Kind {
    Enumerator(Mutex<Vec<usize>>),
    Collection(bool),
    Device,
    Properties,
    Client(Arc<Mutex<client::State>>),
    Render(Arc<Mutex<client::State>>),
    Clock(Arc<Mutex<client::State>>),
    Adjustment(Arc<Mutex<client::State>>),
}
fn object<const N: usize>(vt: &'static Table<N>, kind: Kind) -> *mut c_void {
    Box::into_raw(Box::new(Object {
        vt: vt.0.as_ptr().cast(),
        endpoint: endpoint::ENDPOINT_TABLE.0.as_ptr().cast(),
        refs: AtomicU32::new(1),
        kind,
    }))
    .cast()
}
unsafe fn is(iid: *const u8, expected: &[u8; 16]) -> bool {
    !iid.is_null() && unsafe { std::slice::from_raw_parts(iid, 16) } == expected
}
unsafe extern "win64" fn qi(this: *mut Object, iid: *const u8, out: *mut *mut c_void) -> H {
    if out.is_null() {
        return POINTER;
    }
    unsafe { *out = ptr::null_mut() };
    let own = match unsafe { &(*this).kind } {
        Kind::Enumerator(_) => &ENUM,
        Kind::Collection(_) => &COLLECTION,
        Kind::Device => &DEVICE,
        Kind::Properties => &PROPERTY,
        Kind::Client(_) => &CLIENT,
        Kind::Render(_) => &RENDER,
        Kind::Clock(_) => &CLOCK,
        Kind::Adjustment(_) => &ADJUSTMENT,
    };
    if unsafe { is(iid, &UNKNOWN) || is(iid, own) } {
        unsafe {
            *out = this.cast();
            addref(this)
        };
        return 0;
    }
    if matches!(unsafe { &(*this).kind }, Kind::Device) && unsafe { is(iid, &ENDPOINT) } {
        unsafe {
            *out = ptr::addr_of_mut!((*this).endpoint).cast();
            addref(this)
        };
        return 0;
    }
    NO_INTERFACE
}
unsafe extern "win64" fn addref(this: *mut Object) -> u32 {
    unsafe { (*this).refs.fetch_add(1, Ordering::Relaxed) + 1 }
}
unsafe extern "win64" fn release(this: *mut Object) -> u32 {
    let left = unsafe { (*this).refs.fetch_sub(1, Ordering::AcqRel) - 1 };
    if left == 0 {
        let object = unsafe { Box::from_raw(this) };
        if let Kind::Enumerator(callbacks) = &object.kind {
            for callback in callbacks.lock().unwrap().drain(..) {
                unsafe { callback_ref(callback, 2) };
            }
        }
    }
    left
}
unsafe fn callback_ref(p: usize, slot: usize) {
    let vt = unsafe { *(p as *const *const usize) };
    let f: unsafe extern "win64" fn(usize) -> u32 = unsafe { std::mem::transmute(*vt.add(slot)) };
    unsafe { f(p) };
}
pub(crate) unsafe fn create(iid: *const u8, out: *mut *mut c_void) -> H {
    let p = object(
        &endpoint::ENUM_TABLE,
        Kind::Enumerator(Mutex::new(Vec::new())),
    )
    .cast::<Object>();
    let result = unsafe { qi(p, iid, out) };
    unsafe { release(p) };
    result
}
fn text(value: &str) -> *mut u16 {
    let units = value.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let p = crate::api::com::apartment::allocate(units.len() * 2).cast::<u16>();
    if !p.is_null() {
        unsafe { ptr::copy_nonoverlapping(units.as_ptr(), p, units.len()) };
    }
    p
}
