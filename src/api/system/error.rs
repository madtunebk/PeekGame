use std::cell::Cell;
thread_local! { static LAST_ERROR: Cell<u32> = const { Cell::new(0) }; }
pub(crate) fn set(code: u32) {
    LAST_ERROR.set(code);
    crate::host::teb::set_error(code);
}
pub(crate) extern "win64" fn get_last_error() -> u32 {
    crate::host::teb::error().unwrap_or_else(|| LAST_ERROR.get())
}

// ---- PEEKGAME_TRACE_LAST_ERROR ----------------------------------------------------------------
// GetLastError and WSAGetLastError enter through this thunk so a nonzero error can be logged
// with the call site that read it (the instruction after the failed API call). Diagnostic only:
// the returned value is the same as `get_last_error`.
std::arch::global_asm!(
    ".text",
    ".p2align 4",
    ".globl peekgame_get_last_error_traced",
    "peekgame_get_last_error_traced:",
    "mov (%rsp), %rcx",
    "jmp {0}",
    sym get_last_error_from,
    options(att_syntax)
);
unsafe extern "win64" {
    /// GetLastError / WSAGetLastError with the caller passed to `get_last_error_from`.
    #[link_name = "peekgame_get_last_error_traced"]
    pub(crate) fn get_last_error_traced() -> u32;
}
/// `PEEKGAME_TRACE_LAST_ERROR=<module>` (e.g. PartyWin.dll; `all` for every module).
fn trace_filter() -> Option<&'static str> {
    static FILTER: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    FILTER
        .get_or_init(|| std::env::var("PEEKGAME_TRACE_LAST_ERROR").ok().filter(|v| !v.is_empty()))
        .as_deref()
}
extern "win64" fn get_last_error_from(caller: usize) -> u32 {
    let code = get_last_error();
    if code != 0 {
        if let Some(filter) = trace_filter() {
            trace_read(filter, code, caller);
        }
    }
    code
}
/// The first few reads of each (call site, code) pair, so polling loops do not flood the log.
fn trace_read(filter: &str, code: u32, caller: usize) {
    use std::collections::HashMap;
    use std::sync::Mutex;
    static SEEN: Mutex<Option<HashMap<(usize, u32), u32>>> = Mutex::new(None);
    let Some(module) = crate::images::by_address(caller) else {
        return;
    };
    if !filter.eq_ignore_ascii_case("all") && !module.name.eq_ignore_ascii_case(filter) {
        return;
    }
    let count = {
        let mut seen = SEEN.lock().unwrap_or_else(|e| e.into_inner());
        let n = seen.get_or_insert_with(HashMap::new).entry((caller, code)).or_insert(0);
        *n += 1;
        *n
    };
    if count <= 3 {
        let rva = caller - module.base;
        let thread = crate::api::system::startup::thread_id();
        eprintln!("[last-error] {}+{rva:#x} read error {code} ({code:#x}) thread {thread}", module.name);
    }
}
