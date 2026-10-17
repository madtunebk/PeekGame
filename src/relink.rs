//! `--relink`: turns a PE image into a native Linux ELF executable.
//!
//! Every PE section becomes an ELF section linked at its original virtual address,
//! so the kernel maps the image as ordinary PT_LOAD segments. Each IAT slot becomes
//! a relocation against the runtime's `peekgame.<dll>.<name>` symbol and the static
//! linker binds it; the result links the runtime statically (`libgame_imports.a`).
//! The PE machine code is unchanged: it still uses the Windows x64 ABI and still
//! calls the Rust Win64 API implementations, just without a loader in between.
mod runtime;
use crate::{model::Result, pe};
use std::{collections::BTreeMap, env, fmt::Write, fs, path::Path, process::Command};

/// Host libraries the Rust static runtime needs (`--print native-static-libs`).
const HOST_LIBS: &[&str] = &[
    "-licui18n",
    "-licuuc",
    "-licudata",
    "-lX11",
    "-lGL",
    "-lfontconfig",
    "-lpng16",
    "-lXrandr",
    "-lXinerama",
    "-lcrypto",
    "-lpulse",
    "-lgio-2.0",
    "-lglib-2.0",
    "-lgcc_s",
    "-lutil",
    "-lrt",
    "-lpthread",
    "-lm",
    "-ldl",
    "-lc",
];

pub fn build(exe: &Path, output: &Path, strip: bool) -> Result<()> {
    let bytes = fs::read(exe).map_err(|e| e.to_string())?;
    let image = pe::parse(&bytes)?;
    image.check_layout(Some(bytes.len()))?;
    // IAT slot RVA -> symbol the linker writes there.
    let mut slots = BTreeMap::new();
    let mut missing = Vec::new();
    let mut runtime = Vec::new();
    let mut graphics = false;
    // Delay-load slots point into the image's own helper, which binds them through
    // LoadLibrary/GetProcAddress when first called: nothing to do here.
    for import in image.imports.iter().filter(|i| !i.delayed) {
        let label = || {
            format!(
                "{}!{}",
                import.dll,
                import
                    .name
                    .clone()
                    .unwrap_or_else(|| format!("#{}", import.ordinal.unwrap_or(0)))
            )
        };
        let symbol = if !game_imports::images::builtin(&import.dll) {
            // A program DLL (UnityPlayer.dll, ...): bound at startup by the PE loader.
            runtime.push(import.clone());
            "0".to_owned()
        } else {
            match game_imports::registry::find_import(
                &import.dll,
                import.name.as_deref(),
                import.ordinal,
            )
            .filter(|e| e.address_available())
            {
                Some(entry) => {
                    graphics |= entry.dll == "d3d11.dll";
                    format!("\"peekgame.{}.{}\"", entry.dll, entry.name)
                }
                None => {
                    missing.push(label());
                    format!(".Lmissing{}", missing.len() - 1)
                }
            }
        };
        if slots.insert(import.iat_rva as usize, symbol).is_some() {
            return Err(format!("two imports share IAT slot {:#x}", import.iat_rva));
        }
    }
    let host = env::current_exe().map_err(|e| e.to_string())?;
    let library = host
        .parent()
        .ok_or("binary has no directory")?
        .join("libgame_imports.a");
    if !library.is_file() {
        return Err(format!(
            "relink prerequisite missing: {}",
            library.display()
        ));
    }
    // The executable may go into an existing directory (the game's own, so it finds
    // the DLLs and data beside it), but never replaces an existing file.
    fs::create_dir_all(output).map_err(|e| format!("cannot create {}: {e}", output.display()))?;
    let stem = exe.file_stem().ok_or("EXE has no name")?;
    let target = output.join(stem);
    if target == exe {
        return Err(format!("refusing to overwrite input EXE {}", target.display()));
    }
    let work = output.join(format!("{}.relink", stem.to_string_lossy()));
    fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    let assembly = work.join("image.S");
    let exe_name = exe.file_name().ok_or("EXE has no name")?.to_string_lossy();
    fs::write(
        &assembly,
        render(exe, &image, &slots, &missing, &runtime, &exe_name)?,
    )
    .map_err(|e| e.to_string())?;

    let mut link = Command::new(env::var_os("CC").unwrap_or_else(|| "cc".into()));
    link.arg("-no-pie")
        .arg("-o")
        .arg(&target)
        .arg(&assembly)
        .arg(&library)
        .arg("-Wl,-z,noexecstack");
    if strip {
        // Strip the ELF's symbol/debug tables at link time. The import bindings
        // (peekgame.<dll>.<name>) and SEH data (the PE's own .pdata) are already
        // baked into the image, so nothing load-bearing is removed.
        link.arg("-s");
    }
    for (index, section) in sections(&image) {
        link.arg(format!(
            "-Wl,--section-start={}={:#x}",
            section_name(index),
            image.image_base + section.rva as u64
        ));
    }
    link.args(HOST_LIBS);
    let status = link
        .status()
        .map_err(|e| format!("cannot run linker: {e}"))?;
    if !status.success() {
        return Err(format!(
            "linker failed ({status}); inputs kept in {}",
            work.display()
        ));
    }
    let _ = fs::remove_dir_all(&work);

    runtime::stage_windows_dlls(&host, output)?;
    if graphics {
        let dxvk = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/dxvk-x11/prefix/lib");
        let directory = output.join("runtime");
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        for name in ["libdxvk_dxgi.so.0", "libdxvk_d3d11.so.0"] {
            let source = dxvk.join(name);
            if source.is_file() {
                fs::copy(&source, directory.join(name)).map_err(|e| e.to_string())?;
            } else {
                eprintln!(
                    "warning: {} not built (tools/build_dxvk_x11.py); D3D11 creation will fail until runtime/{name} exists",
                    source.display()
                );
            }
        }
    }
    println!(
        "Relinked: {}\n{} import slot(s) bound at link time, {} from program DLLs bound at startup, {} unresolved (stop if called).",
        target.display(),
        slots.len() - missing.len() - runtime.len(),
        runtime.len(),
        missing.len()
    );
    for name in &missing {
        println!("  unresolved: {name}");
    }
    Ok(())
}

/// The headers (index 0) followed by every PE section, each with its ELF index.
fn sections(image: &pe::Image) -> impl Iterator<Item = (usize, pe::Section)> + '_ {
    std::iter::once(image.headers_section())
        .chain(image.sections.iter().cloned())
        .enumerate()
}

fn section_name(index: usize) -> String {
    format!(".peimage.{index:02}")
}

/// Emits section bytes from `at` up to `to`: file data while it lasts, then zeros.
fn fill(s: &mut String, exe: &str, section: &pe::Section, at: &mut usize, to: usize) {
    let raw = section.raw_size as usize;
    if to > *at && *at < raw {
        let end = to.min(raw);
        writeln!(
            s,
            ".incbin \"{exe}\", {:#x}, {:#x}",
            section.raw_offset as usize + *at,
            end - *at
        )
        .unwrap();
        *at = end;
    }
    if to > *at {
        writeln!(s, ".zero {:#x}", to - *at).unwrap();
        *at = to;
    }
}

fn render(
    exe: &Path,
    image: &pe::Image,
    slots: &BTreeMap<usize, String>,
    missing: &[String],
    runtime: &[pe::Import],
    exe_name: &str,
) -> Result<String> {
    let exe = exe.to_string_lossy();
    if exe.contains(['"', '\\', '\n']) {
        return Err("EXE path cannot be embedded in assembly".into());
    }
    let mut s = String::from(
        "# Generated by peekgame --relink. PE image laid out at its preferred base.\n",
    );
    let mut placed = 0;
    for (index, section) in sections(image) {
        let (rva, extent) = (section.rva as usize, section.extent() as usize);
        let flags = match (section.writable(), section.executable()) {
            (true, _) => "aw",
            (_, true) => "ax",
            _ => "a",
        };
        let inside: Vec<_> = slots
            .range(rva..rva + extent)
            .map(|(slot, symbol)| (slot - rva, symbol))
            .collect();
        let kind = if section.raw_size == 0 && inside.is_empty() {
            "@nobits"
        } else {
            "@progbits"
        };
        writeln!(s, "\n.section {},\"{flags}\",{kind}", section_name(index)).unwrap();
        let mut at = 0;
        for (offset, symbol) in inside {
            if offset + 8 > extent {
                return Err(format!(
                    "IAT slot {:#x} crosses a section end",
                    rva + offset
                ));
            }
            fill(&mut s, &exe, &section, &mut at, offset);
            writeln!(s, ".quad {symbol}").unwrap();
            at += 8;
            placed += 1;
        }
        fill(&mut s, &exe, &section, &mut at, extent);
    }
    if placed != slots.len() {
        return Err("IAT slot outside every section".into());
    }
    // Unresolved imports stop with their name instead of jumping to garbage.
    s.push_str("\n.section .rodata.peekgame_missing,\"a\",@progbits\n");
    for (i, name) in missing.iter().enumerate() {
        if name.contains(['"', '\\', '\n']) {
            return Err(format!(
                "import name cannot be embedded in assembly: {name:?}"
            ));
        }
        writeln!(s, ".Lmissing_name{i}: .asciz \"{name}\"").unwrap();
    }
    s.push_str(".text\n");
    for i in 0..missing.len() {
        writeln!(
            s,
            ".Lmissing{i}:\n    lea .Lmissing_name{i}(%rip), %rcx\n    mov (%rsp), %rdx\n    jmp peekgame_missing_import"
        )
        .unwrap();
    }
    // Imports from program DLLs, bound at startup by the PE loader (layout:
    // relinked::Info and relinked::Entry).
    let quoted = |text: &str| -> Result<String> {
        if text.contains(['"', '\\', '\n']) {
            return Err(format!("name cannot be embedded in assembly: {text:?}"));
        }
        Ok(format!("\"{text}\""))
    };
    s.push_str("\n.section .rodata.peekgame_runtime,\"a\",@progbits\n");
    writeln!(s, ".Lexe_name: .asciz {}", quoted(exe_name)?).unwrap();
    for (i, import) in runtime.iter().enumerate() {
        writeln!(s, ".Lrt_dll{i}: .asciz {}", quoted(&import.dll)?).unwrap();
        if let Some(name) = &import.name {
            writeln!(s, ".Lrt_name{i}: .asciz {}", quoted(name)?).unwrap();
        }
    }
    writeln!(
        s,
        ".balign 8\n.Lruntime_info:\n    .quad .Lexe_name\n    .quad {}",
        runtime.len()
    )
    .unwrap();
    for (i, import) in runtime.iter().enumerate() {
        let name = if import.name.is_some() {
            format!(".Lrt_name{i}")
        } else {
            "0".into()
        };
        writeln!(
            s,
            "    .long {:#x}, {:#x}\n    .quad .Lrt_dll{i}, {name}",
            import.iat_rva,
            import.ordinal.map_or(u32::MAX, u32::from)
        )
        .unwrap();
    }
    s.push_str(".text\n");
    write!(
        s,
        "\n.globl main\n.type main,@function\nmain:\n    movabs ${:#x}, %rdx\n    mov ${:#x}, %ecx\n    lea .Lruntime_info(%rip), %r8\n    jmp peekgame_relinked_main\n.size main, .-main\n",
        image.image_base, image.entry_rva
    )
    .unwrap();
    s.push_str(".section .note.GNU-stack,\"\",@progbits\n");
    Ok(s)
}
