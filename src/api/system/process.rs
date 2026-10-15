// DLL_PROCESS_DETACH notifications require the future PE loader.
// For now this implements host process termination, including all host threads.
pub(crate) extern "win64" fn exit_process(code: u32) -> ! {
    crate::api::crt::startup::cexit();
    std::process::exit((code & 0xff) as i32)
}
