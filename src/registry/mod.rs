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
mod network;
mod ordinals;
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
        .chain(network::EXPORTS.iter())
}
fn canonical(dll: &str) -> String {
    let lower = dll.to_ascii_lowercase();
    let lower = lower
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(&lower)
        .to_owned();
    // API sets (api-ms-win-*, ext-ms-win-*) are served by kernel32, as kernelbase
    // forwards them on Windows.
    // WinRT and COM API sets belong to combase.dll, which ole32 fronts here.
    if lower.starts_with("api-ms-win-core-winrt-")
        || lower.starts_with("api-ms-win-core-com-")
        || lower == "combase.dll"
    {
        return "ole32".into();
    }
    // The UCRT API sets (api-ms-win-crt-*) and ucrtbase are the C runtime PeekGame
    // serves as msvcrt.
    if lower.starts_with("api-ms-win-crt-") || lower == "ucrtbase.dll" {
        return "msvcrt".into();
    }
    // WSOCK32 is the Winsock 1.1 front of WS2_32.
    if lower == "wsock32.dll" || lower == "wsock32" {
        return "ws2_32".into();
    }
    if lower.starts_with("api-ms-win-") || lower.starts_with("ext-ms-win-") {
        return "kernel32".into();
    }
    lower.strip_suffix(".dll").unwrap_or(&lower).to_owned()
}
pub fn find(dll: &str, name: &str) -> Option<&'static Export> {
    let normalized = canonical(dll);
    entries().find(|e| e.dll.strip_suffix(".dll").unwrap_or(e.dll) == normalized && e.name == name)
}
/// An import by name, or by ordinal through the DLL's fixed ordinal table.
pub fn find_import(dll: &str, name: Option<&str>, ordinal: Option<u16>) -> Option<&'static Export> {
    let name = match name {
        Some(name) => name,
        None => ordinals::name(&canonical(dll), ordinal?)?,
    };
    find(dll, name)
}
/// Does PeekGame implement `dll` (at least partly)? Such a DLL is never loaded from disk.
pub fn provides(dll: &str) -> bool {
    let normalized = canonical(dll);
    entries().any(|e| e.dll.strip_suffix(".dll").unwrap_or(e.dll) == normalized)
}
