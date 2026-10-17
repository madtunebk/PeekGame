//! PulseAudio client API; PipeWire's pulse server uses the same protocol.
//! Only this module owns host audio handles. Windows code receives COM objects.
use std::{
    ffi::{c_char, c_void, CStr},
    ptr,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};
type P = *mut c_void;
#[repr(C)]
struct SampleSpec {
    format: i32,
    rate: u32,
    channels: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct BufferAttr {
    maxlength: u32,
    tlength: u32,
    prebuf: u32,
    minreq: u32,
    fragsize: u32,
}
#[link(name = "pulse")]
unsafe extern "C" {
    fn pa_threaded_mainloop_new() -> P;
    fn pa_threaded_mainloop_free(p: P);
    fn pa_threaded_mainloop_start(p: P) -> i32;
    fn pa_threaded_mainloop_stop(p: P);
    fn pa_threaded_mainloop_lock(p: P);
    fn pa_threaded_mainloop_unlock(p: P);
    fn pa_threaded_mainloop_wait(p: P);
    fn pa_threaded_mainloop_signal(p: P, accept: i32);
    fn pa_threaded_mainloop_get_api(p: P) -> P;
    fn pa_context_new(api: P, name: *const c_char) -> P;
    fn pa_context_set_state_callback(p: P, callback: unsafe extern "C" fn(P, P), data: P);
    fn pa_context_connect(p: P, server: *const c_char, flags: i32, spawn: P) -> i32;
    fn pa_context_get_state(p: P) -> i32;
    fn pa_context_disconnect(p: P);
    fn pa_context_unref(p: P);
    fn pa_stream_new(context: P, name: *const c_char, spec: *const SampleSpec, map: P) -> P;
    fn pa_stream_set_state_callback(p: P, callback: unsafe extern "C" fn(P, P), data: P);
    fn pa_stream_set_write_callback(p: P, callback: unsafe extern "C" fn(P, usize, P), data: P);
    fn pa_stream_connect_playback(
        p: P,
        device: *const c_char,
        attr: *const BufferAttr,
        flags: i32,
        volume: P,
        sync: P,
    ) -> i32;
    fn pa_stream_get_state(p: P) -> i32;
    fn pa_stream_get_buffer_attr(p: P) -> *const BufferAttr;
    fn pa_stream_get_device_name(p: P) -> *const c_char;
    fn pa_stream_writable_size(p: P) -> usize;
    fn pa_stream_write(
        p: P,
        data: *const c_void,
        len: usize,
        free: P,
        offset: i64,
        seek: i32,
    ) -> i32;
    fn pa_stream_cork(p: P, cork: i32, cb: unsafe extern "C" fn(P, i32, P), data: P) -> P;
    fn pa_stream_update_sample_rate(
        p: P,
        rate: u32,
        cb: unsafe extern "C" fn(P, i32, P),
        data: P,
    ) -> P;
    fn pa_stream_flush(p: P, cb: unsafe extern "C" fn(P, i32, P), data: P) -> P;
    fn pa_stream_disconnect(p: P) -> i32;
    fn pa_stream_unref(p: P);
    fn pa_operation_unref(p: P);
    fn pa_operation_cancel(p: P);
}
struct Callback {
    event: AtomicUsize,
    started: AtomicBool,
}
unsafe extern "C" fn changed(_: P, data: P) {
    unsafe {
        pa_threaded_mainloop_signal(data, 0);
    }
}
unsafe extern "C" fn writable(_: P, _: usize, data: P) {
    let callback = unsafe { &*data.cast::<Callback>() };
    let event = callback.event.load(Ordering::Acquire);
    if event != 0 && callback.started.load(Ordering::Acquire) {
        crate::api::threading::events::set(event);
    }
}
struct Completion {
    mainloop: P,
    result: i32,
}
unsafe extern "C" fn completed(_: P, success: i32, data: P) {
    let completion = unsafe { &mut *data.cast::<Completion>() };
    completion.result = success;
    unsafe {
        pa_threaded_mainloop_signal(completion.mainloop, 0);
    }
}
pub struct Playback {
    mainloop: P,
    context: P,
    stream: P,
    callback: Box<Callback>,
    pub frames: u32,
    align: u32,
    pub name: String,
}
// Access is serialized by AudioClient's mutex; callbacks only use the atomic event.
unsafe impl Send for Playback {}
struct Lock(P);
impl Drop for Lock {
    fn drop(&mut self) {
        unsafe {
            pa_threaded_mainloop_unlock(self.0);
        }
    }
}
impl Playback {
    fn lock(&self) -> Lock {
        unsafe {
            pa_threaded_mainloop_lock(self.mainloop);
        }
        Lock(self.mainloop)
    }
    pub fn open(
        rate: u32,
        channels: u16,
        format: i32,
        align: u32,
        requested_frames: u32,
        variable_rate: bool,
    ) -> Result<Self, ()> {
        unsafe {
            let mainloop = pa_threaded_mainloop_new();
            if mainloop.is_null() {
                return Err(());
            }
            if pa_threaded_mainloop_start(mainloop) < 0 {
                pa_threaded_mainloop_free(mainloop);
                return Err(());
            }
            let mut output = Self {
                mainloop,
                context: ptr::null_mut(),
                stream: ptr::null_mut(),
                callback: Box::new(Callback {
                    event: AtomicUsize::new(0),
                    started: AtomicBool::new(false),
                }),
                frames: 0,
                align,
                name: String::new(),
            };
            let _lock = output.lock();
            output.context =
                pa_context_new(pa_threaded_mainloop_get_api(mainloop), c"PeekGame".as_ptr());
            if output.context.is_null() {
                return Err(());
            }
            pa_context_set_state_callback(output.context, changed, mainloop);
            if pa_context_connect(output.context, ptr::null(), 1, ptr::null_mut()) < 0 {
                return Err(());
            }
            loop {
                match pa_context_get_state(output.context) {
                    4 => break,
                    5 | 6 => return Err(()),
                    _ => pa_threaded_mainloop_wait(mainloop),
                }
            }
            let spec = SampleSpec {
                format,
                rate,
                channels: channels as u8,
            };
            output.stream = pa_stream_new(
                output.context,
                c"Windows shared playback".as_ptr(),
                &spec,
                ptr::null_mut(),
            );
            if output.stream.is_null() {
                return Err(());
            }
            pa_stream_set_state_callback(output.stream, changed, mainloop);
            pa_stream_set_write_callback(
                output.stream,
                writable,
                (&mut *output.callback as *mut Callback).cast(),
            );
            let bytes = requested_frames.checked_mul(output.align).ok_or(())?;
            let attr = BufferAttr {
                maxlength: bytes,
                tlength: bytes,
                prebuf: 0,
                minreq: (rate / 100).max(1) * output.align,
                fragsize: u32::MAX,
            };
            // START_CORKED, INTERPOLATE_TIMING, AUTO_TIMING_UPDATE, ADJUST_LATENCY.
            if pa_stream_connect_playback(
                output.stream,
                ptr::null(),
                &attr,
                0x200b | if variable_rate { 0x400 } else { 0 },
                ptr::null_mut(),
                ptr::null_mut(),
            ) < 0
            {
                return Err(());
            }
            loop {
                match pa_stream_get_state(output.stream) {
                    2 => break,
                    3 | 4 => return Err(()),
                    _ => pa_threaded_mainloop_wait(mainloop),
                }
            }
            let actual = pa_stream_get_buffer_attr(output.stream);
            if actual.is_null() {
                return Err(());
            }
            output.frames = (*actual).tlength / output.align;
            if output.frames == 0 || output.frames > rate * 2 {
                return Err(());
            }
            let name = pa_stream_get_device_name(output.stream);
            if !name.is_null() {
                output.name = CStr::from_ptr(name).to_string_lossy().into_owned();
            }
            drop(_lock);
            Ok(output)
        }
    }
    pub fn set_event(&self, event: usize) {
        let previous = self.callback.event.swap(event, Ordering::AcqRel);
        if previous != 0 {
            crate::api::threading::events::close(previous);
        }
    }
    pub fn padding(&self) -> Result<u32, ()> {
        let _lock = self.lock();
        let writable = unsafe { pa_stream_writable_size(self.stream) };
        if writable == usize::MAX {
            return Err(());
        }
        Ok(self
            .frames
            .saturating_sub((writable / self.align as usize).min(self.frames as usize) as u32))
    }
    pub fn write(&self, data: &[u8]) -> Result<(), ()> {
        let _lock = self.lock();
        if unsafe {
            pa_stream_write(
                self.stream,
                data.as_ptr().cast(),
                data.len(),
                ptr::null_mut(),
                0,
                0,
            )
        } < 0
        {
            Err(())
        } else {
            Ok(())
        }
    }
    pub fn cork(&self, stopped: bool) -> Result<(), ()> {
        self.operation(Some(stopped), None)?;
        self.callback.started.store(!stopped, Ordering::Release);
        Ok(())
    }
    pub fn flush(&self) -> Result<(), ()> {
        self.operation(None, None)
    }
    pub fn set_rate(&self, rate: u32) -> Result<(), ()> {
        self.operation(None, Some(rate))
    }
    fn operation(&self, cork: Option<bool>, rate: Option<u32>) -> Result<(), ()> {
        let _lock = self.lock();
        let mut completion = Completion {
            mainloop: self.mainloop,
            result: -1,
        };
        let data = (&mut completion as *mut Completion).cast();
        unsafe {
            let operation = if let Some(rate) = rate {
                pa_stream_update_sample_rate(self.stream, rate, completed, data)
            } else {
                match cork {
                    Some(value) => pa_stream_cork(self.stream, value as i32, completed, data),
                    None => pa_stream_flush(self.stream, completed, data),
                }
            };
            if operation.is_null() {
                return Err(());
            }
            while completion.result < 0 && pa_stream_get_state(self.stream) == 2 {
                pa_threaded_mainloop_wait(self.mainloop);
            }
            // If the server disconnects the operation can no longer invoke its callback.
            pa_operation_cancel(operation);
            pa_operation_unref(operation);
        }
        if completion.result == 1 {
            Ok(())
        } else {
            Err(())
        }
    }
}
impl Drop for Playback {
    fn drop(&mut self) {
        unsafe {
            // Stop callback dispatch before destroying callback data or event handles.
            pa_threaded_mainloop_stop(self.mainloop);
            if !self.stream.is_null() {
                pa_stream_disconnect(self.stream);
                pa_stream_unref(self.stream);
            }
            if !self.context.is_null() {
                pa_context_disconnect(self.context);
                pa_context_unref(self.context);
            }
            pa_threaded_mainloop_free(self.mainloop);
            let event = self.callback.event.load(Ordering::Relaxed);
            if event != 0 {
                crate::api::threading::events::close(event);
            }
        }
    }
}
