use crate::api::window::{classes, create, cursor, lifecycle, messages};
// Explicit export catalog. A known name alone never supplies an implementation.
use crate::api::{
    filesystem::write,
    system::{command_line, debug, error, modules, process, standard_handles},
};
use std::ffi::c_void;
#[derive(Clone, Copy)]
pub struct Export {
    pub dll: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub status: &'static str,
    pub scope: &'static str,
    pub(crate) address: Option<fn() -> *const c_void>,
}
impl Export {
    pub fn address_available(&self) -> bool {
        self.address.is_some()
    }
}
macro_rules! implemented {
    ($name:literal,$category:literal,$status:literal,$scope:literal,$function:path) => {
        Export {
            dll: "kernel32.dll",
            name: $name,
            category: $category,
            status: $status,
            scope: $scope,
            address: Some(|| $function as *const c_void),
        }
    };
}
macro_rules! window {
    ($name:literal,$scope:literal,$function:path) => {
        Export {
            dll: "user32.dll",
            name: $name,
            category: "window",
            status: "partial",
            scope: $scope,
            address: Some(|| $function as *const c_void),
        }
    };
}
mod com;
mod crt;
mod filesystem;
mod graphics;
mod shell;
mod system;
mod threading;
mod window;
pub fn entries() -> impl Iterator<Item = &'static Export> {
    system::EXPORTS
        .iter()
        .chain(com::EXPORTS.iter())
        .chain(filesystem::EXPORTS.iter())
        .chain(shell::EXPORTS.iter())
        .chain(crt::EXPORTS.iter())
        .chain(threading::EXPORTS.iter())
        .chain(window::EXPORTS.iter())
        .chain(graphics::EXPORTS.iter())
}
pub fn find(dll: &str, name: &str) -> Option<&'static Export> {
    let normalized = dll.to_ascii_lowercase();
    let normalized = normalized.strip_suffix(".dll").unwrap_or(&normalized);
    entries().find(|e| e.dll.strip_suffix(".dll").unwrap_or(e.dll) == normalized && e.name == name)
}
