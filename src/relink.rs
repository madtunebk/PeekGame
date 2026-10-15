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

const READ: u32 = 0x4000_0000;
const WRITE: u32 = 0x8000_0000;
const EXECUTE: u32 = 0x2000_0000;
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

struct Section {
    rva: usize,
    /// Mapped size: the larger of the virtual and raw sizes, like the PE runner.
    extent: usize,
    raw: usize,
    file: usize,
    flags: u32,
}
struct Layout {
    base: usize,
    headers: usize,
    sections: Vec<Section>,
}

pub fn build(exe: &Path, output: &Path) -> Result<()> {
    let bytes = fs::read(exe).map_err(|e| e.to_string())?;
    let image = pe::parse(&bytes)?;
    let layout = layout(&bytes)?;
    if layout.base as u64 != image.image_base {
        return Err("inconsistent image base".into());
    }
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
    fs::write(
        &assembly,
        render(exe, &layout, &slots, &missing, image.entry_rva)?,
    )
    .map_err(|e| e.to_string())?;

    let stem = exe.file_stem().ok_or("EXE has no name")?;
    let target = output.join(stem);
    let mut link = Command::new(env::var_os("CC").unwrap_or_else(|| "cc".into()));
    link.arg("-no-pie")
        .arg("-o")
        .arg(&target)
        .arg(&assembly)
        .arg(&library)
        .arg("-Wl,-z,noexecstack");
    for (index, section) in sections(&layout) {
        link.arg(format!(
            "-Wl,--section-start={}={:#x}",
            section_name(index),
            layout.base + section.rva
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
fn sections(layout: &Layout) -> impl Iterator<Item = (usize, Section)> + '_ {
    let headers = Section {
        rva: 0,
        extent: layout.headers,
        raw: layout.headers,
        file: 0,
        flags: READ,
    };
    std::iter::once(headers)
        .chain(layout.sections.iter().map(|s| Section { ..*s }))
        .enumerate()
}

fn section_name(index: usize) -> String {
    format!(".peimage.{index:02}")
}

fn layout(bytes: &[u8]) -> Result<Layout> {
    let u16_at = |at: usize| -> Result<usize> {
        Ok(u16::from_le_bytes(
            bytes
                .get(at..at + 2)
                .ok_or("truncated PE header")?
                .try_into()
                .unwrap(),
        ) as usize)
    };
    let u32_at = |at: usize| -> Result<usize> {
        Ok(u32::from_le_bytes(
            bytes
                .get(at..at + 4)
                .ok_or("truncated PE header")?
                .try_into()
                .unwrap(),
        ) as usize)
    };
    let pe = u32_at(60)?;
    let opt = pe + 24;
    let count = u16_at(pe + 6)?;
    let base = u32_at(opt + 24)? | (u32_at(opt + 28)? << 32);
    let size = u32_at(opt + 56)?;
    let headers = u32_at(opt + 60)?;
    if !base.is_multiple_of(4096) || headers == 0 || headers > size {
        return Err("invalid image base or header size".into());
    }
    let table = opt + u16_at(pe + 20)?;
    let mut sections = Vec::new();
    for index in 0..count {
        let at = table + index * 40;
        let (rva, raw, file) = (u32_at(at + 12)?, u32_at(at + 16)?, u32_at(at + 20)?);
        let extent = u32_at(at + 8)?.max(raw);
        let flags = u32_at(at + 36)? as u32;
        if !rva.is_multiple_of(4096)
            || rva < headers
            || rva.checked_add(extent).is_none_or(|end| end > size)
        {
            return Err("invalid section extent/alignment".into());
        }
        if flags & (WRITE | EXECUTE) == WRITE | EXECUTE {
            return Err("writable executable PE sections are unsupported".into());
        }
        if file.checked_add(raw).is_none_or(|end| end > bytes.len()) {
            return Err("truncated section".into());
        }
        sections.push(Section {
            rva,
            extent,
            raw,
            file,
            flags,
        });
    }
    sections.sort_by_key(|s| s.rva);
    if sections
        .windows(2)
        .any(|w| w[0].rva + w[0].extent > w[1].rva)
    {
        return Err("overlapping sections".into());
    }
    Ok(Layout {
        base,
        headers,
        sections,
    })
}

/// Emits section bytes from `at` up to `to`: file data while it lasts, then zeros.
fn fill(s: &mut String, exe: &str, section: &Section, at: &mut usize, to: usize) {
    if to > *at && *at < section.raw {
        let end = to.min(section.raw);
        writeln!(
            s,
            ".incbin \"{exe}\", {:#x}, {:#x}",
            section.file + *at,
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
    layout: &Layout,
    slots: &BTreeMap<usize, String>,
    missing: &[String],
    entry_rva: u32,
) -> Result<String> {
    let exe = exe.to_string_lossy();
    if exe.contains(['"', '\\', '\n']) {
        return Err("EXE path cannot be embedded in assembly".into());
    }
    let mut s = String::from(
        "# Generated by peekgame --relink. PE image laid out at its preferred base.\n",
    );
    let mut placed = 0;
    for (index, section) in sections(layout) {
        let flags = match (section.flags & WRITE != 0, section.flags & EXECUTE != 0) {
            (true, _) => "aw",
            (_, true) => "ax",
            _ => "a",
        };
        let inside: Vec<_> = slots
            .range(section.rva..section.rva + section.extent)
            .map(|(rva, symbol)| (rva - section.rva, symbol))
            .collect();
        let kind = if section.raw == 0 && inside.is_empty() {
            "@nobits"
        } else {
            "@progbits"
        };
        writeln!(s, "\n.section {},\"{flags}\",{kind}", section_name(index)).unwrap();
        let mut at = 0;
        for (offset, symbol) in inside {
            if offset + 8 > section.extent {
                return Err(format!(
                    "IAT slot {:#x} crosses a section end",
                    section.rva + offset
                ));
            }
            fill(&mut s, &exe, &section, &mut at, offset);
            writeln!(s, ".quad {symbol}").unwrap();
            at += 8;
            placed += 1;
        }
        fill(&mut s, &exe, &section, &mut at, section.extent);
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
        "\n.globl main\n.type main,@function\nmain:\n    movabs ${:#x}, %rdx\n    mov ${entry_rva:#x}, %ecx\n    jmp peekgame_relinked_main\n.size main, .-main\n",
        layout.base
    )
    .unwrap();
    s.push_str(".section .note.GNU-stack,\"\",@progbits\n");
    Ok(s)
}
