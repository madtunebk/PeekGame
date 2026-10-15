mod launch;
mod manifest;
use crate::{pe, report::json::quote};
use std::{
    env,
    fmt::Write,
    fs,
    path::{Path, PathBuf},
};
pub fn launch_if_packaged() -> Result<bool, String> {
    let executable = env::current_exe().map_err(|e| e.to_string())?;
    if executable.file_name().is_some_and(|name| name == "play") {
        launch::run(executable.parent().ok_or("launcher has no directory")?)?;
        return Ok(true);
    }
    Ok(false)
}
pub fn build(exe: &Path, output: &Path) -> Result<(), String> {
    let image = pe::parse(&fs::read(exe).map_err(|e| e.to_string())?)?;
    let mut imports = String::new();
    let mut graphics = false;
    for (i, import) in image.imports.iter().enumerate() {
        if import.delayed {
            return Err("package runner does not support delay imports".into());
        }
        let name = import
            .name
            .as_deref()
            .ok_or("package runner does not support ordinal imports")?;
        let entry = game_imports::registry::find(&import.dll, name)
            .filter(|e| e.address_available())
            .ok_or_else(|| format!("unavailable import {}!{name}", import.dll))?;
        graphics |= entry.dll == "d3d11.dll";
        if i > 0 {
            imports.push(',');
        }
        write!(
            imports,
            "\n    {{\"dll\":{},\"name\":{},\"status\":{},\"scope\":{}}}",
            quote(&import.dll),
            quote(name),
            quote(entry.status),
            quote(entry.scope)
        )
        .unwrap();
    }
    let executable = env::current_exe().map_err(|e| e.to_string())?;
    let library = executable
        .parent()
        .ok_or("binary has no directory")?
        .join("libgame_imports.so");
    let mut copies = vec![
        (executable, PathBuf::from("play")),
        (library, PathBuf::from("game_imports.so")),
        (exe.to_owned(), PathBuf::from("game.exe")),
    ];
    let dxvk = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/dxvk-x11");
    if graphics {
        for name in ["libdxvk_dxgi.so.0", "libdxvk_d3d11.so.0"] {
            copies.push((
                dxvk.join("prefix/lib").join(name),
                Path::new("runtime").join(name),
            ));
        }
        copies.push((
            dxvk.join("source/LICENSE"),
            PathBuf::from("runtime/DXVK-LICENSE"),
        ));
    }
    for (source, _) in &copies {
        if !source.is_file() {
            return Err(format!(
                "package prerequisite missing: {}",
                source.display()
            ));
        }
    }
    // Refuse to overwrite existing packages, including user data/cache/logs.
    if let Some(parent) = output.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::create_dir(output)
        .map_err(|e| format!("cannot create package {}: {e}", output.display()))?;
    let result = (|| {
        for dir in ["runtime", "cache", "logs"] {
            fs::create_dir(output.join(dir)).map_err(|e| e.to_string())?;
        }
        for (source, target) in &copies {
            fs::copy(source, output.join(target)).map_err(|e| e.to_string())?;
        }
        let manifest = manifest::render(exe, &imports);
        fs::write(output.join("manifest.json"), manifest).map_err(|e| e.to_string())?;
        Ok::<(), String>(())
    })();
    if result.is_err() {
        eprintln!(
            "incomplete package retained for inspection: {}",
            output.display()
        );
    }
    result?;
    println!(
        "Package created: {}\nRun: {}/play [GAME ARGS]",
        output.display(),
        output.display()
    );
    Ok(())
}
