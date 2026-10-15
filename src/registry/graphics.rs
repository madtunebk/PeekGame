use super::*;
use crate::api::graphics::directx::d3d11::create;
use crate::api::graphics::vulkan;
exports! {
    export!(
        "d3d12.dll",
        "D3D12CreateDevice",
        "graphics/directx/d3d12",
        "partial",
        "Direct3D 12 unavailable: DXGI_ERROR_UNSUPPORTED",
        vulkan::d3d12_create_device
    ),
    export!(
        "d3d12.dll",
        "D3D12GetDebugInterface",
        "graphics/directx/d3d12",
        "partial",
        "E_NOINTERFACE",
        vulkan::d3d12_no_interface
    ),
    export!(
        "d3d12.dll",
        "D3D12SerializeRootSignature",
        "graphics/directx/d3d12",
        "partial",
        "E_NOINTERFACE",
        vulkan::d3d12_no_interface
    ),
    export!(
        "d3d12.dll",
        "D3D12CreateRootSignatureDeserializer",
        "graphics/directx/d3d12",
        "partial",
        "E_NOINTERFACE",
        vulkan::d3d12_no_interface
    ),
    export!(
        "d3d12.dll",
        "D3D12EnableExperimentalFeatures",
        "graphics/directx/d3d12",
        "partial",
        "E_NOINTERFACE",
        vulkan::d3d12_no_interface
    ),
    export!(
        "d3d12.dll",
        "D3D12GetInterface",
        "graphics/directx/d3d12",
        "partial",
        "E_NOINTERFACE",
        vulkan::d3d12_no_interface
    ),
    export!(
        "vulkan-1.dll",
        "vkGetInstanceProcAddr",
        "graphics/vulkan",
        "partial",
        "host libvulkan through ABI thunks",
        vulkan::get_instance_proc_addr
    ),
    export!(
        "vulkan-1.dll",
        "vkGetDeviceProcAddr",
        "graphics/vulkan",
        "partial",
        "host libvulkan through ABI thunks",
        vulkan::get_device_proc_addr
    ),
    export!(
        "vulkan-1.dll",
        "vkCreateInstance",
        "graphics/vulkan",
        "partial",
        "VK_KHR_win32_surface becomes VK_KHR_xlib_surface",
        vulkan::create_instance
    ),
    export!(
        "vulkan-1.dll",
        "vkEnumerateInstanceExtensionProperties",
        "graphics/vulkan",
        "partial",
        "host extensions; win32 surface instead of Linux ones",
        vulkan::enumerate_instance_extensions
    ),
    export!(
        "vulkan-1.dll",
        "vkEnumerateInstanceVersion",
        "graphics/vulkan",
        "partial",
        "host",
        vulkan::enumerate_instance_version
    ),
    export!(
        "vulkan-1.dll",
        "vkEnumerateInstanceLayerProperties",
        "graphics/vulkan",
        "partial",
        "host",
        vulkan::enumerate_instance_layers
    ),
    export!(
        "vulkan-1.dll",
        "vkCreateWin32SurfaceKHR",
        "graphics/vulkan",
        "partial",
        "Xlib surface on the HWND's X window",
        vulkan::create_win32_surface
    ),
    export!(
        "d3d11.dll",
        "D3D11CreateDeviceAndSwapChain",
        "graphics/directx/d3d11",
        "partial",
        "DXVK Native with X11/Vulkan WSI; windowed default adapter; testgame COM subset; requires built backend",
        create::create_device_and_swap_chain
    ),
}
