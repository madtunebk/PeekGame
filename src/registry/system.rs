//! System export catalog, composed from bounded subsystem tables.
//! Each leaf owns declarations; API implementations remain in src/api/.
use super::Export;
mod compatibility;
mod console;
mod devices;
mod diagnostics;
mod exceptions;
mod io;
mod loader;
mod memory;
mod multimedia;
mod nls;
mod process;
mod registry_keys;
mod security;
mod system_info;
mod time;
mod version;
mod pdh;

pub(super) fn entries() -> impl Iterator<Item = &'static Export> {
    [
        compatibility::EXPORTS,
        console::EXPORTS,
        devices::EXPORTS,
        diagnostics::EXPORTS,
        exceptions::EXPORTS,
        io::EXPORTS,
        loader::EXPORTS,
        memory::EXPORTS,
        multimedia::EXPORTS,
        nls::EXPORTS,
        process::EXPORTS,
        registry_keys::EXPORTS,
        security::EXPORTS,
        system_info::EXPORTS,
        time::EXPORTS,
        version::EXPORTS,
        pdh::EXPORTS,
    ]
    .into_iter()
    .flatten()
}
