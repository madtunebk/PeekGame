use crate::report::json::quote;
use std::path::Path;
pub(super) fn render(exe: &Path, imports: &str) -> String {
    format!("{{\n  \"schema_version\":1,\n  \"executable\":\"game.exe\",\n  \"launcher\":\"play\",\n  \"import_library\":\"game_imports.so\",\n  \"runtime_selection\":\"full_prototype_catalog\",\n  \"original_name\":{},\n  \"host_requirements\":[\"Linux x86-64/glibc\",\"X11\",\"Vulkan driver\",\"libstdc++ and DXVK system dependencies\"],\n  \"limitations\":[\"CRT-free prototype runner\",\"preferred PE base required\",\"no TLS/SEH, external PE DLL loading, delay or ordinal binding\",\"assets are not automatically copied\",\"dynamic imports are observed only during execution\",\"DXVK Native modified with PeekGame X11 WSI\"],\n  \"imports\":[{imports}\n  ]\n}}\n",quote(&exe.file_name().unwrap().to_string_lossy()))
}
