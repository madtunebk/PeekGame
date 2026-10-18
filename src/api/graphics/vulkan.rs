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
        if instance != 0 {
            return (h.instance_proc)(instance, name.as_ptr());
        }
        let global = (h.instance_proc)(0, name.as_ptr());
        if global != 0 {
            return global;
        }
        let last = LAST_INSTANCE.load(Ordering::Acquire);
        if last != 0 {
            return (h.instance_proc)(last, name.as_ptr());
        }
        0
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
        b"vkCreateDebugUtilsMessengerEXT" => {
            create_debug_utils_messenger as *const () as usize
        }
        b"vkCreateDebugReportCallbackEXT" => {
            no_callbacks as *const () as usize
        }
        b"vkCmdSetDepthBias" => cmd_set_depth_bias as *const () as usize,
        b"vkCmdSetDepthBounds" => cmd_set_depth_bounds as *const () as usize,
        b"vkCmdSetLineWidth" => cmd_set_line_width as *const () as usize,
        b"vkCmdSetExtraPrimitiveOverestimationSizeEXT" => {
            cmd_set_overestimation as *const () as usize
        }
        b"vkSetDeviceMemoryPriorityEXT" => set_memory_priority as *const () as usize,
        b"vkCreateSwapchainKHR" => create_swapchain as *const () as usize,
        b"vkQueuePresentKHR" => queue_present as *const () as usize,
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
            | b"vkEnumerateInstanceExtensionProperties"
            | b"vkCreateDebugReportCallbackEXT" => return f,
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
    let mut host_messenger: DebugUtilsMessengerCreateInfo;
    if !copy.next.is_null() && unsafe { *copy.next.cast::<u32>() } == 1_000_128_004 {
        let win_info = unsafe { &*copy.next.cast::<DebugUtilsMessengerCreateInfo>() };
        let (hm, _wrap) = wrap_messenger(win_info);
        host_messenger = hm;
        host_messenger.next = std::ptr::null_mut();
        copy.next = &host_messenger as *const _ as *const c_void;
    }
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
    // A static import binds here without vkGetInstanceProcAddr filling the cache.
    let mut real = written(c"vkCreateWin32SurfaceKHR");
    if real == 0 {
        real = host_function(instance, 0, c"vkCreateXlibSurfaceKHR");
        if real != 0 {
            FLOATS
                .lock()
                .unwrap()
                .get_or_insert_with(HashMap::new)
                .insert(c"vkCreateWin32SurfaceKHR".to_owned(), real);
        }
    }
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
/// VkSwapchainCreateInfoKHR.presentMode is a u32 at offset 88 on x64. PeekGame
/// forwards it unchanged; PEEKGAME_VULKAN_PRESENT overrides it (fifo = vsync, no
/// tearing; mailbox = no tearing, low latency; immediate = uncapped/tearing;
/// relaxed = fifo-relaxed). The requested/applied mode is logged with PEEKGAME_TRACE_GL.
pub(crate) unsafe extern "win64" fn create_swapchain(
    device: usize,
    info: *const c_void,
    allocator: *const c_void,
    swapchain: *mut u64,
) -> i32 {
    let mut real = written(c"vkCreateSwapchainKHR");
    if real == 0 {
        real = host_function(0, device, c"vkCreateSwapchainKHR");
    }
    if info.is_null() || real == 0 {
        return VK_ERROR_INITIALIZATION_FAILED;
    }
    const PRESENT_MODE_OFFSET: usize = 88;
    let slot = unsafe { (info as *const u8).add(PRESENT_MODE_OFFSET) as *mut u32 };
    let requested = unsafe { slot.read_unaligned() };
    let override_mode = std::env::var("PEEKGAME_VULKAN_PRESENT").ok().and_then(|v| {
        Some(match v.trim().to_ascii_lowercase().as_str() {
            "immediate" => 0u32,
            "mailbox" => 1,
            "fifo" | "vsync" => 2,
            "relaxed" | "fifo_relaxed" => 3,
            _ => return None,
        })
    });
    if let Some(mode) = override_mode {
        unsafe { slot.write_unaligned(mode) };
    }
    // PEEKGAME_VULKAN_IMAGES caps the swapchain image count (VkSwapchainCreateInfoKHR
    // .minImageCount @32). FIFO with a deep queue adds frames of latency; clamping to
    // 2-3 keeps vsync's low GPU use with a short, low-latency queue. Only lowers the
    // count the program asked for, never below 2 (the FIFO minimum).
    if let Ok(cap) = std::env::var("PEEKGAME_VULKAN_IMAGES") {
        if let Ok(cap) = cap.trim().parse::<u32>() {
            let field = unsafe { (info as *const u8).add(32) as *mut u32 };
            let requested = unsafe { field.read_unaligned() };
            let wanted = cap.max(2);
            if wanted < requested {
                unsafe { field.write_unaligned(wanted) };
            }
        }
    }
    if std::env::var_os("PEEKGAME_TRACE_WINDOW").is_some() {
        // VkSwapchainCreateInfoKHR.imageExtent @44 (width) and @48 (height).
        let (w, h) = unsafe { ((info as *const u8).add(44).cast::<u32>().read_unaligned(), (info as *const u8).add(48).cast::<u32>().read_unaligned()) };
        eprintln!("[window] vkCreateSwapchainKHR extent {w}x{h}");
    }
    if std::env::var_os("PEEKGAME_TRACE_GL").is_some() {
        match override_mode {
            Some(mode) => eprintln!("[gl] vkCreateSwapchainKHR present mode {requested} -> {mode}"),
            None => eprintln!("[gl] vkCreateSwapchainKHR present mode {requested}"),
        }
    }
    let real: unsafe extern "C" fn(usize, *const c_void, *const c_void, *mut u64) -> i32 =
        unsafe { std::mem::transmute(real) };
    unsafe { real(device, info, allocator, swapchain) }
}
/// PEEKGAME_VULKAN_FPS caps the present rate: after each vkQueuePresentKHR the
/// thread sleeps until the next frame boundary, so mailbox/immediate stay tear-free
/// or smooth without the GPU running flat-out. 0 or unset = no cap (pure passthrough).
fn frame_interval() -> Option<std::time::Duration> {
    static INTERVAL: std::sync::OnceLock<Option<std::time::Duration>> = std::sync::OnceLock::new();
    *INTERVAL.get_or_init(|| {
        let fps: f64 = std::env::var("PEEKGAME_VULKAN_FPS").ok()?.trim().parse().ok()?;
        (fps > 0.0).then(|| std::time::Duration::from_secs_f64(1.0 / fps))
    })
}
pub(crate) unsafe extern "win64" fn queue_present(queue: usize, info: *const c_void) -> i32 {
    // A static import binds here without vkGetDeviceProcAddr filling the cache.
    let mut real = written(c"vkQueuePresentKHR");
    if real == 0 {
        real = host_function(LAST_INSTANCE.load(Ordering::Acquire), 0, c"vkQueuePresentKHR");
        if real != 0 {
            FLOATS
                .lock()
                .unwrap()
                .get_or_insert_with(HashMap::new)
                .insert(c"vkQueuePresentKHR".to_owned(), real);
        }
    }
    if real == 0 {
        return VK_ERROR_INITIALIZATION_FAILED;
    }
    let real: unsafe extern "C" fn(usize, *const c_void) -> i32 = unsafe { std::mem::transmute(real) };
    let result = unsafe { real(queue, info) };
    if let Some(interval) = frame_interval() {
        // One deadline per presenting thread (the render thread), advanced by the
        // interval; a late frame resets it so we never burst to catch up.
        thread_local! {
            static NEXT: std::cell::Cell<Option<std::time::Instant>> = const { std::cell::Cell::new(None) };
        }
        NEXT.with(|cell| {
            let now = std::time::Instant::now();
            let deadline = cell.get().filter(|&d| d > now);
            if let Some(deadline) = deadline {
                std::thread::sleep(deadline - now);
                cell.set(Some(deadline + interval));
            } else {
                cell.set(Some(now + interval));
            }
        });
    }
    result
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

type WinDebugCallback = unsafe extern "win64" fn(u32, u32, *const c_void, *mut c_void) -> u32;

struct DebugWrap {
    callback: WinDebugCallback,
    user: *mut c_void,
}

unsafe extern "C" fn vk_debug_trampoline(
    severity: u32,
    types: u32,
    data: *const c_void,
    user: *mut c_void,
) -> u32 {
    let wrap = unsafe { &*user.cast::<DebugWrap>() };
    unsafe { (wrap.callback)(severity, types, data, wrap.user) }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DebugUtilsMessengerCreateInfo {
    pub s_type: u32,
    pub next: *mut c_void,
    pub flags: u32,
    pub severity: u32,
    pub message_type: u32,
    pub callback: usize,
    pub user_data: *mut c_void,
}

fn wrap_messenger(
    win: &DebugUtilsMessengerCreateInfo,
) -> (DebugUtilsMessengerCreateInfo, *mut DebugWrap) {
    let wrap = Box::into_raw(Box::new(DebugWrap {
        callback: unsafe { std::mem::transmute::<usize, WinDebugCallback>(win.callback) },
        user: win.user_data,
    }));
    let mut host = *win;
    host.callback = vk_debug_trampoline as *const () as usize;
    host.user_data = wrap.cast();
    (host, wrap)
}

pub(crate) unsafe extern "win64" fn create_debug_utils_messenger(
    instance: usize,
    info: *const DebugUtilsMessengerCreateInfo,
    allocator: *const c_void,
    out: *mut u64,
) -> i32 {
    if info.is_null() || out.is_null() {
        return VK_ERROR_INITIALIZATION_FAILED;
    }
    let real = written(c"vkCreateDebugUtilsMessengerEXT");
    if real == 0 {
        return VK_ERROR_EXTENSION_NOT_PRESENT;
    }
    let (host_info, _wrap) = wrap_messenger(unsafe { &*info });
    let real: unsafe extern "C" fn(
        usize,
        *const DebugUtilsMessengerCreateInfo,
        *const c_void,
        *mut u64,
    ) -> i32 = unsafe { std::mem::transmute(real) };
    unsafe { real(instance, &host_info, allocator, out) }
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

pub(crate) unsafe extern "win64" fn destroy_instance(
    instance: usize,
    allocator: *const c_void,
) {
    let f = resolve(instance, 0, c"vkDestroyInstance".as_ptr());
    if f != 0 {
        let f: unsafe extern "win64" fn(usize, *const c_void) = unsafe { std::mem::transmute(f) };
        unsafe { f(instance, allocator) };
    }
    let _ = LAST_INSTANCE.compare_exchange(instance, 0, Ordering::Release, Ordering::Relaxed);
}

macro_rules! vk_forward {
    ($fn:ident, $name:literal, ($first:ident: $first_ty:ty $(, $arg:ident: $ty:ty)*) -> $ret:ty) => {
        pub(crate) unsafe extern "win64" fn $fn($first: $first_ty $(, $arg: $ty)*) -> $ret {
            static CACHE: AtomicUsize = AtomicUsize::new(0);
            let mut f_ptr = CACHE.load(Ordering::Relaxed);
            if f_ptr == 0 {
                let inst = LAST_INSTANCE.load(Ordering::Acquire);
                f_ptr = resolve(inst, $first as usize, concat!($name, "\0").as_ptr().cast());
                if f_ptr == 0 {
                    f_ptr = resolve(inst, $first as usize, concat!($name, "KHR\0").as_ptr().cast());
                }
                if f_ptr != 0 {
                    CACHE.store(f_ptr, Ordering::Relaxed);
                }
            }
            if f_ptr == 0 {
                eprintln!("vulkan-1.dll: failed to resolve {}", $name);
                return VK_ERROR_INITIALIZATION_FAILED as $ret;
            }
            let f: unsafe extern "win64" fn($first_ty $(, $ty)*) -> $ret =
                unsafe { std::mem::transmute(f_ptr) };
            unsafe { f($first $(, $arg)*) }
        }
    };
    ($fn:ident, $name:literal, ($first:ident: $first_ty:ty $(, $arg:ident: $ty:ty)*)) => {
        pub(crate) unsafe extern "win64" fn $fn($first: $first_ty $(, $arg: $ty)*) {
            static CACHE: AtomicUsize = AtomicUsize::new(0);
            let mut f_ptr = CACHE.load(Ordering::Relaxed);
            if f_ptr == 0 {
                let inst = LAST_INSTANCE.load(Ordering::Acquire);
                f_ptr = resolve(inst, $first as usize, concat!($name, "\0").as_ptr().cast());
                if f_ptr == 0 {
                    f_ptr = resolve(inst, $first as usize, concat!($name, "KHR\0").as_ptr().cast());
                }
                if f_ptr != 0 {
                    CACHE.store(f_ptr, Ordering::Relaxed);
                }
            }
            if f_ptr != 0 {
                let f: unsafe extern "win64" fn($first_ty $(, $ty)*) =
                    unsafe { std::mem::transmute(f_ptr) };
                unsafe { f($first $(, $arg)*) };
            }
        }
    };
}

vk_forward!(enumerate_physical_devices, "vkEnumeratePhysicalDevices", (instance: usize, count: *mut u32, devices: *mut usize) -> i32);
vk_forward!(get_physical_device_properties, "vkGetPhysicalDeviceProperties", (device: usize, props: *mut c_void));
vk_forward!(get_physical_device_properties2, "vkGetPhysicalDeviceProperties2", (device: usize, props: *mut c_void));
vk_forward!(get_physical_device_features2, "vkGetPhysicalDeviceFeatures2", (device: usize, features: *mut c_void));
vk_forward!(get_physical_device_format_properties, "vkGetPhysicalDeviceFormatProperties", (device: usize, format: u32, props: *mut c_void));
vk_forward!(get_physical_device_queue_family_properties, "vkGetPhysicalDeviceQueueFamilyProperties", (device: usize, count: *mut u32, props: *mut c_void));
vk_forward!(get_physical_device_memory_properties, "vkGetPhysicalDeviceMemoryProperties", (device: usize, props: *mut c_void));
vk_forward!(enumerate_device_extension_properties, "vkEnumerateDeviceExtensionProperties", (device: usize, layer: *const c_char, count: *mut u32, props: *mut c_void) -> i32);
vk_forward!(create_device, "vkCreateDevice", (device: usize, info: *const c_void, allocator: *const c_void, out: *mut usize) -> i32);
vk_forward!(destroy_device, "vkDestroyDevice", (device: usize, allocator: *const c_void));
vk_forward!(get_device_queue, "vkGetDeviceQueue", (device: usize, family: u32, index: u32, out: *mut usize));
vk_forward!(device_wait_idle, "vkDeviceWaitIdle", (device: usize) -> i32);
vk_forward!(queue_submit, "vkQueueSubmit", (queue: usize, count: u32, submits: *const c_void, fence: u64) -> i32);
vk_forward!(queue_bind_sparse, "vkQueueBindSparse", (queue: usize, count: u32, binds: *const c_void, fence: u64) -> i32);
vk_forward!(get_physical_device_surface_support_khr, "vkGetPhysicalDeviceSurfaceSupportKHR", (device: usize, family: u32, surface: u64, supported: *mut u32) -> i32);
vk_forward!(get_physical_device_surface_capabilities_khr, "vkGetPhysicalDeviceSurfaceCapabilitiesKHR", (device: usize, surface: u64, caps: *mut c_void) -> i32);
vk_forward!(get_physical_device_surface_formats_khr, "vkGetPhysicalDeviceSurfaceFormatsKHR", (device: usize, surface: u64, count: *mut u32, formats: *mut c_void) -> i32);
vk_forward!(get_physical_device_surface_present_modes_khr, "vkGetPhysicalDeviceSurfacePresentModesKHR", (device: usize, surface: u64, count: *mut u32, modes: *mut u32) -> i32);
vk_forward!(destroy_swapchain_khr, "vkDestroySwapchainKHR", (device: usize, swapchain: u64, allocator: *const c_void));
vk_forward!(get_swapchain_images_khr, "vkGetSwapchainImagesKHR", (device: usize, swapchain: u64, count: *mut u32, images: *mut u64) -> i32);
vk_forward!(acquire_next_image_khr, "vkAcquireNextImageKHR", (device: usize, swapchain: u64, timeout: u64, semaphore: u64, fence: u64, index: *mut u32) -> i32);
vk_forward!(allocate_memory, "vkAllocateMemory", (device: usize, info: *const c_void, allocator: *const c_void, memory: *mut u64) -> i32);
vk_forward!(free_memory, "vkFreeMemory", (device: usize, memory: u64, allocator: *const c_void));
vk_forward!(map_memory, "vkMapMemory", (device: usize, memory: u64, offset: u64, size: u64, flags: u32, data: *mut *mut c_void) -> i32);
vk_forward!(unmap_memory, "vkUnmapMemory", (device: usize, memory: u64));
vk_forward!(get_buffer_memory_requirements, "vkGetBufferMemoryRequirements", (device: usize, buffer: u64, reqs: *mut c_void));
vk_forward!(bind_buffer_memory, "vkBindBufferMemory", (device: usize, buffer: u64, memory: u64, offset: u64) -> i32);
vk_forward!(reset_fences, "vkResetFences", (device: usize, count: u32, fences: *const u64) -> i32);
vk_forward!(get_fence_status, "vkGetFenceStatus", (device: usize, fence: u64) -> i32);
vk_forward!(wait_for_fences, "vkWaitForFences", (device: usize, count: u32, fences: *const u64, wait_all: u32, timeout: u64) -> i32);
vk_forward!(get_query_pool_results, "vkGetQueryPoolResults", (device: usize, pool: u64, first: u32, count: u32, size: usize, data: *mut c_void, stride: u64, flags: u32) -> i32);
vk_forward!(update_descriptor_sets, "vkUpdateDescriptorSets", (device: usize, write_count: u32, writes: *const c_void, copy_count: u32, copies: *const c_void));
vk_forward!(allocate_descriptor_sets, "vkAllocateDescriptorSets", (device: usize, info: *const c_void, sets: *mut u64) -> i32);
vk_forward!(reset_descriptor_pool, "vkResetDescriptorPool", (device: usize, pool: u64, flags: u32) -> i32);
vk_forward!(free_descriptor_sets, "vkFreeDescriptorSets", (device: usize, pool: u64, count: u32, sets: *const u64) -> i32);
vk_forward!(allocate_command_buffers, "vkAllocateCommandBuffers", (device: usize, info: *const c_void, buffers: *mut usize) -> i32);
vk_forward!(begin_command_buffer, "vkBeginCommandBuffer", (buffer: usize, info: *const c_void) -> i32);
vk_forward!(end_command_buffer, "vkEndCommandBuffer", (buffer: usize) -> i32);
vk_forward!(cmd_begin_query, "vkCmdBeginQuery", (buffer: usize, pool: u64, query: u32, flags: u32));
vk_forward!(cmd_end_query, "vkCmdEndQuery", (buffer: usize, pool: u64, query: u32));
vk_forward!(cmd_reset_query_pool, "vkCmdResetQueryPool", (buffer: usize, pool: u64, first: u32, count: u32));
vk_forward!(cmd_copy_query_pool_results, "vkCmdCopyQueryPoolResults", (buffer: usize, pool: u64, first: u32, count: u32, dst: u64, offset: u64, stride: u64, flags: u32));
vk_forward!(cmd_write_timestamp, "vkCmdWriteTimestamp", (buffer: usize, stage: u32, pool: u64, query: u32));
vk_forward!(cmd_pipeline_barrier, "vkCmdPipelineBarrier", (buffer: usize, src_stage: u32, dst_stage: u32, dep_flags: u32, mem_barriers: u32, p_mem_barriers: *const c_void, buf_barriers: u32, p_buf_barriers: *const c_void, img_barriers: u32, p_img_barriers: *const c_void));
vk_forward!(cmd_clear_attachments, "vkCmdClearAttachments", (buffer: usize, count: u32, attachments: *const c_void, rect_count: u32, rects: *const c_void));
vk_forward!(cmd_clear_color_image, "vkCmdClearColorImage", (buffer: usize, image: u64, layout: u32, color: *const c_void, range_count: u32, ranges: *const c_void));
vk_forward!(cmd_blit_image, "vkCmdBlitImage", (buffer: usize, src: u64, src_layout: u32, dst: u64, dst_layout: u32, region_count: u32, regions: *const c_void, filter: u32));
vk_forward!(cmd_copy_buffer, "vkCmdCopyBuffer", (buffer: usize, src: u64, dst: u64, region_count: u32, regions: *const c_void));
vk_forward!(cmd_copy_image, "vkCmdCopyImage", (buffer: usize, src: u64, src_layout: u32, dst: u64, dst_layout: u32, region_count: u32, regions: *const c_void));
vk_forward!(cmd_copy_buffer_to_image, "vkCmdCopyBufferToImage", (buffer: usize, src: u64, dst: u64, layout: u32, region_count: u32, regions: *const c_void));
vk_forward!(cmd_copy_image_to_buffer, "vkCmdCopyImageToBuffer", (buffer: usize, src: u64, layout: u32, dst: u64, region_count: u32, regions: *const c_void));

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enumerate_extensions() {
        let mut count = 0u32;
        let r = unsafe { enumerate_instance_extensions(std::ptr::null(), &mut count, std::ptr::null_mut()) };
        assert_eq!(r, 0);
        let mut exts = vec![Extension { name: [0; 256], version: 0 }; count as usize];
        let r = unsafe { enumerate_instance_extensions(std::ptr::null(), &mut count, exts.as_mut_ptr()) };
        assert_eq!(r, 0);
        let names: Vec<String> = exts.iter().map(|e| unsafe { CStr::from_ptr(e.name.as_ptr()) }.to_string_lossy().into_owned()).collect();
        println!("Available Vulkan extensions ({}): {:?}", names.len(), names);
        assert!(names.contains(&"VK_KHR_surface".to_string()));
        assert!(names.contains(&"VK_KHR_win32_surface".to_string()));
    }
}

