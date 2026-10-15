use super::tables;
use crate::host::graphics::surface::Surface;
use std::{
    collections::HashMap,
    ffi::c_void,
    sync::{atomic::AtomicU32, Arc, Mutex, OnceLock},
};
#[repr(u8)]
#[derive(Clone, Copy, Hash, Eq, PartialEq)]
pub enum Kind {
    Device,
    Context,
    SwapChain,
    Texture,
    Rtv,
}
impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Device => "ID3D11Device",
            Self::Context => "ID3D11DeviceContext",
            Self::SwapChain => "IDXGISwapChain",
            Self::Texture => "ID3D11Texture2D",
            Self::Rtv => "ID3D11RenderTargetView",
        }
    }
    pub fn methods(self) -> &'static str {
        match self {
            Self::Device => super::names::ID3D11DEVICE,
            Self::Context => super::names::ID3D11DEVICECONTEXT,
            Self::SwapChain => super::names::IDXGISWAPCHAIN,
            Self::Texture => super::names::ID3D11TEXTURE2D,
            Self::Rtv => super::names::ID3D11RENDERTARGETVIEW,
        }
    }
}
#[repr(C)]
pub struct Com {
    pub table: *const usize,
    pub native: *mut c_void,
    pub kind: Kind,
    pub refs: AtomicU32,
    pub surface: Option<Arc<Surface>>,
}
pub(super) fn cache() -> &'static Mutex<HashMap<(usize, Kind), usize>> {
    static CACHE: OnceLock<Mutex<HashMap<(usize, Kind), usize>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}
pub unsafe fn wrap(native: *mut c_void, kind: Kind, surface: Option<Arc<Surface>>) -> *mut Com {
    if native.is_null() {
        return std::ptr::null_mut();
    }
    let mut cache = cache().lock().unwrap();
    if let Some(&existing) = cache.get(&(native as usize, kind)) {
        unsafe {
            (*(existing as *mut Com))
                .refs
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        return existing as *mut Com;
    }
    let object = Box::into_raw(Box::new(Com {
        table: tables::get(kind),
        native,
        kind,
        refs: AtomicU32::new(1),
        surface,
    }));
    cache.insert((native as usize, kind), object as usize);
    object
}
pub unsafe fn native_of(object: *mut Com) -> *mut c_void {
    if object.is_null() {
        return std::ptr::null_mut();
    }
    if !cache()
        .lock()
        .unwrap()
        .values()
        .any(|p| *p == object as usize)
    {
        eprintln!("PeekGame DirectX: foreign COM pointer {object:p}");
        std::process::exit(1);
    }
    unsafe { (*object).native }
}
pub unsafe fn surface_of(object: *mut Com) -> Option<Arc<Surface>> {
    unsafe {
        native_of(object);
        (*object).surface.clone()
    }
}
