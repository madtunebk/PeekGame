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
pub fn detach() {
    notify(3);
    crate::images::thread_notify(3);
}
