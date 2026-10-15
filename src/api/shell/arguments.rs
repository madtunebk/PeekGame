use crate::{
    api::{memory::local, system::error},
    host::arguments,
};
pub unsafe extern "win64" fn command_line_to_argv(
    input: *const u16,
    count: *mut i32,
) -> *mut *mut u16 {
    if input.is_null() || count.is_null() {
        error::set(87);
        return std::ptr::null_mut();
    }
    let mut n = 0;
    while unsafe { *input.add(n) } != 0 {
        n += 1;
    }
    let input = unsafe { std::slice::from_raw_parts(input, n) };
    let mut args = arguments::split(input);
    if args.is_empty() {
        if let Some(process) = crate::host::process::PROCESS.get() {
            let line = process
                .command_line
                .to_string_lossy()
                .encode_utf16()
                .collect::<Vec<_>>();
            args = arguments::split(&line).into_iter().take(1).collect();
        }
    } else if input.first().is_some_and(|c| *c == 32 || *c == 9) {
        args.insert(0, Vec::new());
    }
    let header = (args.len() + 1) * 8;
    let bytes = header + args.iter().map(|s| (s.len() + 1) * 2).sum::<usize>();
    let base = local::allocate(0x40, bytes) as *mut u8;
    if base.is_null() {
        return std::ptr::null_mut();
    }
    let pointers = base.cast::<*mut u16>();
    let mut offset = header;
    for (i, arg) in args.iter().enumerate() {
        let text = unsafe { base.add(offset).cast::<u16>() };
        unsafe {
            *pointers.add(i) = text;
            std::ptr::copy_nonoverlapping(arg.as_ptr(), text, arg.len());
        }
        offset += (arg.len() + 1) * 2;
    }
    unsafe {
        *count = args.len() as i32;
    }
    pointers
}
