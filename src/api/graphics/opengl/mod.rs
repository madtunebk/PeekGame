//! Windows WGL handles and typed OpenGL entry points over native GLX.
mod contexts;
mod extensions;
mod pixel;
pub(crate) mod thunks {
    include!(concat!(env!("OUT_DIR"), "/gl_ffi.rs"));
}
pub(crate) use contexts::*;
pub(crate) use extensions::*;
pub(crate) use pixel::*;
fn fail<T>(code: u32, value: T) -> T {
    if std::env::var_os("PEEKGAME_TRACE_GL").is_some() {
        eprintln!("OpenGL: failed with Windows error {code}");
    }
    crate::api::system::error::set(code);
    value
}

fn trace(args: std::fmt::Arguments) {
    if std::env::var_os("PEEKGAME_TRACE_GL").is_some() {
        eprintln!("OpenGL: {args}");
    }
}
