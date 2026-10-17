use std::{ffi::CString, sync::OnceLock};
pub(crate) struct Process {
    pub image_base: usize,
    pub command_line: CString,
    pub standard_handles: [std::sync::atomic::AtomicUsize; 3],
}
pub(crate) static PROCESS: OnceLock<Process> = OnceLock::new();
