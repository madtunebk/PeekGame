use super::*;
use crate::api::graphics::directx::d3d11::create;
pub(super) static EXPORTS:&[Export]=&[Export{
    dll:"d3d11.dll",name:"D3D11CreateDeviceAndSwapChain",category:"graphics/directx/d3d11",
    status:"partial",scope:"DXVK Native with X11/Vulkan WSI; windowed default adapter; testgame COM subset; requires built backend",
    address:Some(||create::create_device_and_swap_chain as *const c_void),
}];
