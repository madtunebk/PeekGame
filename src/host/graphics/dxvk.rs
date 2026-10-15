use crate::api::graphics::directx::types::*;
use std::{
    ffi::{c_char, c_void, CStr, CString},
    sync::OnceLock,
};
unsafe extern "C" {
    fn dlopen(name: *const c_char, flags: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
    fn dlerror() -> *const c_char;
}
pub(crate) type Create = unsafe extern "C" fn(
    *mut c_void,
    u32,
    *mut c_void,
    u32,
    *const u32,
    u32,
    u32,
    *const SwapDesc,
    *mut *mut c_void,
    *mut *mut c_void,
    *mut u32,
    *mut *mut c_void,
) -> i32;
struct Driver {
    create: Create,
    _handles: [usize; 2],
}
static DRIVER: OnceLock<Result<Driver, String>> = OnceLock::new();
fn load() -> Result<Driver, String> {
    let dir = std::env::var_os("PEEKGAME_DXVK_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/dxvk-x11/prefix/lib")
        });
    match std::env::var("DXVK_WSI_DRIVER") {
        Ok(value) if value != "PeekX11" => {
            return Err(format!(
                "DXVK_WSI_DRIVER={value} cannot accept PeekGame X11 surfaces"
            ))
        }
        _ => std::env::set_var("DXVK_WSI_DRIVER", "PeekX11"),
    }
    let mut handles = [0usize; 2];
    for (i, name) in ["libdxvk_dxgi.so.0", "libdxvk_d3d11.so.0"]
        .iter()
        .enumerate()
    {
        let path = CString::new(dir.join(name).as_os_str().as_encoded_bytes())
            .map_err(|e| e.to_string())?;
        let handle = unsafe { dlopen(path.as_ptr(), 2 | 0x100) }; // RTLD_NOW | RTLD_GLOBAL, resolves DXGI dependency.
        if handle.is_null() {
            let e = unsafe { dlerror() };
            return Err(if e.is_null() {
                "dlopen failed".into()
            } else {
                unsafe { CStr::from_ptr(e) }.to_string_lossy().into_owned()
            });
        }
        handles[i] = handle as usize;
    }
    let create = unsafe {
        dlsym(
            handles[1] as *mut c_void,
            c"D3D11CreateDeviceAndSwapChain".as_ptr(),
        )
    };
    if create.is_null() {
        return Err("DXVK D3D11 export is missing".into());
    }
    Ok(Driver {
        create: unsafe { std::mem::transmute::<*mut c_void, Create>(create) },
        _handles: handles,
    })
}
pub(crate) fn create() -> Result<Create, i32> {
    match DRIVER.get_or_init(load) {
        Ok(driver) => Ok(driver.create),
        Err(e) => {
            eprintln!("PeekGame DXVK: {e}; run tools/build_dxvk_x11.py");
            Err(E_FAIL)
        }
    }
}
