#![no_std]
#![no_main]

use core::{ffi::c_void, panic::PanicInfo};

#[link(name = "winhttp")]
unsafe extern "system" {
    fn WinHttpOpen(agent: *const u16, access: u32, proxy: *const u16, bypass: *const u16, flags: u32) -> *mut c_void;
    fn WinHttpConnect(session: *mut c_void, server: *const u16, port: u16, reserved: u32) -> *mut c_void;
    fn WinHttpOpenRequest(connection: *mut c_void, verb: *const u16, object: *const u16, version: *const u16, referrer: *const u16, accept: *const *const u16, flags: u32) -> *mut c_void;
    fn WinHttpSendRequest(request: *mut c_void, headers: *const u16, headers_length: u32, optional: *const c_void, optional_length: u32, total_length: u32, context: usize) -> i32;
    fn WinHttpReceiveResponse(request: *mut c_void, reserved: *const c_void) -> i32;
    fn WinHttpQueryHeaders(request: *mut c_void, level: u32, name: *const u16, buffer: *mut c_void, length: *mut u32, index: *mut u32) -> i32;
    fn WinHttpReadData(request: *mut c_void, buffer: *mut c_void, length: u32, read: *mut u32) -> i32;
    fn WinHttpCloseHandle(handle: *mut c_void) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn ExitProcess(code: u32) -> !;
    fn GetLastError() -> u32;
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! { loop {} }

#[no_mangle]
pub extern "C" fn rust_eh_personality() {}

const AGENT: &[u16] = &[b'P' as u16, b'e' as u16, b'e' as u16, b'k' as u16, 0];
#[cfg(not(local_http))]
const HOST: &[u16] = &[b'e' as u16, b'x' as u16, b'a' as u16, b'm' as u16, b'p' as u16, b'l' as u16, b'e' as u16, b'.' as u16, b'c' as u16, b'o' as u16, b'm' as u16, 0];
#[cfg(local_http)]
const HOST: &[u16] = &[b'1' as u16, b'2' as u16, b'7' as u16, b'.' as u16, b'0' as u16, b'.' as u16, b'0' as u16, b'.' as u16, b'1' as u16, 0];
#[cfg(not(local_http))]
const PORT: u16 = 443;
#[cfg(local_http)]
const PORT: u16 = parse_port(env!("WINHTTP_PROBE_PORT").as_bytes());
#[cfg(not(local_http))]
const FLAGS: u32 = 0x0080_0000;
#[cfg(local_http)]
const FLAGS: u32 = 0;
#[cfg(local_http)]
const fn parse_port(bytes: &[u8]) -> u16 {
    let mut i = 0;
    let mut port = 0u16;
    while i < bytes.len() {
        port = port * 10 + (bytes[i] - b'0') as u16;
        i += 1;
    }
    port
}
const PATH: &[u16] = &[b'/' as u16, 0];
const GET: &[u16] = &[b'G' as u16, b'E' as u16, b'T' as u16, 0];

unsafe fn probe() -> u32 {
    let session = unsafe { WinHttpOpen(AGENT.as_ptr(), 1, core::ptr::null(), core::ptr::null(), 0) };
    if session.is_null() { return 1; }
    let connection = unsafe { WinHttpConnect(session, HOST.as_ptr(), PORT, 0) };
    if connection.is_null() { return 2; }
    let request = unsafe { WinHttpOpenRequest(connection, GET.as_ptr(), PATH.as_ptr(), core::ptr::null(), core::ptr::null(), core::ptr::null(), FLAGS) };
    if request.is_null() { return 3; }
    if unsafe { WinHttpSendRequest(request, core::ptr::null(), 0, core::ptr::null(), 0, 0, 0) } == 0 {
        return if unsafe { GetLastError() } == 12029 { 29 } else { 4 };
    }
    if unsafe { WinHttpReceiveResponse(request, core::ptr::null()) } == 0 { return 5; }
    let mut status: u32 = 0;
    let mut status_size: u32 = 4;
    if unsafe { WinHttpQueryHeaders(request, 0x2000_0013, core::ptr::null(), (&mut status as *mut u32).cast(), &mut status_size, core::ptr::null_mut()) } == 0 { return 6; }
    if status != 200 { return 7; }
    let mut body = [0u8; 256];
    let mut total = 0usize;
    loop {
        let mut read = 0u32;
        if unsafe { WinHttpReadData(request, body.as_mut_ptr().cast(), body.len() as u32, &mut read) } == 0 { return 8; }
        if read == 0 { break; }
        total += read as usize;
        if total > 1024 * 1024 { return 9; }
    }
    if total == 0 { return 10; }
    unsafe { WinHttpCloseHandle(request); WinHttpCloseHandle(connection); WinHttpCloseHandle(session); }
    0
}

#[no_mangle]
pub extern "system" fn entry() -> ! { unsafe { ExitProcess(probe()) } }
