//! `--relink`: turns a PE image into a native Linux ELF executable.
//!
//! Every PE section becomes an ELF section linked at its original virtual address,
//! so the kernel maps the image as ordinary PT_LOAD segments. Each IAT slot becomes
//! a relocation against the runtime's `peekgame.<dll>.<name>` symbol and the static
//! linker binds it; the result links the runtime statically (`libgame_imports.a`).
//! The PE machine code is unchanged: it still uses the Windows x64 ABI and still
//! calls the Rust Win64 API implementations, just without a loader in between.
use crate::{model::Result, pe};
use std::{collections::BTreeMap, env, fmt::Write, fs, path::Path, process::Command};

/// Host libraries the Rust static runtime needs (`--print native-static-libs`).
const HOST_LIBS: &[&str] = &[
    "-lX11",
    "-lgcc_s",
    "-lutil",
    "-lrt",
    "-lpthread",
    "-lm",
    "-ldl",
    "-lc",
];

pub fn build(exe: &Path, output: &Path) -> Result<()> {
    let bytes = fs::read(exe).map_err(|e| e.to_string())?;
    let image = pe::parse(&bytes)?;
    image.check_layout(Some(bytes.len()))?;
    // IAT slot RVA -> symbol the linker writes there.
    let mut slots = BTreeMap::new();
    let mut missing = Vec::new();
    let mut graphics = false;
    for import in &image.imports {
        if import.delayed {
            return Err("relink does not support delay imports yet".into());
        }
        let name = import
            .name
            .as_deref()
            .ok_or("relink does not support ordinal imports yet")?;
        let symbol = match game_imports::registry::find(&import.dll, name)
            .filter(|e| e.address_available())
        {
            Some(entry) => {
                graphics |= entry.dll == "d3d11.dll";
                format!("\"peekgame.{}.{}\"", entry.dll, entry.name)
            }
            None => {
                missing.push(format!("{}!{name}", import.dll));
                format!(".Lmissing{}", missing.len() - 1)
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
    if let Some(parent) = output.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    // Refuse to overwrite an earlier result, like --package.
    fs::create_dir(output).map_err(|e| format!("cannot create {}: {e}", output.display()))?;
    let work = output.join("build");
    fs::create_dir(&work).map_err(|e| e.to_string())?;
    let assembly = work.join("image.S");
    fs::write(&assembly, render(exe, &image, &slots, &missing)?).map_err(|e| e.to_string())?;

    let stem = exe.file_stem().ok_or("EXE has no name")?;
    let target = output.join(stem);
    let mut link = Command::new(env::var_os("CC").unwrap_or_else(|| "cc".into()));
    link.arg("-no-pie")
        .arg("-o")
        .arg(&target)
        .arg(&assembly)
        .arg(&library)
        .arg("-Wl,-z,noexecstack");
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

    if graphics {
        let dxvk = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/dxvk-x11/prefix/lib");
        let runtime = output.join("runtime");
        fs::create_dir(&runtime).map_err(|e| e.to_string())?;
        for name in ["libdxvk_dxgi.so.0", "libdxvk_d3d11.so.0"] {
            let source = dxvk.join(name);
            if source.is_file() {
                fs::copy(&source, runtime.join(name)).map_err(|e| e.to_string())?;
            } else {
                eprintln!(
                    "warning: {} not built (tools/build_dxvk_x11.py); D3D11 creation will fail until runtime/{name} exists",
                    source.display()
                );
            }
        }
    }
    println!(
        "Relinked: {}\n{} import slot(s) bound at link time, {} unresolved (stop if called).",
        target.display(),
        slots.len() - missing.len(),
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
    write!(
        s,
        "\n.globl main\n.type main,@function\nmain:\n    movabs ${:#x}, %rdx\n    mov ${:#x}, %ecx\n    jmp peekgame_relinked_main\n.size main, .-main\n",
        image.image_base, image.entry_rva
    )
    .unwrap();
    s.push_str(".section .note.GNU-stack,\"\",@progbits\n");
    Ok(s)
}
