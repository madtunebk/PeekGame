use crate::pe;
use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, String>;
pub struct Module {
    pub path: PathBuf,
    pub bytes: usize,
    pub image: pe::Image,
}
pub struct Dependency {
    pub importer: PathBuf,
    pub dll: String,
    pub resolved: Option<PathBuf>,
}
