use std::ffi::{c_char, c_void};
unsafe extern "C" {
    #[link_name = "malloc"]
    fn host_0(size: usize) -> *mut c_void;
}
pub unsafe extern "win64" fn call_0(size: usize) -> *mut c_void {
    unsafe { host_0(size) }
}
unsafe extern "C" {
    #[link_name = "free"]
    fn host_1(ptr: *mut c_void);
}
pub unsafe extern "win64" fn call_1(ptr: *mut c_void) {
    unsafe { host_1(ptr) }
}
unsafe extern "C" {
    #[link_name = "calloc"]
    fn host_2(count: usize, size: usize) -> *mut c_void;
}
pub unsafe extern "win64" fn call_2(count: usize, size: usize) -> *mut c_void {
    unsafe { host_2(count, size) }
}
unsafe extern "C" {
    #[link_name = "realloc"]
    fn host_3(ptr: *mut c_void, size: usize) -> *mut c_void;
}
pub unsafe extern "win64" fn call_3(ptr: *mut c_void, size: usize) -> *mut c_void {
    unsafe { host_3(ptr, size) }
}
unsafe extern "C" {
    #[link_name = "memcpy"]
    fn host_4(dst: *mut c_void, src: *const c_void, size: usize) -> *mut c_void;
}
pub unsafe extern "win64" fn call_4(
    dst: *mut c_void,
    src: *const c_void,
    size: usize,
) -> *mut c_void {
    unsafe { host_4(dst, src, size) }
}
unsafe extern "C" {
    #[link_name = "memmove"]
    fn host_5(dst: *mut c_void, src: *const c_void, size: usize) -> *mut c_void;
}
pub unsafe extern "win64" fn call_5(
    dst: *mut c_void,
    src: *const c_void,
    size: usize,
) -> *mut c_void {
    unsafe { host_5(dst, src, size) }
}
unsafe extern "C" {
    #[link_name = "memset"]
    fn host_6(dst: *mut c_void, value: i32, size: usize) -> *mut c_void;
}
pub unsafe extern "win64" fn call_6(dst: *mut c_void, value: i32, size: usize) -> *mut c_void {
    unsafe { host_6(dst, value, size) }
}
unsafe extern "C" {
    #[link_name = "memcmp"]
    fn host_7(a: *const c_void, b: *const c_void, size: usize) -> i32;
}
pub unsafe extern "win64" fn call_7(a: *const c_void, b: *const c_void, size: usize) -> i32 {
    unsafe { host_7(a, b, size) }
}
unsafe extern "C" {
    #[link_name = "memchr"]
    fn host_8(a: *const c_void, value: i32, size: usize) -> *mut c_void;
}
pub unsafe extern "win64" fn call_8(a: *const c_void, value: i32, size: usize) -> *mut c_void {
    unsafe { host_8(a, value, size) }
}
unsafe extern "C" {
    #[link_name = "strlen"]
    fn host_9(text: *const c_char) -> usize;
}
pub unsafe extern "win64" fn call_9(text: *const c_char) -> usize {
    unsafe { host_9(text) }
}
unsafe extern "C" {
    #[link_name = "strcmp"]
    fn host_10(a: *const c_char, b: *const c_char) -> i32;
}
pub unsafe extern "win64" fn call_10(a: *const c_char, b: *const c_char) -> i32 {
    unsafe { host_10(a, b) }
}
unsafe extern "C" {
    #[link_name = "strncmp"]
    fn host_11(a: *const c_char, b: *const c_char, n: usize) -> i32;
}
pub unsafe extern "win64" fn call_11(a: *const c_char, b: *const c_char, n: usize) -> i32 {
    unsafe { host_11(a, b, n) }
}
unsafe extern "C" {
    #[link_name = "strcpy"]
    fn host_12(dst: *mut c_char, src: *const c_char) -> *mut c_char;
}
pub unsafe extern "win64" fn call_12(dst: *mut c_char, src: *const c_char) -> *mut c_char {
    unsafe { host_12(dst, src) }
}
unsafe extern "C" {
    #[link_name = "strncpy"]
    fn host_13(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
}
pub unsafe extern "win64" fn call_13(
    dst: *mut c_char,
    src: *const c_char,
    n: usize,
) -> *mut c_char {
    unsafe { host_13(dst, src, n) }
}
unsafe extern "C" {
    #[link_name = "strcat"]
    fn host_14(dst: *mut c_char, src: *const c_char) -> *mut c_char;
}
pub unsafe extern "win64" fn call_14(dst: *mut c_char, src: *const c_char) -> *mut c_char {
    unsafe { host_14(dst, src) }
}
unsafe extern "C" {
    #[link_name = "strncat"]
    fn host_15(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
}
pub unsafe extern "win64" fn call_15(
    dst: *mut c_char,
    src: *const c_char,
    n: usize,
) -> *mut c_char {
    unsafe { host_15(dst, src, n) }
}
unsafe extern "C" {
    #[link_name = "strchr"]
    fn host_16(text: *const c_char, value: i32) -> *mut c_char;
}
pub unsafe extern "win64" fn call_16(text: *const c_char, value: i32) -> *mut c_char {
    unsafe { host_16(text, value) }
}
unsafe extern "C" {
    #[link_name = "strrchr"]
    fn host_17(text: *const c_char, value: i32) -> *mut c_char;
}
pub unsafe extern "win64" fn call_17(text: *const c_char, value: i32) -> *mut c_char {
    unsafe { host_17(text, value) }
}
unsafe extern "C" {
    #[link_name = "strstr"]
    fn host_18(text: *const c_char, needle: *const c_char) -> *mut c_char;
}
pub unsafe extern "win64" fn call_18(text: *const c_char, needle: *const c_char) -> *mut c_char {
    unsafe { host_18(text, needle) }
}
unsafe extern "C" {
    #[link_name = "strpbrk"]
    fn host_19(text: *const c_char, accept: *const c_char) -> *mut c_char;
}
pub unsafe extern "win64" fn call_19(text: *const c_char, accept: *const c_char) -> *mut c_char {
    unsafe { host_19(text, accept) }
}
unsafe extern "C" {
    #[link_name = "strcspn"]
    fn host_20(text: *const c_char, reject: *const c_char) -> usize;
}
pub unsafe extern "win64" fn call_20(text: *const c_char, reject: *const c_char) -> usize {
    unsafe { host_20(text, reject) }
}
unsafe extern "C" {
    #[link_name = "strdup"]
    fn host_21(text: *const c_char) -> *mut c_char;
}
pub unsafe extern "win64" fn call_21(text: *const c_char) -> *mut c_char {
    unsafe { host_21(text) }
}
unsafe extern "C" {
    #[link_name = "atoi"]
    fn host_22(text: *const c_char) -> i32;
}
pub unsafe extern "win64" fn call_22(text: *const c_char) -> i32 {
    unsafe { host_22(text) }
}
unsafe extern "C" {
    #[link_name = "atof"]
    fn host_23(text: *const c_char) -> f64;
}
pub unsafe extern "win64" fn call_23(text: *const c_char) -> f64 {
    unsafe { host_23(text) }
}

unsafe extern "C" {
    #[link_name = "getenv"]
    fn host_getenv(name: *const c_char) -> *mut c_char;
}
pub unsafe extern "win64" fn environment(name: *const c_char) -> *mut c_char {
    unsafe { host_getenv(name) }
}
