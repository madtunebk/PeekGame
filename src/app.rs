use crate::{analysis, cli, model::Result, report};
use std::fs;
pub fn run() -> Result<()> {
    let Some(options) = cli::parse()? else {
        return Ok(());
    };
    if options.package {
        return crate::package::build(&options.exe, &options.output);
    }
    if options.relink {
        return crate::relink::build(&options.exe, &options.output, options.strip);
    }
    if options.run {
        return crate::loader::run(&options.exe, &options.args, &options.output);
    }
    let (modules, deps) = analysis::analyze(options.exe, options.dll_dirs)?;
    let json = report::json::render(&modules, &deps);
    let text = report::text::render(&modules, &deps);
    fs::create_dir_all(&options.output).map_err(|e| e.to_string())?;
    fs::write(options.output.join("imports.json"), json).map_err(|e| e.to_string())?;
    fs::write(options.output.join("imports.txt"), text).map_err(|e| e.to_string())?;
    println!(
        "Analyzed {} PE module(s), {} import slots.\n{}\n{}",
        modules.len(),
        modules.iter().map(|m| m.image.imports.len()).sum::<usize>(),
        options.output.join("imports.json").display(),
        options.output.join("imports.txt").display()
    );
    Ok(())
}
