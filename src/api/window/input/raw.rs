//! Raw input implementation and owned state.
use super::{fail, trace_input};
use crate::api::window::{
    queue,
    state::{self, connection},
    types::*,
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Mutex,
};

// ---- raw input --------------------------------------------------------------------
const MOUSE_DEVICE: usize = 0x0007_0001;
const KEYBOARD_DEVICE: usize = 0x0007_0002;
#[derive(Clone, Copy)]
struct Registration {
    page: u16,
    usage: u16,
    flags: u32,
    target: usize,
}
struct RawState {
    devices: Vec<Registration>,
    /// Recent events by handle (RAWINPUT bytes), for GetRawInputData.
    events: VecDeque<(usize, Vec<u8>)>,
    /// Events not yet read through GetRawInputBuffer, per thread.
    unread: HashMap<u32, VecDeque<usize>>,
    /// A caller has observed these handles through GetRawInputData.
    exposed: HashSet<usize>,
    next: usize,
    last: Option<(i32, i32)>,
}
static RAW: std::sync::LazyLock<Mutex<RawState>> = std::sync::LazyLock::new(|| {
    Mutex::new(RawState {
        devices: Vec::new(),
        events: VecDeque::new(),
        unread: HashMap::new(),
        exposed: HashSet::new(),
        next: 0x0008_0000,
        last: None,
    })
});
/// The window registered for a generic-desktop usage (2 mouse, 6 keyboard), or the
/// focus window when registered without a target.
fn raw_target(usage: u16, event_window: usize) -> Option<usize> {
    let raw = RAW.lock().unwrap();
    let r = raw
        .devices
        .iter()
        .find(|d| d.page == 1 && d.usage == usage && d.flags & 1 == 0)?;
    Some(if r.target != 0 {
        r.target
    } else {
        event_window
    })
}
fn deliver(target: usize, device: usize, kind: u32, body: &[u8]) {
    trace_input(format_args!(
        "raw deliver target={target:#x} device={device:#x} kind={kind} body={body:?}"
    ));
    let Some(thread) = queue::owner(target) else {
        return;
    };
    // RAWINPUTHEADER: dwType, dwSize, hDevice, wParam (RIM_INPUT 0).
    let mut bytes = Vec::with_capacity(24 + body.len());
    bytes.extend_from_slice(&kind.to_le_bytes());
    bytes.extend_from_slice(&((24 + body.len()) as u32).to_le_bytes());
    bytes.extend_from_slice(&device.to_le_bytes());
    bytes.extend_from_slice(&0usize.to_le_bytes());
    bytes.extend_from_slice(body);
    let handle = {
        let mut raw = RAW.lock().unwrap();
        raw.next += 4;
        let handle = raw.next;
        raw.events.push_back((handle, bytes));
        if raw.events.len() > 4096 {
            if let Some((old, _)) = raw.events.pop_front() {
                raw.exposed.remove(&old);
            }
        }
        let unread = raw.unread.entry(thread).or_default();
        unread.push_back(handle);
        if unread.len() > 4096 {
            unread.pop_front();
        }
        handle
    };
    let mut message = queue::message(target, WM_INPUT, 0, handle as isize);
    if let Ok(c) = connection() {
        let (x, y, _) = c.pointer();
        message.point = Point { x, y };
    }
    queue::post_to(thread, message);
}
/// Only an unread, unobserved RAWMOUSE packet with no button or wheel data may
/// absorb more relative motion. Its handle and size stay valid for Windows code.
fn is_mouse_motion(bytes: &[u8]) -> bool {
    bytes.len() >= 44
        && bytes[0..4] == [0; 4]
        && bytes[8..16] == MOUSE_DEVICE.to_le_bytes()
        && bytes[24..36].iter().all(|&b| b == 0)
}
pub(in crate::api::window) fn is_pending_mouse_motion(thread: u32, handle: usize) -> bool {
    let raw = RAW.lock().unwrap();
    raw.unread.get(&thread).is_some_and(|q| q.contains(&handle))
        && !raw.exposed.contains(&handle)
        && raw.events.iter().any(|(h, bytes)| *h == handle && is_mouse_motion(bytes))
}
pub(in crate::api::window) fn merge_raw_mouse(thread: u32, handle: usize, dx: i32, dy: i32) -> bool {
    let mut raw = RAW.lock().unwrap();
    if !raw.unread.get(&thread).is_some_and(|q| q.contains(&handle))
        || raw.exposed.contains(&handle)
    {
        return false;
    }
    let Some((_, bytes)) = raw.events.iter_mut().find(|(h, _)| *h == handle) else {
        return false;
    };
    if !is_mouse_motion(bytes) {
        return false;
    }
    let old_x = i32::from_le_bytes(bytes[36..40].try_into().unwrap());
    let old_y = i32::from_le_bytes(bytes[40..44].try_into().unwrap());
    let (Some(x), Some(y)) = (old_x.checked_add(dx), old_y.checked_add(dy)) else {
        return false;
    };
    bytes[36..40].copy_from_slice(&x.to_le_bytes());
    bytes[40..44].copy_from_slice(&y.to_le_bytes());
    true
}
/// RAWMOUSE: usFlags, (pad), usButtonFlags, usButtonData, ulRawButtons, lLastX,
/// lLastY, ulExtraInformation.
pub(in crate::api::window) fn raw_mouse(
    event_window: usize,
    buttons: u16,
    data: i16,
    x_root: Option<(i32, i32)>,
) {
    let Some(target) = raw_target(2, event_window) else {
        if let Some(p) = x_root {
            RAW.lock().unwrap().last = Some(p);
        }
        return;
    };
    let (dx, dy) = match x_root {
        Some((x, y)) => {
            let mut raw = RAW.lock().unwrap();
            let delta = raw.last.map_or((0, 0), |(lx, ly)| (x - lx, y - ly));
            raw.last = Some((x, y));
            delta
        }
        None => (0, 0),
    };
    if buttons == 0 && dx == 0 && dy == 0 {
        return;
    }
    if buttons == 0 {
        if let (Some((x, y)), Some(thread)) = (x_root, queue::owner(target)) {
            if queue::merge_pending_raw_mouse(thread, target, dx, dy, Point { x, y }) {
                return;
            }
        }
    }
    let mut body = [0u8; 24];
    body[4..6].copy_from_slice(&buttons.to_le_bytes());
    body[6..8].copy_from_slice(&data.to_le_bytes());
    body[12..16].copy_from_slice(&dx.to_le_bytes());
    body[16..20].copy_from_slice(&dy.to_le_bytes());
    deliver(target, MOUSE_DEVICE, 0, &body);
}
/// RAWKEYBOARD: MakeCode, Flags (RI_KEY_BREAK 1, RI_KEY_E0 2), Reserved, VKey,
/// Message, ExtraInformation.
pub(in crate::api::window) fn raw_keyboard(
    event_window: usize,
    scan: u16,
    vk: u8,
    down: bool,
    message: u32,
) {
    let Some(target) = raw_target(6, event_window) else {
        return;
    };
    let mut body = [0u8; 16];
    body[0..2].copy_from_slice(&(scan & 0xff).to_le_bytes());
    let flags: u16 = (!down) as u16 | if scan & 0xe000 != 0 { 2 } else { 0 };
    body[2..4].copy_from_slice(&flags.to_le_bytes());
    body[6..8].copy_from_slice(&(vk as u16).to_le_bytes());
    body[8..12].copy_from_slice(&message.to_le_bytes());
    deliver(target, KEYBOARD_DEVICE, 1, &body);
}
/// RAWINPUTDEVICE: usUsagePage, usUsage, dwFlags (RIDEV_REMOVE 1), hwndTarget.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RawDevice {
    page: u16,
    usage: u16,
    flags: u32,
    target: usize,
}
pub(crate) unsafe extern "win64" fn register_raw_input_devices(
    devices: *const RawDevice,
    count: u32,
    size: u32,
) -> i32 {
    if devices.is_null() || size as usize != std::mem::size_of::<RawDevice>() {
        return fail(87, 0);
    }
    let list = unsafe { std::slice::from_raw_parts(devices, count as usize) };
    let mut raw = RAW.lock().unwrap();
    for d in list {
        trace_input(format_args!(
            "register page={} usage={} flags={:#x} target={:#x}",
            d.page, d.usage, d.flags, d.target
        ));
        raw.devices
            .retain(|r| !(r.page == d.page && r.usage == d.usage));
        if d.flags & 1 == 0 {
            raw.devices.push(Registration {
                page: d.page,
                usage: d.usage,
                flags: d.flags,
                target: d.target,
            });
        }
    }
    1
}
pub(crate) unsafe extern "win64" fn get_registered_raw_input_devices(
    out: *mut RawDevice,
    count: *mut u32,
    size: u32,
) -> i32 {
    if count.is_null() || size as usize != std::mem::size_of::<RawDevice>() {
        return fail(87, -1);
    }
    let raw = RAW.lock().unwrap();
    let n = raw.devices.len() as u32;
    if out.is_null() || unsafe { *count } < n {
        unsafe { *count = n };
        return if out.is_null() { 0 } else { fail(122, -1) };
    }
    for (i, d) in raw.devices.iter().enumerate() {
        unsafe {
            out.add(i).write(RawDevice {
                page: d.page,
                usage: d.usage,
                flags: d.flags,
                target: d.target,
            })
        };
    }
    n as i32
}
/// GetRawInputData: RID_INPUT (0x10000003) the whole RAWINPUT, RID_HEADER
/// (0x10000005) its header.
pub(crate) unsafe extern "win64" fn get_raw_input_data(
    handle: usize,
    command: u32,
    data: *mut u8,
    size: *mut u32,
    header_size: u32,
) -> u32 {
    if size.is_null() || header_size != 24 {
        return fail(87, u32::MAX);
    }
    let mut raw = RAW.lock().unwrap();
    let Some((_, bytes)) = raw.events.iter().find(|(h, _)| *h == handle) else {
        return fail(6, u32::MAX);
    };
    trace_input(format_args!(
        "GetRawInputData handle={handle:#x} command={command:#x} data={data:p}"
    ));
    let n = match command {
        0x1000_0003 => bytes.len(),
        0x1000_0005 => 24,
        _ => return fail(87, u32::MAX),
    } as u32;
    if data.is_null() {
        unsafe { *size = n };
        return 0;
    }
    if unsafe { *size } < n {
        return fail(122, u32::MAX);
    }
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), data, n as usize) };
    raw.exposed.insert(handle);
    n
}
/// GetRawInputBuffer: this thread's unread RAWINPUTs, each 8-byte aligned; reading
/// them removes their WM_INPUT messages.
pub(crate) unsafe extern "win64" fn get_raw_input_buffer(
    data: *mut u8,
    size: *mut u32,
    header_size: u32,
) -> u32 {
    if size.is_null() || header_size != 24 {
        return fail(87, u32::MAX);
    }
    let me = state::current_thread();
    let mut raw = RAW.lock().unwrap();
    let pending: Vec<usize> = raw
        .unread
        .get(&me)
        .map(|q| q.iter().copied().collect())
        .unwrap_or_default();
    let sizes: Vec<(usize, Vec<u8>)> = pending
        .iter()
        .filter_map(|h| raw.events.iter().find(|(e, _)| e == h).cloned())
        .collect();
    if data.is_null() {
        unsafe { *size = sizes.first().map_or(0, |(_, b)| (b.len() as u32 + 7) & !7) };
        return 0;
    }
    if !sizes.is_empty() {
        trace_input(format_args!(
            "GetRawInputBuffer pending={} data={data:p}",
            sizes.len()
        ));
    }
    let capacity = unsafe { *size } as usize;
    let mut offset = 0;
    let mut count = 0;
    for (handle, bytes) in &sizes {
        let length = (bytes.len() + 7) & !7;
        if offset + length > capacity {
            break;
        }
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), data.add(offset), bytes.len());
            data.add(offset + bytes.len())
                .write_bytes(0, length - bytes.len());
        }
        offset += length;
        count += 1;
        if let Some(q) = raw.unread.get_mut(&me) {
            q.retain(|h| h != handle);
        }
    }
    drop(raw);
    queue::consume_raw(
        &sizes
            .iter()
            .take(count)
            .map(|(handle, _)| *handle)
            .collect::<Vec<_>>(),
    );
    if count == 0 && !sizes.is_empty() {
        unsafe { *size = (sizes[0].1.len() as u32 + 7) & !7 };
        return fail(122, u32::MAX);
    }
    count as u32
}
/// The WM_INPUT message was retrieved: its event is no longer unread.
pub(in crate::api::window) fn raw_read(handle: usize) {
    let me = state::current_thread();
    if let Some(q) = RAW.lock().unwrap().unread.get_mut(&me) {
        q.retain(|&h| h != handle);
    }
}
/// RAWINPUTDEVICELIST: hDevice, dwType (0 mouse, 1 keyboard).
pub(crate) unsafe extern "win64" fn get_raw_input_device_list(
    out: *mut u8,
    count: *mut u32,
    size: u32,
) -> u32 {
    trace_input(format_args!(
        "GetRawInputDeviceList data={out:p} size={size}"
    ));
    if count.is_null() || size != 16 {
        return fail(87, u32::MAX);
    }
    if out.is_null() {
        unsafe { *count = 2 };
        return 0;
    }
    if unsafe { *count } < 2 {
        unsafe { *count = 2 };
        return fail(122, u32::MAX);
    }
    for (i, (device, kind)) in [(MOUSE_DEVICE, 0u32), (KEYBOARD_DEVICE, 1)]
        .iter()
        .enumerate()
    {
        unsafe {
            out.add(16 * i).cast::<usize>().write_unaligned(*device);
            out.add(16 * i + 8).cast::<u32>().write_unaligned(*kind);
        }
    }
    2
}
/// RIDI_DEVICENAME (0x20000007), RIDI_DEVICEINFO (0x2000000b), RIDI_PREPARSEDDATA
/// (0x20000005, none for mouse and keyboard).
pub(crate) unsafe extern "win64" fn get_raw_input_device_info_w(
    device: usize,
    command: u32,
    data: *mut u8,
    size: *mut u32,
) -> u32 {
    trace_input(format_args!(
        "GetRawInputDeviceInfo device={device:#x} command={command:#x} data={data:p}"
    ));
    if size.is_null() || (device != MOUSE_DEVICE && device != KEYBOARD_DEVICE) {
        return fail(6, u32::MAX);
    }
    let mouse = device == MOUSE_DEVICE;
    let bytes: Vec<u8> = match command {
        0x2000_0007 => {
            let name = if mouse {
                r"\\?\HID#PEEKGAME_MOUSE#0#{378de44c-56ef-11d1-bc8c-00a0c91405dd}"
            } else {
                r"\\?\HID#PEEKGAME_KEYBOARD#0#{884b96c3-56ef-11d1-bc8c-00a0c91405dd}"
            };
            // The size is in characters for this command.
            let units: Vec<u16> = name.encode_utf16().chain([0]).collect();
            let n = units.len() as u32;
            if data.is_null() || unsafe { *size } < n {
                unsafe { *size = n };
                return if data.is_null() {
                    0
                } else {
                    fail(122, u32::MAX)
                };
            }
            unsafe {
                std::ptr::copy_nonoverlapping(units.as_ptr(), data.cast::<u16>(), units.len())
            };
            return n;
        }
        0x2000_000b => {
            let mut info = vec![0u8; 32];
            info[0..4].copy_from_slice(&32u32.to_le_bytes());
            info[4..8].copy_from_slice(&(if mouse { 0u32 } else { 1 }).to_le_bytes());
            let fields: [u32; 6] = if mouse {
                [0x100, 5, 0, 1, 0, 0]
            } else {
                [4, 0, 1, 12, 3, 101]
            };
            for (i, f) in fields.iter().enumerate() {
                info[8 + 4 * i..12 + 4 * i].copy_from_slice(&f.to_le_bytes());
            }
            info
        }
        0x2000_0005 => Vec::new(),
        _ => return fail(87, u32::MAX),
    };
    let n = bytes.len() as u32;
    if data.is_null() {
        unsafe { *size = n };
        return 0;
    }
    if unsafe { *size } < n {
        unsafe { *size = n };
        return fail(122, u32::MAX);
    }
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), data, bytes.len()) };
    n
}
pub(crate) unsafe extern "win64" fn get_raw_input_device_info_a(
    device: usize,
    command: u32,
    data: *mut u8,
    size: *mut u32,
) -> u32 {
    if command != 0x2000_0007 {
        return unsafe { get_raw_input_device_info_w(device, command, data, size) };
    }
    let mut wide = [0u16; 128];
    let mut n = 128u32;
    if unsafe { get_raw_input_device_info_w(device, command, wide.as_mut_ptr().cast(), &mut n) }
        == u32::MAX
    {
        return u32::MAX;
    }
    if data.is_null() || unsafe { *size } < n {
        unsafe { *size = n };
        return if data.is_null() {
            0
        } else {
            fail(122, u32::MAX)
        };
    }
    for (i, unit) in wide.iter().take(n as usize).enumerate() {
        unsafe { *data.add(i) = *unit as u8 };
    }
    n
}
pub(crate) extern "win64" fn define_raw_input_proc() -> isize {
    0
}

/// Remove registration when the HWND is destroyed; raw state stays owned here.
pub(super) fn forget_window(hwnd: usize) {
    RAW.lock().unwrap().devices.retain(|d| d.target != hwnd);
}
/// A program warp resets the raw delta anchor, as before the module split.
pub(super) fn warped(point: (i32, i32)) {
    RAW.lock().unwrap().last = Some(point);
}
