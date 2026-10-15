//! EncodePointer/DecodePointer (and the System* forms): XOR with a per-process
//! random cookie, then a rotation by its low bits, as Windows does.
use std::sync::OnceLock;
unsafe extern "C" {
    fn getrandom(buffer: *mut u8, length: usize, flags: u32) -> isize;
}
fn cookie() -> u64 {
    static COOKIE: OnceLock<u64> = OnceLock::new();
    *COOKIE.get_or_init(|| loop {
        let mut c = [0u8; 8];
        if unsafe { getrandom(c.as_mut_ptr(), 8, 0) } == 8 && u64::from_ne_bytes(c) != 0 {
            break u64::from_ne_bytes(c);
        }
    })
}
pub extern "win64" fn encode(p: usize) -> usize {
    let c = cookie();
    ((p as u64 ^ c).rotate_right((c & 63) as u32)) as usize
}
pub extern "win64" fn decode(p: usize) -> usize {
    let c = cookie();
    ((p as u64).rotate_left((c & 63) as u32) ^ c) as usize
}
