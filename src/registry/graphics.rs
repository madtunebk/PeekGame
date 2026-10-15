use super::*;
use crate::api::graphics::directx::d3d11::create;
exports! {
    export!(
        "d3d11.dll",
        "D3D11CreateDeviceAndSwapChain",
        "graphics/directx/d3d11",
        "partial",
        "DXVK Native with X11/Vulkan WSI; windowed default adapter; testgame COM subset; requires built backend",
        create::create_device_and_swap_chain
    ),
}
