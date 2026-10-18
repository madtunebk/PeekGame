use std::sync::OnceLock;
struct Template {
    base: usize,
    data: Vec<u8>,
    callbacks: Vec<usize>,
}
static TEMPLATE: OnceLock<Template> = OnceLock::new();
pub fn configure(base: usize, data: Vec<u8>, callbacks: Vec<usize>) -> Result<(), String> {
    TEMPLATE
        .set(Template {
            base,
            data,
            callbacks,
        })
        .map_err(|_| "thread TLS template already configured".into())
}
fn notify(reason: u32) {
    if let Some(t) = TEMPLATE.get() {
        for &address in &t.callbacks {
            let callback: extern "win64" fn(usize, u32, usize) =
                unsafe { std::mem::transmute(address) };
            callback(t.base, reason, 0);
        }
    }
}
pub fn attach() -> Result<(), String> {
    if let Some(t) = TEMPLATE.get() {
        super::teb::initialize(t.base, t.data.clone())?;
        crate::images::thread_notify(2);
        notify(2);
    }
    Ok(())
}
/// A host-created worker can enter PE code through a callback without going through
/// CreateThread. Give it its own TEB and static TLS before resuming PE execution.
/// DLL_THREAD_ATTACH is deliberately deferred: this path is reached from a fault
/// handler, where running arbitrary PE callbacks would be unsafe.
pub fn attach_foreign_worker() -> Result<(), String> {
    let t = TEMPLATE.get().ok_or("thread TLS template is not configured")?;
    super::teb::initialize(t.base, t.data.clone())
}
pub fn detach() {
    crate::api::threading::fls::thread_exit();
    crate::api::crt::startup::run_thread_local_callbacks(3);
    notify(3);
    crate::images::thread_notify(3);
}
