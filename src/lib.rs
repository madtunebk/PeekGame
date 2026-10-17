//! Windows import implementations and the Linux loader ABI.
#![deny(unsafe_op_in_unsafe_fn)]
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
compile_error!("PeekGame currently requires Linux x86-64");
mod abi;
mod api;
mod diagnostics;
mod host;
pub mod images;
pub mod mapped;
pub mod pe;
pub mod registry;
mod relinked;
mod runtime;
mod seh;
pub use relinked::quote as quote_windows_argument;
pub use runtime::{peekgame_abi_version, peekgame_configure_process, peekgame_resolve};

pub use api::system::dynamic::start as start_import_trace;

pub use host::teb::initialize as initialize_windows_thread;
pub use host::thread_template::configure as configure_windows_threads;
