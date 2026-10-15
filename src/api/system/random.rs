//! bcrypt!BCryptGenRandom (and ProcessPrng) from the host CSPRNG.
unsafe extern "C" {
    fn getrandom(buffer: *mut u8, length: usize, flags: u32) -> isize;
}
const STATUS_INVALID_PARAMETER: i32 = 0xC000000Du32 as i32;
/// BCRYPT_USE_SYSTEM_PREFERRED_RNG (2) with no algorithm handle, or any handle:
/// all requests are served by the host generator.
pub unsafe extern "win64" fn bcrypt_gen_random(
    _algorithm: usize,
    buffer: *mut u8,
    length: u32,
    _flags: u32,
) -> i32 {
    if buffer.is_null() && length != 0 {
        return STATUS_INVALID_PARAMETER;
    }
    let mut done = 0usize;
    while done < length as usize {
        let n = unsafe { getrandom(buffer.add(done), length as usize - done, 0) };
        if n < 0 {
            if std::io::Error::last_os_error().raw_os_error() == Some(4) {
                continue;
            }
            return STATUS_INVALID_PARAMETER;
        }
        done += n as usize;
    }
    0
}
pub unsafe extern "win64" fn process_prng(buffer: *mut u8, length: usize) -> i32 {
    (unsafe { bcrypt_gen_random(0, buffer, length as u32, 2) } == 0) as i32
}
