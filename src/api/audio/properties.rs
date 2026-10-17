//! Read-only default endpoint property store (CoTaskMem-owned strings).
use super::*;
const DEVICE_KEY: [u8; 16] = [
    0x4e, 0x25, 0x5c, 0xa4, 0x1c, 0xdf, 0xfd, 0x4e, 0x80, 0x20, 0x67, 0xd1, 0x46, 0xa8, 0x50, 0xe0,
];
const INTERFACE_KEY: [u8; 16] = [
    0x6e, 0x51, 0x6e, 2, 0x14, 0xb8, 0x4b, 0x41, 0x83, 0xcd, 0x85, 0x6d, 0x6f, 0xef, 0x48, 0x22,
];
const KEYS: [([u8; 16], u32); 3] = [(DEVICE_KEY, 14), (DEVICE_KEY, 2), (INTERFACE_KEY, 2)];
unsafe extern "win64" fn count(_: *mut Object, out: *mut u32) -> H {
    if out.is_null() {
        return POINTER;
    }
    unsafe { *out = 3 };
    0
}
unsafe extern "win64" fn at(_: *mut Object, index: u32, out: *mut u8) -> H {
    if out.is_null() {
        return POINTER;
    }
    let Some((guid, id)) = KEYS.get(index as usize) else {
        return ARG;
    };
    unsafe {
        ptr::copy_nonoverlapping(guid.as_ptr(), out, 16);
        out.add(16).cast::<u32>().write_unaligned(*id)
    };
    0
}
unsafe extern "win64" fn get(_: *mut Object, key: *const u8, out: *mut u8) -> H {
    if key.is_null() || out.is_null() {
        return POINTER;
    }
    unsafe { out.write_bytes(0, 24) };
    let id = unsafe { key.add(16).cast::<u32>().read_unaligned() };
    if KEYS
        .iter()
        .any(|(guid, pid)| *pid == id && unsafe { is(key, guid) })
    {
        let name = text("PeekGame default playback (PulseAudio/PipeWire)");
        if name.is_null() {
            return 0x8007000eu32 as H;
        }
        unsafe {
            out.cast::<u16>().write_unaligned(31);
            out.add(8).cast::<*mut u16>().write_unaligned(name)
        };
    }
    0
}
unsafe extern "win64" fn set(_: *mut Object, _key: *const u8, _value: *const u8) -> H {
    0x80030005u32 as H
}
unsafe extern "win64" fn commit(_: *mut Object) -> H {
    0x80030005u32 as H
}
table!(
    PROPERTY_TABLE,
    qi,
    addref,
    release,
    count,
    at,
    get,
    set,
    commit
);
