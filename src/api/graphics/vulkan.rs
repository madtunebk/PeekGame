//! vulkan-1.dll on the host Vulkan loader (libvulkan.so.1), after peek's
//! vulkan_bridge.inc. Windows code (DXVK's Windows d3d11.dll/dxgi.dll, or a game's
//! own Vulkan renderer) gets every command through vkGetInstanceProcAddr; each
//! returned pointer is a thunk that calls the host function with the System V ABI.
//! Vulkan structures have the same layout on Windows x64 and Linux x86-64, and
//! no command takes a structure by value, so only the calling convention changes:
//! one generic thunk serves every command whose arguments are integers or
//! pointers; the few commands with float arguments are written out here.
//! Windows-specific parts:
//! - VK_KHR_win32_surface is offered instead of the host's surface extensions,
//!   and vkCreateWin32SurfaceKHR makes an Xlib surface on the HWND's X window;
//! - debug callbacks (function pointers in structures) are not supported.
//!
//! Allocation callbacks are passed through: the Windows libraries PeekGame runs
//! (DXVK) pass none.
use crate::api::system::error;
use std::{
    collections::HashMap,
    ffi::{c_char, c_void, CStr, CString},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex, OnceLock,
    },
};
const SLOTS: usize = 2048;
static mut VK_REAL: [usize; SLOTS] = [0; SLOTS];
// Thunk i: eax = i, then the generic converter. 16 bytes each.
std::arch::global_asm!(
    ".text",
    ".p2align 4",
    ".globl peekgame_vk_thunks",
    "peekgame_vk_thunks:",
    ".set vk_index, 0",
    ".rept 2048",
    "mov $vk_index, %eax",
    "jmp peekgame_vk_generic",
    ".p2align 4",
    ".set vk_index, vk_index + 1",
    ".endr",
    // Windows x64 -> System V: integer arguments 1-4 from rcx, rdx, r8, r9; 5 and 6
    // from the caller's stack into r8 and r9; 7-16 copied to our stack. rsi, rdi and
    // xmm6-xmm15 are callee-saved on Windows only, so they are kept here.
    ".p2align 4",
    "peekgame_vk_generic:",
    "push %rbp",
    "mov %rsp, %rbp",
    "push %rsi",
    "push %rdi",
    "sub $240, %rsp",
    "movdqu %xmm6, 80(%rsp)",
    "movdqu %xmm7, 96(%rsp)",
    "movdqu %xmm8, 112(%rsp)",
    "movdqu %xmm9, 128(%rsp)",
    "movdqu %xmm10, 144(%rsp)",
    "movdqu %xmm11, 160(%rsp)",
    "movdqu %xmm12, 176(%rsp)",
    "movdqu %xmm13, 192(%rsp)",
    "movdqu %xmm14, 208(%rsp)",
    "movdqu %xmm15, 224(%rsp)",
    "lea {real}(%rip), %r11",
    "mov (%r11,%rax,8), %r11",
    ".irp k, 6,7,8,9,10,11,12,13,14,15",
    "mov (16+8*\\k)(%rbp), %rax",
    "mov %rax, (8*(\\k-6))(%rsp)",
    ".endr",
    "mov %rcx, %rdi",
    "mov %rdx, %rsi",
    "mov %r8, %rdx",
    "mov %r9, %rcx",
    "mov 48(%rbp), %r8",
    "mov 56(%rbp), %r9",
    "call *%r11",
    "movdqu 80(%rsp), %xmm6",
    "movdqu 96(%rsp), %xmm7",
    "movdqu 112(%rsp), %xmm8",
    "movdqu 128(%rsp), %xmm9",
    "movdqu 144(%rsp), %xmm10",
    "movdqu 160(%rsp), %xmm11",
    "movdqu 176(%rsp), %xmm12",
    "movdqu 192(%rsp), %xmm13",
    "movdqu 208(%rsp), %xmm14",
    "movdqu 224(%rsp), %xmm15",
    "add $240, %rsp",
    "pop %rdi",
    "pop %rsi",
    "pop %rbp",
    "ret",
    real = sym VK_REAL,
    options(att_syntax)
);
unsafe extern "C" {
    static peekgame_vk_thunks: u8;
    fn dlopen(name: *const c_char, flags: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
}
type ProcAddr = unsafe extern "C" fn(usize, *const c_char) -> usize;
struct Host {
    instance_proc: ProcAddr,
    device_proc: ProcAddr,
}
fn host() -> Option<&'static Host> {
    static HOST: OnceLock<Option<Host>> = OnceLock::new();
    HOST.get_or_init(|| unsafe {
        let lib = dlopen(c"libvulkan.so.1".as_ptr(), 2);
        if lib.is_null() {
            eprintln!("vulkan-1.dll: the host Vulkan loader libvulkan.so.1 is not installed");
            return None;
        }
        let gipa = dlsym(lib, c"vkGetInstanceProcAddr".as_ptr());
        let gdpa = dlsym(lib, c"vkGetDeviceProcAddr".as_ptr());
        if gipa.is_null() || gdpa.is_null() {
            return None;
        }
        Some(Host {
            instance_proc: std::mem::transmute::<*mut c_void, ProcAddr>(gipa),
            device_proc: std::mem::transmute::<*mut c_void, ProcAddr>(gdpa),
        })
    })
    .as_ref()
}
/// Host function by name: from the instance (or the device) given, else the last
/// instance.
static LAST_INSTANCE: AtomicUsize = AtomicUsize::new(0);
fn host_function(instance: usize, device: usize, name: &CStr) -> usize {
    let Some(h) = host() else {
        return 0;
    };
    unsafe {
        if device != 0 {
            let f = (h.device_proc)(device, name.as_ptr());
            if f != 0 {
                return f;
            }
        }
        let instance = if instance != 0 {
            instance
        } else {
            LAST_INSTANCE.load(Ordering::Acquire)
        };
        (h.instance_proc)(instance, name.as_ptr())
    }
}
/// The thunk for a host function (one slot per command name).
fn thunk(name: &CStr, real: usize) -> usize {
    static NAMES: Mutex<Option<HashMap<CString, usize>>> = Mutex::new(None);
    let mut names = NAMES.lock().unwrap();
    let names = names.get_or_insert_with(HashMap::new);
    let next = names.len();
    let slot = *names.entry(name.to_owned()).or_insert(next);
    if slot >= SLOTS {
        eprintln!("vulkan-1.dll: more than {SLOTS} commands requested");
        return 0;
    }
    unsafe {
        let table = &raw mut VK_REAL;
        if (*table)[slot] == 0 {
            (*table)[slot] = real;
        }
        &raw const peekgame_vk_thunks as usize + 16 * slot
    }
}

// ---- commands written out -------------------------------------------------------
const VK_SUCCESS: i32 = 0;
const VK_ERROR_INITIALIZATION_FAILED: i32 = -3;
const VK_ERROR_EXTENSION_NOT_PRESENT: i32 = -7;
const VK_INCOMPLETE: i32 = 5;
/// Commands PeekGame answers itself (Windows-specific, float arguments, function
/// pointers).
fn own(name: &[u8]) -> Option<usize> {
    let f: usize = match name {
        b"vkGetInstanceProcAddr" => get_instance_proc_addr as *const () as usize,
        b"vkGetDeviceProcAddr" => get_device_proc_addr as *const () as usize,
        b"vkCreateInstance" => create_instance as *const () as usize,
        b"vkEnumerateInstanceExtensionProperties" => {
            enumerate_instance_extensions as *const () as usize
        }
        b"vkCreateWin32SurfaceKHR" => create_win32_surface as *const () as usize,
        b"vkGetPhysicalDeviceWin32PresentationSupportKHR" => {
            win32_presentation_support as *const () as usize
        }
        b"vkCreateDebugUtilsMessengerEXT" | b"vkCreateDebugReportCallbackEXT" => {
            no_callbacks as *const () as usize
        }
        b"vkCmdSetDepthBias" => cmd_set_depth_bias as *const () as usize,
        b"vkCmdSetDepthBounds" => cmd_set_depth_bounds as *const () as usize,
        b"vkCmdSetLineWidth" => cmd_set_line_width as *const () as usize,
        b"vkCmdSetExtraPrimitiveOverestimationSizeEXT" => {
            cmd_set_overestimation as *const () as usize
        }
        b"vkSetDeviceMemoryPriorityEXT" => set_memory_priority as *const () as usize,
        _ => return None,
    };
    Some(f)
}
fn resolve(instance: usize, device: usize, name: *const c_char) -> usize {
    if name.is_null() || host().is_none() {
        return 0;
    }
    let name = unsafe { CStr::from_ptr(name) };
    if let Some(f) = own(name.to_bytes()) {
        // Only offer what the host has (for the Windows-only commands: the surface).
        let host_name: &[u8] = match name.to_bytes() {
            b"vkCreateWin32SurfaceKHR" => b"vkCreateXlibSurfaceKHR\0",
            b"vkGetPhysicalDeviceWin32PresentationSupportKHR" => {
                b"vkGetPhysicalDeviceXlibPresentationSupportKHR\0"
            }
            b"vkGetInstanceProcAddr"
            | b"vkGetDeviceProcAddr"
            | b"vkCreateInstance"
            | b"vkEnumerateInstanceExtensionProperties" => return f,
            b"vkCreateDebugUtilsMessengerEXT" | b"vkCreateDebugReportCallbackEXT" => return f,
            _ => name.to_bytes_with_nul(),
        };
        let host_name = CStr::from_bytes_with_nul(host_name).unwrap();
        let real = host_function(instance, device, host_name);
        if real == 0 {
            return 0;
        }
        FLOATS
            .lock()
            .unwrap()
            .get_or_insert_with(HashMap::new)
            .insert(name.to_owned(), real);
        return f;
    }
    let real = host_function(instance, device, name);
    if real == 0 {
        return 0;
    }
    thunk(name, real)
}
static FLOATS: Mutex<Option<HashMap<CString, usize>>> = Mutex::new(None);
fn written(name: &CStr) -> usize {
    FLOATS
        .lock()
        .unwrap()
        .get_or_insert_with(HashMap::new)
        .get(name)
        .copied()
        .unwrap_or(0)
}
pub(crate) unsafe extern "win64" fn get_instance_proc_addr(
    instance: usize,
    name: *const c_char,
) -> usize {
    resolve(instance, 0, name)
}
pub(crate) unsafe extern "win64" fn get_device_proc_addr(
    device: usize,
    name: *const c_char,
) -> usize {
    resolve(0, device, name)
}
const XLIB_SURFACE: &[u8] = b"VK_KHR_xlib_surface";
const WIN32_SURFACE: &[u8] = b"VK_KHR_win32_surface";
/// Host surface extensions the program must not see (it would try to use them).
fn hidden(name: &[u8]) -> bool {
    matches!(
        name,
        b"VK_KHR_xlib_surface"
            | b"VK_KHR_xcb_surface"
            | b"VK_KHR_wayland_surface"
            | b"VK_EXT_debug_report"
            | b"VK_EXT_headless_surface"
            | b"VK_KHR_display"
            | b"VK_EXT_acquire_xlib_display"
            | b"VK_EXT_direct_mode_display"
            | b"VK_EXT_acquire_drm_display"
    )
}
/// VkExtensionProperties: extensionName[256], specVersion.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Extension {
    name: [c_char; 256],
    version: u32,
}
pub(crate) unsafe extern "win64" fn enumerate_instance_extensions(
    layer: *const c_char,
    count: *mut u32,
    out: *mut Extension,
) -> i32 {
    if count.is_null() {
        return VK_ERROR_INITIALIZATION_FAILED;
    }
    let real = host_function(0, 0, c"vkEnumerateInstanceExtensionProperties");
    if real == 0 {
        return VK_ERROR_INITIALIZATION_FAILED;
    }
    let real: unsafe extern "C" fn(*const c_char, *mut u32, *mut Extension) -> i32 =
        unsafe { std::mem::transmute(real) };
    let mut n = 0u32;
    let r = unsafe { real(layer, &mut n, std::ptr::null_mut()) };
    if r != VK_SUCCESS {
        return r;
    }
    let mut all = vec![
        Extension {
            name: [0; 256],
            version: 0
        };
        n as usize
    ];
    let r = unsafe { real(layer, &mut n, all.as_mut_ptr()) };
    if r < 0 {
        return r;
    }
    all.truncate(n as usize);
    let name_of = |e: &Extension| {
        unsafe { CStr::from_ptr(e.name.as_ptr()) }
            .to_bytes()
            .to_vec()
    };
    let has_xlib = all.iter().any(|e| name_of(e) == XLIB_SURFACE);
    let mut list: Vec<Extension> = all.into_iter().filter(|e| !hidden(&name_of(e))).collect();
    if layer.is_null() && has_xlib {
        let mut win32 = Extension {
            name: [0; 256],
            version: 6,
        };
        for (i, b) in WIN32_SURFACE.iter().enumerate() {
            win32.name[i] = *b as c_char;
        }
        list.push(win32);
    }
    unsafe {
        if out.is_null() {
            *count = list.len() as u32;
            return VK_SUCCESS;
        }
        let n = (*count as usize).min(list.len());
        std::ptr::copy_nonoverlapping(list.as_ptr(), out, n);
        *count = n as u32;
        if n < list.len() {
            VK_INCOMPLETE
        } else {
            VK_SUCCESS
        }
    }
}
/// VkInstanceCreateInfo: sType, pNext, flags, pApplicationInfo, enabledLayerCount,
/// ppEnabledLayerNames, enabledExtensionCount @48, ppEnabledExtensionNames @56.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct InstanceInfo {
    kind: u32,
    next: *const c_void,
    flags: u32,
    application: *const c_void,
    layer_count: u32,
    layers: *const *const c_char,
    extension_count: u32,
    extensions: *const *const c_char,
}
pub(crate) unsafe extern "win64" fn create_instance(
    info: *const InstanceInfo,
    allocator: *const c_void,
    instance: *mut usize,
) -> i32 {
    if info.is_null() || instance.is_null() {
        return VK_ERROR_INITIALIZATION_FAILED;
    }
    let real = host_function(0, 0, c"vkCreateInstance");
    if real == 0 {
        return VK_ERROR_INITIALIZATION_FAILED;
    }
    let real: unsafe extern "C" fn(*const InstanceInfo, *const c_void, *mut usize) -> i32 =
        unsafe { std::mem::transmute(real) };
    let mut copy = unsafe { *info };
    let original: &[*const c_char] = if copy.extension_count == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(copy.extensions, copy.extension_count as usize) }
    };
    let mut names: Vec<*const c_char> = Vec::new();
    for &e in original {
        let n = unsafe { CStr::from_ptr(e) }.to_bytes();
        if n == WIN32_SURFACE {
            names.push(c"VK_KHR_xlib_surface".as_ptr());
        } else if n == b"VK_EXT_debug_report" {
            continue;
        } else {
            names.push(e);
        }
    }
    copy.extension_count = names.len() as u32;
    copy.extensions = names.as_ptr();
    let r = unsafe { real(&copy, allocator, instance) };
    if r == VK_SUCCESS {
        LAST_INSTANCE.store(unsafe { *instance }, Ordering::Release);
    }
    r
}
/// VkWin32SurfaceCreateInfoKHR: sType, pNext, flags, hinstance, hwnd @32.
pub(crate) unsafe extern "win64" fn create_win32_surface(
    instance: usize,
    info: *const u8,
    allocator: *const c_void,
    surface: *mut u64,
) -> i32 {
    let real = written(c"vkCreateWin32SurfaceKHR");
    if info.is_null() || real == 0 {
        return VK_ERROR_EXTENSION_NOT_PRESENT;
    }
    let hwnd = unsafe { info.add(32).cast::<usize>().read_unaligned() };
    let mut native = crate::api::window::NativeWindow {
        display: std::ptr::null_mut(),
        window: 0,
    };
    if unsafe { crate::api::window::peekgame_x11_window(hwnd, &mut native) } != 0 {
        eprintln!("vkCreateWin32SurfaceKHR: HWND {hwnd:#x} is not a PeekGame window");
        return VK_ERROR_INITIALIZATION_FAILED;
    }
    // VkXlibSurfaceCreateInfoKHR: sType 1000004000, pNext, flags, dpy, window.
    #[repr(C)]
    struct Xlib {
        kind: u32,
        next: *const c_void,
        flags: u32,
        display: *mut c_void,
        window: u64,
    }
    let xlib = Xlib {
        kind: 1_000_004_000,
        next: std::ptr::null(),
        flags: 0,
        display: native.display,
        window: native.window,
    };
    let real: unsafe extern "C" fn(usize, *const Xlib, *const c_void, *mut u64) -> i32 =
        unsafe { std::mem::transmute(real) };
    unsafe { real(instance, &xlib, allocator, surface) }
}
/// Every queue family that can present to an X window presents to our windows.
pub(crate) unsafe extern "win64" fn win32_presentation_support(
    _device: usize,
    _family: u32,
) -> u32 {
    1
}
pub(crate) unsafe extern "win64" fn no_callbacks(
    _instance: usize,
    _info: *const c_void,
    _allocator: *const c_void,
    _out: *mut u64,
) -> i32 {
    error::set(50);
    VK_ERROR_EXTENSION_NOT_PRESENT
}
macro_rules! float_command {
    ($fn:ident, $name:literal, ($($a:ident: $t:ty),*)) => {
        pub(crate) unsafe extern "win64" fn $fn(first: usize, $($a: $t),*) {
            let real = written($name);
            if real != 0 {
                let real: unsafe extern "C" fn(usize, $($t),*) = unsafe { std::mem::transmute(real) };
                unsafe { real(first, $($a),*) };
            }
        }
    };
}
float_command!(cmd_set_depth_bias, c"vkCmdSetDepthBias", (constant: f32, clamp: f32, slope: f32));
float_command!(cmd_set_depth_bounds, c"vkCmdSetDepthBounds", (min: f32, max: f32));
float_command!(cmd_set_line_width, c"vkCmdSetLineWidth", (width: f32));
float_command!(cmd_set_overestimation, c"vkCmdSetExtraPrimitiveOverestimationSizeEXT", (size: f32));
pub(crate) unsafe extern "win64" fn set_memory_priority(device: usize, memory: u64, priority: f32) {
    let real = written(c"vkSetDeviceMemoryPriorityEXT");
    if real != 0 {
        let real: unsafe extern "C" fn(usize, u64, f32) = unsafe { std::mem::transmute(real) };
        unsafe { real(device, memory, priority) };
    }
}
/// Exports of vulkan-1.dll that programs take with GetProcAddress: the global
/// commands (others come through vkGetInstanceProcAddr).
pub(crate) unsafe extern "win64" fn enumerate_instance_version(version: *mut u32) -> i32 {
    let f = resolve(0, 0, c"vkEnumerateInstanceVersion".as_ptr());
    if f == 0 {
        unsafe { *version = 1 << 22 };
        return VK_SUCCESS;
    }
    let f: unsafe extern "win64" fn(*mut u32) -> i32 = unsafe { std::mem::transmute(f) };
    unsafe { f(version) }
}
pub(crate) unsafe extern "win64" fn enumerate_instance_layers(
    count: *mut u32,
    out: *mut c_void,
) -> i32 {
    let f = resolve(0, 0, c"vkEnumerateInstanceLayerProperties".as_ptr());
    if f == 0 {
        return VK_ERROR_INITIALIZATION_FAILED;
    }
    let f: unsafe extern "win64" fn(*mut u32, *mut c_void) -> i32 =
        unsafe { std::mem::transmute(f) };
    unsafe { f(count, out) }
}

// ---- d3d12.dll: reported unavailable, so programs choose Direct3D 11 -------------
const DXGI_ERROR_UNSUPPORTED: i32 = 0x887a_0004u32 as i32;
const E_NOINTERFACE: i32 = 0x8000_4002u32 as i32;
pub(crate) extern "win64" fn d3d12_create_device(
    _adapter: usize,
    _level: u32,
    _iid: usize,
    _out: usize,
) -> i32 {
    DXGI_ERROR_UNSUPPORTED
}
pub(crate) extern "win64" fn d3d12_no_interface(_a: usize, _b: usize, _c: usize, _d: usize) -> i32 {
    E_NOINTERFACE
}
