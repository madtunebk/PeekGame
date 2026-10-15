mod dlls;
use crate::{
    model::{Dependency, Module, Result},
    pe,
};
use dlls::find_dll;
use std::{
    collections::{BTreeSet, VecDeque},
    fs,
    path::PathBuf,
};
pub fn analyze(exe: PathBuf, dirs: Vec<PathBuf>) -> Result<(Vec<Module>, Vec<Dependency>)> {
    let mut queue = VecDeque::from([exe.clone()]);
    let mut visited = BTreeSet::new();
    let mut modules = Vec::new();
    let mut deps = Vec::new();
    while let Some(path) = queue.pop_front() {
        if !visited.insert(path.clone()) {
            continue;
        }
        if visited.len() > 1024 {
            return Err("dependency graph exceeds 1024 modules".into());
        }
        let bytes = fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let image = pe::parse(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut search = vec![exe.parent().unwrap().to_path_buf()];
        for dir in std::iter::once(path.parent().unwrap().to_path_buf()).chain(dirs.iter().cloned())
        {
            if !search.contains(&dir) {
                search.push(dir);
            }
        }
        let dlls: BTreeSet<_> = image.imports.iter().map(|i| i.dll.clone()).collect();
        for dll in dlls {
            let resolved = find_dll(&dll, &search)?;
            if let Some(p) = &resolved {
                queue.push_back(p.clone());
            }
            deps.push(Dependency {
                importer: path.clone(),
                dll,
                resolved,
            });
        }
        modules.push(Module {
            path,
            bytes: bytes.len(),
            image,
        });
    }
    Ok((modules, deps))
}
