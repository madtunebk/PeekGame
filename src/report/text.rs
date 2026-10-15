use crate::{
    model::{Dependency, Module},
    pe,
};
use std::fmt::Write as _;
pub fn render(modules: &[Module], deps: &[Dependency]) -> String {
    let mut out=String::from("PeekGame static import analysis\nImplementation availability: Rust registry (partial means limited supported scope)\nRuntime coverage: incomplete (LoadLibrary/GetProcAddress require tracing)\nAPI arguments and COM vtable calls are not inferred.\n\n");
    for m in modules {
        writeln!(out,"{}\n  x86-64 PE32+, {} bytes, image base {:#x}, entry RVA {:#x}\n  {} import slots, {} unique DLL/function pairs",
            m.path.display(),m.bytes,m.image.image_base,m.image.entry_rva,m.image.imports.len(),pe::unique_count(&m.image)).unwrap();
        for i in &m.image.imports {
            let label = i
                .name
                .clone()
                .unwrap_or_else(|| format!("#{}", i.ordinal.unwrap()));
            let export = game_imports::registry::find(&i.dll, i.name.as_deref().unwrap_or(""));
            writeln!(
                out,
                "    {}!{} [{}; IAT RVA {:#x}; backend: {}]",
                i.dll,
                label,
                if i.delayed { "delay" } else { "normal" },
                i.iat_rva,
                export.map(|e| e.status).unwrap_or("unknown")
            )
            .unwrap();
            if let Some(export) = export {
                writeln!(out, "      {}: {}", export.category, export.scope).unwrap();
            }
        }
        out.push('\n');
    }
    out.push_str("DLL dependencies (EXE directory, module directory, then --dll-dir):\n");
    for d in deps {
        writeln!(
            out,
            "  {} -> {}: {}",
            d.importer.file_name().unwrap().to_string_lossy(),
            d.dll,
            d.resolved
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or("external (backend availability not checked)".into())
        )
        .unwrap();
    }
    out
}
