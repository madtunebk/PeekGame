use std::ffi::c_void;
unsafe extern "C" {
    fn _exit(code: i32) -> !;
    fn mmap(a: *mut c_void, n: usize, p: i32, f: i32, fd: i32, o: i64) -> *mut c_void;
    fn mprotect(a: *mut c_void, n: usize, p: i32) -> i32;
    fn munmap(a: *mut c_void, n: usize) -> i32;
}
pub struct Traps {
    memory: *mut u8,
    size: usize,
    names: Vec<Box<str>>,
}
impl Traps {
    pub fn new(names: Vec<String>) -> Result<Self, String> {
        let size = (names.len().max(1) * 40).div_ceil(4096) * 4096;
        let memory = unsafe { mmap(std::ptr::null_mut(), size, 3, 0x22, -1, 0) };
        if memory as isize == -1 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let result = Self {
            memory: memory.cast(),
            size,
            names: names.into_iter().map(String::into_boxed_str).collect(),
        };
        for (i, name) in result.names.iter().enumerate() {
            // Windows ABI, original return address remains on stack. Never returns.
            let mut code = vec![0x48, 0xb9];
            code.extend_from_slice(
                &(name.as_ref() as *const str as *const u8 as usize).to_le_bytes(),
            );
            code.extend_from_slice(&[0x48, 0xba]);
            code.extend_from_slice(&name.len().to_le_bytes());
            code.extend_from_slice(&[0x4c, 0x8b, 0x04, 0x24]);
            code.extend_from_slice(&[0x48, 0xb8]);
            code.extend_from_slice(&(missing as *const () as usize).to_le_bytes());
            code.extend_from_slice(&[0xff, 0xe0]);
            unsafe {
                std::ptr::copy_nonoverlapping(code.as_ptr(), result.memory.add(i * 40), code.len());
            }
        }
        if unsafe { mprotect(memory, size, 5) } != 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(result)
    }
    pub fn address(&self, index: usize) -> usize {
        unsafe { self.memory.add(index * 40) as usize }
    }
}
impl Drop for Traps {
    fn drop(&mut self) {
        unsafe {
            munmap(self.memory.cast(), self.size);
        }
    }
}
extern "win64" fn missing(name: *const u8, len: usize, caller: usize) -> ! {
    let name = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(name, len)) };
    eprintln!("STOP: missing Windows API {name}, called from {caller:#x}");
    unsafe { _exit(78) }
}
