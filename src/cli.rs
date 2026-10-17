use crate::model::Result;
use std::{
    env, fs,
    path::{Path, PathBuf},
};
pub struct Options {
    pub exe: PathBuf,
    pub run: bool,
    pub package: bool,
    pub relink: bool,
    pub args: Vec<String>,
    pub output: PathBuf,
    pub dll_dirs: Vec<PathBuf>,
}
pub fn parse() -> Result<Option<Options>> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        println!("Usage: peekgame --analyze GAME.exe [--output DIRECTORY] [--dll-dir DIRECTORY]...\nAlso accepts: peekgame GAME.exe --analyze\nPackage: peekgame GAME.exe --package --output NEW_DIRECTORY\nNative ELF: peekgame GAME.exe --relink [--output NEW_DIRECTORY]\nRun prototype: peekgame GAME.exe [--output DIRECTORY] --run [GAME ARGS]\nDefault: reports/GAME/imports.json and imports.txt. Analysis does not execute the PE. --run executes supported PE images.");
        return Ok(None);
    }
    let mut run = false;
    let mut package = false;
    let mut relink = false;
    let mut program_args = Vec::new();
    let (mut exe, mut output, mut dirs, mut flag, mut i) = (None, None, Vec::new(), false, 0);
    while i < args.len() {
        if args[i] == "--run" {
            run = true;
            program_args = args[i + 1..]
                .iter()
                .map(|s| s.to_string_lossy().into_owned())
                .collect();
            break;
        } else if args[i] == "--package" {
            package = true;
        } else if args[i] == "--relink" {
            relink = true;
        } else if args[i] == "--analyze" {
            flag = true;
        } else if args[i] == "--output" || args[i] == "--dll-dir" {
            let option = args[i].clone();
            i += 1;
            let value = args.get(i).ok_or("option requires a directory")?;
            if option == "--output" {
                output = Some(PathBuf::from(value));
            } else {
                dirs.push(PathBuf::from(value));
            }
        } else if args[i].to_string_lossy().starts_with('-') {
            return Err(format!("unknown option: {}", args[i].to_string_lossy()));
        } else if exe.replace(PathBuf::from(&args[i])).is_some() {
            return Err("provide one EXE".into());
        }
        i += 1;
    }
    if [flag, run, package, relink].iter().filter(|v| **v).count() != 1 {
        return Err("choose exactly one of --analyze, --run, --package, --relink".into());
    }
    let exe = fs::canonicalize(exe.ok_or("provide an EXE")?).map_err(|e| e.to_string())?;
    let dirs = dirs
        .into_iter()
        .map(|d| fs::canonicalize(&d).map_err(|e| format!("{}: {e}", d.display())))
        .collect::<Result<Vec<_>>>()?;
    let output = output.unwrap_or_else(|| {
        if relink {
            return exe.parent().unwrap_or(Path::new(".")).to_path_buf();
        }
        Path::new(if package {
            "packages"
        } else if run {
            "logs"
        } else {
            "reports"
        })
        .join(exe.file_stem().unwrap())
    });
    Ok(Some(Options {
        exe,
        run,
        package,
        relink,
        args: program_args,
        output,
        dll_dirs: dirs,
    }))
}
