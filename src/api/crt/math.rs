//! Typed Windows ABI wrappers around host libm.
#[link(name = "m")]
unsafe extern "C" {
    #[link_name = "acos"]
    fn host_acos(x: f64) -> f64;
    #[link_name = "acosf"]
    fn host_acosf(x: f32) -> f32;
    #[link_name = "asin"]
    fn host_asin(x: f64) -> f64;
    #[link_name = "asinf"]
    fn host_asinf(x: f32) -> f32;
    #[link_name = "atan"]
    fn host_atan(x: f64) -> f64;
    #[link_name = "atanf"]
    fn host_atanf(x: f32) -> f32;
    #[link_name = "ceil"]
    fn host_ceil(x: f64) -> f64;
    #[link_name = "ceilf"]
    fn host_ceilf(x: f32) -> f32;
    #[link_name = "cos"]
    fn host_cos(x: f64) -> f64;
    #[link_name = "cosf"]
    fn host_cosf(x: f32) -> f32;
    #[link_name = "cosh"]
    fn host_cosh(x: f64) -> f64;
    #[link_name = "coshf"]
    fn host_coshf(x: f32) -> f32;
    #[link_name = "exp"]
    fn host_exp(x: f64) -> f64;
    #[link_name = "expf"]
    fn host_expf(x: f32) -> f32;
    #[link_name = "floor"]
    fn host_floor(x: f64) -> f64;
    #[link_name = "floorf"]
    fn host_floorf(x: f32) -> f32;
    #[link_name = "log"]
    fn host_log(x: f64) -> f64;
    #[link_name = "logf"]
    fn host_logf(x: f32) -> f32;
    #[link_name = "log10"]
    fn host_log10(x: f64) -> f64;
    #[link_name = "log10f"]
    fn host_log10f(x: f32) -> f32;
    #[link_name = "sin"]
    fn host_sin(x: f64) -> f64;
    #[link_name = "sinf"]
    fn host_sinf(x: f32) -> f32;
    #[link_name = "sinh"]
    fn host_sinh(x: f64) -> f64;
    #[link_name = "sinhf"]
    fn host_sinhf(x: f32) -> f32;
    #[link_name = "tan"]
    fn host_tan(x: f64) -> f64;
    #[link_name = "tanh"]
    fn host_tanh(x: f64) -> f64;
    #[link_name = "atan2"]
    fn host_atan2(x: f64, y: f64) -> f64;
    #[link_name = "atan2f"]
    fn host_atan2f(x: f32, y: f32) -> f32;
    #[link_name = "fmod"]
    fn host_fmod(x: f64, y: f64) -> f64;
    #[link_name = "fmodf"]
    fn host_fmodf(x: f32, y: f32) -> f32;
    #[link_name = "pow"]
    fn host_pow(x: f64, y: f64) -> f64;
    #[link_name = "hypot"]
    fn host_hypot(x: f64, y: f64) -> f64;
    #[link_name = "nextafter"]
    fn host_nextafter(x: f64, y: f64) -> f64;
    #[link_name = "ldexp"]
    fn host_ldexp(x: f64, n: i32) -> f64;
}
pub extern "win64" fn acos(x: f64) -> f64 {
    unsafe { host_acos(x) }
}
pub extern "win64" fn acosf(x: f32) -> f32 {
    unsafe { host_acosf(x) }
}
pub extern "win64" fn asin(x: f64) -> f64 {
    unsafe { host_asin(x) }
}
pub extern "win64" fn asinf(x: f32) -> f32 {
    unsafe { host_asinf(x) }
}
pub extern "win64" fn atan(x: f64) -> f64 {
    unsafe { host_atan(x) }
}
pub extern "win64" fn atanf(x: f32) -> f32 {
    unsafe { host_atanf(x) }
}
pub extern "win64" fn ceil(x: f64) -> f64 {
    unsafe { host_ceil(x) }
}
pub extern "win64" fn ceilf(x: f32) -> f32 {
    unsafe { host_ceilf(x) }
}
pub extern "win64" fn cos(x: f64) -> f64 {
    unsafe { host_cos(x) }
}
pub extern "win64" fn cosf(x: f32) -> f32 {
    unsafe { host_cosf(x) }
}
pub extern "win64" fn cosh(x: f64) -> f64 {
    unsafe { host_cosh(x) }
}
pub extern "win64" fn coshf(x: f32) -> f32 {
    unsafe { host_coshf(x) }
}
pub extern "win64" fn exp(x: f64) -> f64 {
    unsafe { host_exp(x) }
}
pub extern "win64" fn expf(x: f32) -> f32 {
    unsafe { host_expf(x) }
}
pub extern "win64" fn floor(x: f64) -> f64 {
    unsafe { host_floor(x) }
}
pub extern "win64" fn floorf(x: f32) -> f32 {
    unsafe { host_floorf(x) }
}
pub extern "win64" fn log(x: f64) -> f64 {
    unsafe { host_log(x) }
}
pub extern "win64" fn logf(x: f32) -> f32 {
    unsafe { host_logf(x) }
}
pub extern "win64" fn log10(x: f64) -> f64 {
    unsafe { host_log10(x) }
}
pub extern "win64" fn log10f(x: f32) -> f32 {
    unsafe { host_log10f(x) }
}
pub extern "win64" fn sin(x: f64) -> f64 {
    unsafe { host_sin(x) }
}
pub extern "win64" fn sinf(x: f32) -> f32 {
    unsafe { host_sinf(x) }
}
pub extern "win64" fn sinh(x: f64) -> f64 {
    unsafe { host_sinh(x) }
}
pub extern "win64" fn sinhf(x: f32) -> f32 {
    unsafe { host_sinhf(x) }
}
pub extern "win64" fn tan(x: f64) -> f64 {
    unsafe { host_tan(x) }
}
pub extern "win64" fn tanh(x: f64) -> f64 {
    unsafe { host_tanh(x) }
}
pub extern "win64" fn atan2(x: f64, y: f64) -> f64 {
    unsafe { host_atan2(x, y) }
}
pub extern "win64" fn atan2f(x: f32, y: f32) -> f32 {
    unsafe { host_atan2f(x, y) }
}
pub extern "win64" fn fmod(x: f64, y: f64) -> f64 {
    unsafe { host_fmod(x, y) }
}
pub extern "win64" fn fmodf(x: f32, y: f32) -> f32 {
    unsafe { host_fmodf(x, y) }
}
pub extern "win64" fn pow(x: f64, y: f64) -> f64 {
    unsafe { host_pow(x, y) }
}
pub extern "win64" fn hypot(x: f64, y: f64) -> f64 {
    unsafe { host_hypot(x, y) }
}
pub extern "win64" fn nextafter(x: f64, y: f64) -> f64 {
    unsafe { host_nextafter(x, y) }
}
pub extern "win64" fn ldexp(x: f64, n: i32) -> f64 {
    unsafe { host_ldexp(x, n) }
}
unsafe extern "C" {
    #[link_name = "tanf"]
    fn host_tanf(x: f32) -> f32;
}
pub extern "win64" fn tanf(x: f32) -> f32 {
    unsafe { host_tanf(x) }
}

unsafe extern "C" {
    #[link_name = "trunc"]
    fn host_trunc(x: f64) -> f64;
    #[link_name = "round"]
    fn host_round(x: f64) -> f64;
    #[link_name = "truncf"]
    fn host_truncf(x: f32) -> f32;
    #[link_name = "roundf"]
    fn host_roundf(x: f32) -> f32;
    #[link_name = "exp2f"]
    fn host_exp2f(x: f32) -> f32;
    #[link_name = "exp2"]
    fn host_exp2(x: f64) -> f64;
}
pub extern "win64" fn trunc(x: f64) -> f64 {
    unsafe { host_trunc(x) }
}
pub extern "win64" fn round(x: f64) -> f64 {
    unsafe { host_round(x) }
}
pub extern "win64" fn truncf(x: f32) -> f32 {
    unsafe { host_truncf(x) }
}
pub extern "win64" fn roundf(x: f32) -> f32 {
    unsafe { host_roundf(x) }
}
pub extern "win64" fn exp2f(x: f32) -> f32 {
    unsafe { host_exp2f(x) }
}
pub extern "win64" fn exp2(x: f64) -> f64 {
    unsafe { host_exp2(x) }
}

unsafe extern "C" {
    #[link_name = "hypotf"]
    fn host_hypotf(x: f32, y: f32) -> f32;
    #[link_name = "cbrtf"]
    fn host_cbrtf(x: f32) -> f32;
    #[link_name = "fma"]
    fn host_fma(x: f64, y: f64, z: f64) -> f64;
    #[link_name = "fmaf"]
    fn host_fmaf(x: f32, y: f32, z: f32) -> f32;
    #[link_name = "log1p"]
    fn host_log1p(x: f64) -> f64;
    #[link_name = "log1pf"]
    fn host_log1pf(x: f32) -> f32;
    #[link_name = "log2"]
    fn host_log2(x: f64) -> f64;
    #[link_name = "log2f"]
    fn host_log2f(x: f32) -> f32;
    #[link_name = "tanhf"]
    fn host_tanhf(x: f32) -> f32;
}

pub extern "win64" fn hypotf(x: f32, y: f32) -> f32 {
    unsafe { host_hypotf(x, y) }
}
pub extern "win64" fn cbrtf(x: f32) -> f32 {
    unsafe { host_cbrtf(x) }
}
pub extern "win64" fn fma(x: f64, y: f64, z: f64) -> f64 {
    unsafe { host_fma(x, y, z) }
}
pub extern "win64" fn fmaf(x: f32, y: f32, z: f32) -> f32 {
    unsafe { host_fmaf(x, y, z) }
}
pub extern "win64" fn log1p(x: f64) -> f64 {
    unsafe { host_log1p(x) }
}
pub extern "win64" fn log1pf(x: f32) -> f32 {
    unsafe { host_log1pf(x) }
}
pub extern "win64" fn log2(x: f64) -> f64 {
    unsafe { host_log2(x) }
}
pub extern "win64" fn log2f(x: f32) -> f32 {
    unsafe { host_log2f(x) }
}
pub extern "win64" fn tanhf(x: f32) -> f32 {
    unsafe { host_tanhf(x) }
}
