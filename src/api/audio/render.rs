//! Render buffers and the stream clock. State lives until the last service is released.
use super::*;
use client::state;
unsafe extern "win64" fn get(this: *mut Object, frames: u32, out: *mut *mut u8) -> H {
    if out.is_null() {
        return POINTER;
    }
    let mut s = unsafe { state(this) }.lock().unwrap();
    if s.pending.is_some() {
        return 0x88890007u32 as H;
    }
    let Some(stream) = &s.stream else {
        return NOT_INITIALIZED;
    };
    let available = match stream.padding() {
        Ok(p) => s.frames - p,
        Err(_) => return INVALIDATED,
    };
    if frames > available {
        return 0x88890006u32 as H;
    }
    let f = s.format.unwrap();
    let mut buffer = vec![f.silence; frames as usize * f.align as usize];
    if frames != 0 {
        unsafe { *out = buffer.as_mut_ptr() };
    }
    s.pending = Some(buffer);
    0
}
unsafe extern "win64" fn release_buffer(this: *mut Object, frames: u32, flags: u32) -> H {
    let mut s = unsafe { state(this) }.lock().unwrap();
    let Some(f) = s.format else {
        return NOT_INITIALIZED;
    };
    let Some(buffer) = s.pending.as_mut() else {
        return 0x88890007u32 as H;
    };
    let bytes = frames as usize * f.align as usize;
    if bytes > buffer.len() {
        return 0x88890009u32 as H;
    }
    if flags & !2 != 0 {
        return ARG;
    }
    if flags & 2 != 0 {
        buffer[..bytes].fill(f.silence);
    }
    let buffer = s.pending.take().unwrap();
    if bytes > 0 && s.stream.as_ref().unwrap().write(&buffer[..bytes]).is_err() {
        return INVALIDATED;
    }
    s.written += frames as u64;
    0
}
table!(RENDER_TABLE, qi, addref, release, get, release_buffer);
unsafe extern "win64" fn frequency(this: *mut Object, out: *mut u64) -> H {
    if out.is_null() {
        return POINTER;
    }
    let s = unsafe { state(this) }.lock().unwrap();
    let Some(f) = s.format else {
        return NOT_INITIALIZED;
    };
    unsafe { *out = f.rate as u64 };
    0
}
unsafe extern "win64" fn position(this: *mut Object, out: *mut u64, qpc: *mut u64) -> H {
    if out.is_null() {
        return POINTER;
    }
    let s = unsafe { state(this) }.lock().unwrap();
    let Some(stream) = &s.stream else {
        return NOT_INITIALIZED;
    };
    let padding = match stream.padding() {
        Ok(p) => p,
        Err(_) => return INVALIDATED,
    };
    unsafe { *out = s.written.saturating_sub(padding as u64) };
    if !qpc.is_null() {
        let mut ticks = 0;
        unsafe {
            crate::api::system::startup::performance_counter(&mut ticks);
            *qpc = ticks as u64 / 100
        };
    }
    0
}
unsafe extern "win64" fn characteristics(_: *mut Object, out: *mut u32) -> H {
    if out.is_null() {
        return POINTER;
    }
    unsafe { *out = 1 };
    0
}
table!(
    CLOCK_TABLE,
    qi,
    addref,
    release,
    frequency,
    position,
    characteristics
);
