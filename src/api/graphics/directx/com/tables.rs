use super::{refs, Com, Kind};
use crate::api::graphics::directx::{
    d3d11::{context, device, texture},
    dxgi::swapchain,
};
use std::sync::OnceLock;
unsafe extern "win64" fn missing<const SLOT: usize>(object: *mut Com) -> ! {
    unsafe {
        super::native_of(object);
    }
    let kind = unsafe { (*object).kind };
    let method = kind.methods().split(',').nth(SLOT).unwrap_or("unknown");
    eprintln!(
        "PeekGame: unsupported {}::{} (COM slot {})",
        kind.name(),
        method,
        SLOT
    );
    std::process::exit(1)
}
macro_rules! slots {($($slot:literal),*)=>{vec![$(missing::<$slot> as *const () as usize),*]};}
fn make(kind: Kind) -> Vec<usize> {
    let mut v = slots!(
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
        48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70,
        71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93,
        94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112,
        113, 114
    );
    v.truncate(kind.methods().split(',').count());
    v[0] = refs::query as *const () as usize;
    v[1] = refs::add_ref as *const () as usize;
    v[2] = refs::release as *const () as usize;
    match kind {
        Kind::Device => {
            v[5] = device::create_texture as *const () as usize;
            v[9] = device::create_view as *const () as usize;
        }
        Kind::Context => {
            v[14] = context::map as *const () as usize;
            v[15] = context::unmap as *const () as usize;
            v[33] = context::set_targets as *const () as usize;
            v[47] = context::copy as *const () as usize;
            v[50] = context::clear_view as *const () as usize;
            v[110] = context::clear_state as *const () as usize;
        }
        Kind::SwapChain => {
            v[8] = swapchain::present as *const () as usize;
            v[9] = swapchain::get_buffer as *const () as usize;
            v[13] = swapchain::resize_buffers as *const () as usize;
        }
        Kind::Texture => v[10] = texture::get_desc as *const () as usize,
        Kind::Rtv => {}
    }
    v
}
pub(super) fn get(kind: Kind) -> *const usize {
    static TABLES: OnceLock<Vec<Vec<usize>>> = OnceLock::new();
    TABLES
        .get_or_init(|| {
            [
                Kind::Device,
                Kind::Context,
                Kind::SwapChain,
                Kind::Texture,
                Kind::Rtv,
            ]
            .map(make)
            .into()
        })
        .get(kind as usize)
        .unwrap()
        .as_ptr()
}
