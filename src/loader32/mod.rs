//! 32-bit PE process loader.
//! Dedicated loader for 32-bit PE binaries (Agent.exe, Blizzard services).
//! Separated from the 64-bit loader (`src/loader/`).

use game_imports::{images32, pe32};
use std::{fs, path::Path};

pub fn run(path: &Path, _args: &[String], _output: &Path) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let image = pe32::parse(&bytes)?;
    eprintln!(
        "PeekGame32: loading 32-bit PE {} (image_base={:#x}, size={:#x}, entry_rva={:#x})",
        path.display(),
        image.image_base,
        image.image_size,
        image.entry_rva
    );

    let mapped = images32::Mapping32::new(&image, &bytes)?;
    images32::register_exe(mapped.base(), path, image);

    eprintln!(
        "PeekGame32: mapped {} at {:#x} (entry at {:#x})",
        path.display(),
        mapped.base(),
        mapped.base() + mapped.base().wrapping_sub(mapped.base())
    );

    Ok(())
}
