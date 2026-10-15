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
/// Emits the global link-time entry `peekgame.<dll>.<name>`: a direct jump to the
/// implementation, so a relinked image binds its IAT through the static linker.
macro_rules! link_symbol {
    ($dll:literal, $name:literal, $target:path) => {
        std::arch::global_asm!(
            concat!(
                ".pushsection .text.peekgame.", $dll, ".", $name, ",\"ax\",@progbits\n",
                ".globl \"peekgame.", $dll, ".", $name, "\"\n",
                ".type \"peekgame.", $dll, ".", $name, "\",@function\n",
                "\"peekgame.", $dll, ".", $name, "\":\n",
                "jmp {0}\n",
                ".popsection"
            ),
            sym $target
        );
    };
}
/// Declares a subsystem's `EXPORTS` table and the link-time alias of every entry.
macro_rules! exports {
    ($($kind:ident!($($args:tt)*)),* $(,)?) => {
        pub(super) static EXPORTS: &[Export] = &[$($kind!($($args)*)),*];
        $($kind!(@link $($args)*);)*
    };
}
macro_rules! export {
    (@link $dll:literal,$name:literal,$category:literal,$status:literal,$scope:literal,$function:path $(,)?) => {
        link_symbol!($dll, $name, $function);
    };
    ($dll:literal,$name:literal,$category:literal,$status:literal,$scope:literal,$function:path $(,)?) => {
        Export {
            dll: $dll,
            name: $name,
            category: $category,
            status: $status,
            scope: $scope,
            address: Some(|| $function as *const std::ffi::c_void),
        }
    };
}
/// A data export: the import slot receives the address of `$variable`. Its link
/// symbol is the variable's own `export_name`, since a jump cannot stand in for data.
macro_rules! data {
    (@link $($args:tt)*) => {};
    ($dll:literal,$name:literal,$category:literal,$status:literal,$scope:literal,$variable:path $(,)?) => {
        Export {
            dll: $dll,
            name: $name,
            category: $category,
            status: $status,
            scope: $scope,
            address: Some(|| &$variable as *const _ as *const std::ffi::c_void),
        }
    };
}
macro_rules! implemented {
    (@link $name:literal,$category:literal,$status:literal,$scope:literal,$function:path $(,)?) => {
        link_symbol!("kernel32.dll", $name, $function);
    };
    ($name:literal,$category:literal,$status:literal,$scope:literal,$function:path $(,)?) => {
        Export {
            dll: "kernel32.dll",
            name: $name,
            category: $category,
            status: $status,
            scope: $scope,
            address: Some(|| $function as *const std::ffi::c_void),
        }
    };
}
macro_rules! window {
    (@link $name:literal,$scope:literal,$function:path $(,)?) => {
        link_symbol!("user32.dll", $name, $function);
    };
    ($name:literal,$scope:literal,$function:path $(,)?) => {
        Export {
            dll: "user32.dll",
            name: $name,
            category: "window",
            status: "partial",
            scope: $scope,
            address: Some(|| $function as *const std::ffi::c_void),
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
