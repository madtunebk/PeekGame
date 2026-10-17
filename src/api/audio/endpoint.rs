//! Device discovery and COM endpoint interfaces. One stable default render endpoint.
use super::*;
const ID: &str = "peekgame:default-playback";
fn available() -> bool {
    crate::host::pulse::Playback::open(48000, 2, 5, 8, 2400, false).is_ok()
}
fn device() -> *mut c_void {
    object(&DEVICE_TABLE, Kind::Device)
}
unsafe extern "win64" fn enumerate(
    _: *mut Object,
    flow: i32,
    mask: u32,
    out: *mut *mut c_void,
) -> H {
    if out.is_null() {
        return POINTER;
    }
    unsafe { *out = ptr::null_mut() };
    if !(0..=2).contains(&flow) || mask == 0 || mask & !15 != 0 {
        return ARG;
    }
    let active = flow != 1 && mask & 1 != 0 && available();
    unsafe { *out = object(&COLLECTION_TABLE, Kind::Collection(active)) };
    0
}
unsafe extern "win64" fn default(_: *mut Object, flow: i32, role: i32, out: *mut *mut c_void) -> H {
    if out.is_null() {
        return POINTER;
    }
    unsafe { *out = ptr::null_mut() };
    if !(0..=2).contains(&role) || !(0..=1).contains(&flow) {
        return ARG;
    }
    if flow == 1 || !available() {
        return 0x80070490u32 as H;
    }
    unsafe { *out = device() };
    0
}
unsafe extern "win64" fn get(_: *mut Object, id: *const u16, out: *mut *mut c_void) -> H {
    if out.is_null() || id.is_null() {
        return POINTER;
    }
    unsafe { *out = ptr::null_mut() };
    let expected = ID.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    for (i, unit) in expected.iter().enumerate() {
        if unsafe { *id.add(i) } != *unit {
            return 0x80070490u32 as H;
        }
    }
    if !available() {
        return 0x80070490u32 as H;
    }
    unsafe { *out = device() };
    0
}
unsafe extern "win64" fn register(this: *mut Object, callback: usize) -> H {
    if callback == 0 {
        return POINTER;
    }
    let Kind::Enumerator(callbacks) = (unsafe { &(*this).kind }) else {
        return ARG;
    };
    // The logical endpoint stays fixed; PulseAudio routes it when the host default changes.
    unsafe { callback_ref(callback, 1) };
    let mut list = callbacks.lock().unwrap();
    if list.contains(&callback) {
        drop(list);
        unsafe { callback_ref(callback, 2) };
        return ARG;
    }
    list.push(callback);
    0
}
unsafe extern "win64" fn unregister(this: *mut Object, callback: usize) -> H {
    let Kind::Enumerator(callbacks) = (unsafe { &(*this).kind }) else {
        return ARG;
    };
    let mut list = callbacks.lock().unwrap();
    let Some(i) = list.iter().position(|p| *p == callback) else {
        return 0x80070490u32 as H;
    };
    list.remove(i);
    drop(list);
    unsafe { callback_ref(callback, 2) };
    0
}
table!(ENUM_TABLE, qi, addref, release, enumerate, default, get, register, unregister);
unsafe extern "win64" fn count(this: *mut Object, out: *mut u32) -> H {
    if out.is_null() {
        return POINTER;
    }
    let Kind::Collection(active) = (unsafe { &(*this).kind }) else {
        return ARG;
    };
    unsafe { *out = *active as u32 };
    0
}
unsafe extern "win64" fn item(this: *mut Object, index: u32, out: *mut *mut c_void) -> H {
    if out.is_null() {
        return POINTER;
    }
    unsafe { *out = ptr::null_mut() };
    if !matches!(unsafe { &(*this).kind }, Kind::Collection(true)) || index != 0 {
        return ARG;
    }
    unsafe { *out = device() };
    0
}
table!(COLLECTION_TABLE, qi, addref, release, count, item);
unsafe extern "win64" fn activate(
    _: *mut Object,
    iid: *const u8,
    _context: u32,
    parameters: *const c_void,
    out: *mut *mut c_void,
) -> H {
    if out.is_null() {
        return POINTER;
    }
    unsafe { *out = ptr::null_mut() };
    if !parameters.is_null() {
        return NOT_IMPL;
    }
    if !unsafe { is(iid, &CLIENT) } {
        return NO_INTERFACE;
    }
    unsafe {
        *out = object(
            &client::CLIENT_TABLE,
            Kind::Client(Arc::new(Mutex::new(client::State::default()))),
        )
    };
    0
}
unsafe extern "win64" fn properties(_: *mut Object, access: u32, out: *mut *mut c_void) -> H {
    if out.is_null() {
        return POINTER;
    }
    unsafe { *out = ptr::null_mut() };
    if access != 0 {
        return 0x80070005u32 as H;
    }
    unsafe { *out = object(&properties::PROPERTY_TABLE, Kind::Properties) };
    0
}
unsafe extern "win64" fn id(_: *mut Object, out: *mut *mut u16) -> H {
    if out.is_null() {
        return POINTER;
    }
    unsafe { *out = text(ID) };
    if unsafe { *out }.is_null() {
        0x8007000eu32 as H
    } else {
        0
    }
}
unsafe extern "win64" fn state(_: *mut Object, out: *mut u32) -> H {
    if out.is_null() {
        return POINTER;
    }
    unsafe { *out = if available() { 1 } else { 4 } };
    0
}
table!(
    DEVICE_TABLE,
    qi,
    addref,
    release,
    activate,
    properties,
    id,
    state
);
unsafe fn owner(this: *mut u8) -> *mut Object {
    unsafe { this.sub(8).cast() }
}
unsafe extern "win64" fn endpoint_qi(this: *mut u8, iid: *const u8, out: *mut *mut c_void) -> H {
    unsafe { qi(owner(this), iid, out) }
}
unsafe extern "win64" fn endpoint_addref(this: *mut u8) -> u32 {
    unsafe { addref(owner(this)) }
}
unsafe extern "win64" fn endpoint_release(this: *mut u8) -> u32 {
    unsafe { release(owner(this)) }
}
unsafe extern "win64" fn flow(_: *mut u8, out: *mut i32) -> H {
    if out.is_null() {
        return POINTER;
    }
    unsafe { *out = 0 };
    0
}
table!(
    ENDPOINT_TABLE,
    endpoint_qi,
    endpoint_addref,
    endpoint_release,
    flow
);
