//! Rectangular GDI regions, copied from the old implementation's supported scope.
use super::state;
use crate::api::system::error;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex, OnceLock,
    },
};
#[derive(Clone, Copy)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}
impl Rect {
    fn empty(self) -> bool {
        self.left >= self.right || self.top >= self.bottom
    }
}
static NEXT: AtomicUsize = AtomicUsize::new(0x6d00_0000);
static NEXT_BRUSH: AtomicUsize = AtomicUsize::new(0x6e00_0000);
static REGIONS: OnceLock<Mutex<HashMap<usize, Rect>>> = OnceLock::new();
static BRUSHES: OnceLock<Mutex<HashMap<usize, u32>>> = OnceLock::new();
static FONTS: OnceLock<Mutex<HashMap<usize, [u8; 60]>>> = OnceLock::new();
fn regions() -> &'static Mutex<HashMap<usize, Rect>> {
    REGIONS.get_or_init(|| Mutex::new(HashMap::new()))
}
fn brushes() -> &'static Mutex<HashMap<usize, u32>> {
    BRUSHES.get_or_init(|| Mutex::new(HashMap::new()))
}
fn fonts() -> &'static Mutex<HashMap<usize, [u8; 60]>> {
    FONTS.get_or_init(|| Mutex::new(HashMap::new()))
}
fn fail(code: u32) -> i32 {
    error::set(code);
    0
}
pub extern "win64" fn create_solid_brush(color: u32) -> usize {
    let handle = NEXT_BRUSH.fetch_add(16, Ordering::Relaxed);
    brushes().lock().unwrap().insert(handle, color);
    handle
}
pub extern "win64" fn create_pen(_style: i32, _width: i32, color: u32) -> usize {
    let handle = NEXT_BRUSH.fetch_add(16, Ordering::Relaxed);
    brushes().lock().unwrap().insert(handle, color);
    handle
}
fn default_logfont_a() -> [u8; 60] {
    let mut lf = [0u8; 60];
    unsafe {
        lf.as_mut_ptr().cast::<i32>().write_unaligned(-12); // lfHeight
        lf.as_mut_ptr().add(16).cast::<i32>().write_unaligned(400); // lfWeight
    }
    lf[28..36].copy_from_slice(b"Segoe UI");
    lf
}
pub unsafe extern "win64" fn create_font_indirect_a(logfont: *const u8) -> usize {
    let handle = NEXT_BRUSH.fetch_add(16, Ordering::Relaxed);
    let mut lf = default_logfont_a();
    if !logfont.is_null() {
        unsafe { std::ptr::copy_nonoverlapping(logfont, lf.as_mut_ptr(), 60) };
    }
    fonts().lock().unwrap().insert(handle, lf);
    handle
}
#[allow(clippy::too_many_arguments)]
pub unsafe extern "win64" fn create_font_a(
    height: i32,
    width: i32,
    escapement: i32,
    orientation: i32,
    weight: i32,
    italic: u32,
    underline: u32,
    strike_out: u32,
    char_set: u32,
    out_precision: u32,
    clip_precision: u32,
    quality: u32,
    pitch_and_family: u32,
    face_name: *const std::ffi::c_char,
) -> usize {
    let mut lf = [0u8; 60];
    unsafe {
        lf.as_mut_ptr().cast::<i32>().write_unaligned(height);
        lf.as_mut_ptr().add(4).cast::<i32>().write_unaligned(width);
        lf.as_mut_ptr().add(8).cast::<i32>().write_unaligned(escapement);
        lf.as_mut_ptr().add(12).cast::<i32>().write_unaligned(orientation);
        lf.as_mut_ptr().add(16).cast::<i32>().write_unaligned(weight);
        lf[20] = italic as u8;
        lf[21] = underline as u8;
        lf[22] = strike_out as u8;
        lf[23] = char_set as u8;
        lf[24] = out_precision as u8;
        lf[25] = clip_precision as u8;
        lf[26] = quality as u8;
        lf[27] = pitch_and_family as u8;
        if !face_name.is_null() {
            let bytes = std::ffi::CStr::from_ptr(face_name).to_bytes();
            let n = bytes.len().min(31);
            lf[28..28 + n].copy_from_slice(&bytes[..n]);
        }
        create_font_indirect_a(lf.as_ptr())
    }
}
pub unsafe extern "win64" fn create_font_indirect_w(logfont: *const u8) -> usize {
    let handle = NEXT_BRUSH.fetch_add(16, Ordering::Relaxed);
    let mut lf = default_logfont_a();
    if !logfont.is_null() {
        unsafe { std::ptr::copy_nonoverlapping(logfont, lf.as_mut_ptr(), 28) };
    }
    fonts().lock().unwrap().insert(handle, lf);
    handle
}
#[allow(clippy::too_many_arguments)]
pub unsafe extern "win64" fn create_font_w(
    height: i32,
    width: i32,
    escapement: i32,
    orientation: i32,
    weight: i32,
    italic: u32,
    underline: u32,
    strike_out: u32,
    char_set: u32,
    out_precision: u32,
    clip_precision: u32,
    quality: u32,
    pitch_and_family: u32,
    _face_name: *const u16,
) -> usize {
    unsafe {
        create_font_a(
            height,
            width,
            escapement,
            orientation,
            weight,
            italic,
            underline,
            strike_out,
            char_set,
            out_precision,
            clip_precision,
            quality,
            pitch_and_family,
            std::ptr::null(),
        )
    }
}
pub unsafe extern "win64" fn create_bitmap(
    width: i32,
    height: i32,
    _planes: u32,
    bpp: u32,
    _bits: *const u8,
) -> usize {
    if width <= 0 || height <= 0 {
        return 0;
    }
    let handle = NEXT_BRUSH.fetch_add(16, Ordering::Relaxed);
    brushes().lock().unwrap().insert(handle, bpp);
    handle
}
pub unsafe extern "win64" fn create_dib_section(
    _dc: usize,
    bmi: *const u8,
    _usage: u32,
    ppv_bits: *mut *mut u8,
    _section: usize,
    _offset: u32,
) -> usize {
    let (w, h, bpp) = if !bmi.is_null() {
        unsafe {
            let w = (bmi.add(4) as *const i32).read_unaligned().unsigned_abs().max(1);
            let h = (bmi.add(8) as *const i32).read_unaligned().unsigned_abs().max(1);
            let bpp = (bmi.add(14) as *const u16).read_unaligned().max(8) as u32;
            (w, h, bpp)
        }
    } else {
        (64, 64, 32)
    };
    let size = (w as usize)
        .saturating_mul(h as usize)
        .saturating_mul(bpp.div_ceil(8) as usize)
        .max(64);
    let mut buf = vec![0u8; size].into_boxed_slice();
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    if !ppv_bits.is_null() {
        unsafe { *ppv_bits = ptr };
    }
    let handle = NEXT_BRUSH.fetch_add(16, Ordering::Relaxed);
    brushes().lock().unwrap().insert(handle, bpp);
    handle
}
pub extern "win64" fn create_round_rect_rgn(
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    _w: i32,
    _h: i32,
) -> usize {
    create(left, top, right, bottom)
}
pub extern "win64" fn get_stock_object(index: i32) -> usize {
    if (0..=21).contains(&index) {
        0x000f_0000 + index as usize
    } else {
        0
    }
}
pub unsafe extern "win64" fn get_object_a(handle: usize, count: i32, out: *mut u8) -> i32 {
    if let Some(lf) = fonts().lock().unwrap().get(&handle).copied().or_else(|| {
        matches!(handle, 0x000f_000a..=0x000f_000e | 0x000f_0010 | 0x000f_0011)
            .then(default_logfont_a)
    }) {
        if out.is_null() {
            return 60;
        }
        let n = count.clamp(0, 60) as usize;
        unsafe { std::ptr::copy_nonoverlapping(lf.as_ptr(), out, n) };
        return n as i32;
    }
    if let Some(color) = brushes().lock().unwrap().get(&handle).copied().or_else(|| {
        (0x000e_0000..=0x000f_0015).contains(&handle).then_some(0)
    }) {
        if out.is_null() {
            return 16;
        }
        let mut lb = [0u8; 16];
        unsafe { lb.as_mut_ptr().add(4).cast::<u32>().write_unaligned(color) };
        let n = count.clamp(0, 16) as usize;
        unsafe { std::ptr::copy_nonoverlapping(lb.as_ptr(), out, n) };
        return n as i32;
    }
    0
}
pub unsafe extern "win64" fn get_object_w(handle: usize, count: i32, out: *mut u8) -> i32 {
    if let Some(lf) = fonts().lock().unwrap().get(&handle).copied().or_else(|| {
        matches!(handle, 0x000f_000a..=0x000f_000e | 0x000f_0010 | 0x000f_0011)
            .then(default_logfont_a)
    }) {
        if out.is_null() {
            return 92;
        }
        let mut lfw = [0u8; 92];
        lfw[..28].copy_from_slice(&lf[..28]);
        let n = count.clamp(0, 92) as usize;
        unsafe { std::ptr::copy_nonoverlapping(lfw.as_ptr(), out, n) };
        return n as i32;
    }
    unsafe { get_object_a(handle, count, out) }
}
pub extern "win64" fn select_object(_dc: usize, obj: usize) -> usize {
    if let Some(r) = regions().lock().unwrap().get(&obj).copied() {
        return if r.empty() { 1 } else { 2 };
    }
    if obj == 0 {
        return 0;
    }
    0x000f_0000
}
pub extern "win64" fn set_bk_mode(_dc: usize, _mode: i32) -> i32 {
    1
}
pub extern "win64" fn set_bk_color(_dc: usize, color: u32) -> u32 {
    color
}
pub extern "win64" fn set_text_color(_dc: usize, color: u32) -> u32 {
    color
}
pub extern "win64" fn set_text_align(_dc: usize, align: u32) -> u32 {
    align
}
pub unsafe extern "win64" fn move_to_ex(_dc: usize, _x: i32, _y: i32, pt: *mut u64) -> i32 {
    if !pt.is_null() {
        unsafe { *pt = 0 };
    }
    1
}
pub extern "win64" fn line_to(_dc: usize, _x: i32, _y: i32) -> i32 {
    1
}
pub extern "win64" fn rectangle(_dc: usize, _l: i32, _t: i32, _r: i32, _b: i32) -> i32 {
    1
}
pub extern "win64" fn round_rect(
    _dc: usize,
    _l: i32,
    _t: i32,
    _r: i32,
    _b: i32,
    _w: i32,
    _h: i32,
) -> i32 {
    1
}
pub extern "win64" fn polygon(_dc: usize, _pts: usize, _count: i32) -> i32 {
    1
}
pub extern "win64" fn text_out_w(_dc: usize, _x: i32, _y: i32, _str: *const u16, _c: i32) -> i32 {
    1
}
#[allow(clippy::too_many_arguments)]
pub extern "win64" fn ext_text_out_w(
    _dc: usize,
    _x: i32,
    _y: i32,
    _opts: u32,
    _rect: usize,
    _str: *const u16,
    _c: u32,
    _dx: usize,
) -> i32 {
    1
}
pub unsafe extern "win64" fn get_text_extent_point32_w(
    _dc: usize,
    _str: *const u16,
    c: i32,
    size: *mut i32,
) -> i32 {
    if !size.is_null() {
        unsafe {
            *size = c.max(0) * 8;
            *size.add(1) = 16;
        }
    }
    1
}
pub extern "win64" fn create(left: i32, top: i32, right: i32, bottom: i32) -> usize {
    let rect = Rect {
        left: left.min(right),
        top: top.min(bottom),
        right: left.max(right),
        bottom: top.max(bottom),
    };
    let handle = NEXT.fetch_add(16, Ordering::Relaxed);
    regions().lock().unwrap().insert(handle, rect);
    handle
}
pub extern "win64" fn delete(handle: usize) -> i32 {
    if regions().lock().unwrap().remove(&handle).is_some()
        || brushes().lock().unwrap().remove(&handle).is_some()
        || fonts().lock().unwrap().remove(&handle).is_some()
        || (0x000d_0000..=0x000f_0015).contains(&handle)
    {
        1
    } else {
        fail(6)
    }
}
pub extern "win64" fn combine(dest: usize, source1: usize, source2: usize, mode: i32) -> i32 {
    let mut map = regions().lock().unwrap();
    if !map.contains_key(&dest) {
        return fail(6);
    }
    let Some(mut rect) = map.get(&source1).copied() else {
        return fail(6);
    };
    if mode != 5 {
        let Some(b) = map.get(&source2).copied() else {
            return fail(6);
        };
        if mode != 1 {
            return fail(if (1..=5).contains(&mode) { 50 } else { 87 });
        }
        rect = Rect {
            left: rect.left.max(b.left),
            top: rect.top.max(b.top),
            right: rect.right.min(b.right),
            bottom: rect.bottom.min(b.bottom),
        };
    }
    if rect.empty() {
        rect = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
    }
    map.insert(dest, rect);
    if rect.empty() {
        1
    } else {
        2
    }
}
pub unsafe extern "win64" fn bounds(handle: usize, out: *mut i32) -> i32 {
    if out.is_null() {
        return fail(87);
    }
    let map = regions().lock().unwrap();
    let Some(rect) = map.get(&handle).copied() else {
        return fail(6);
    };
    let values = if rect.empty() {
        [0; 4]
    } else {
        [rect.left, rect.top, rect.right, rect.bottom]
    };
    unsafe {
        std::ptr::copy_nonoverlapping(values.as_ptr(), out, 4);
    }
    if rect.empty() {
        1
    } else {
        2
    }
}
/// Existing X11 windows are rectangular. Only an unrestrictive full-window
/// region can be applied without a shape backend; ownership transfers on success.
pub extern "win64" fn set_window(hwnd: usize, region: usize, _redraw: i32) -> i32 {
    let Some((width, height)) = state::windows().get(&hwnd).map(|w| (w.width, w.height)) else {
        return fail(1400);
    };
    if region == 0 {
        return 1;
    }
    let mut map = regions().lock().unwrap();
    let Some(rect) = map.get(&region).copied() else {
        return fail(6);
    };
    if rect.left > 0 || rect.top > 0 || rect.right < width || rect.bottom < height {
        return fail(50);
    }
    map.remove(&region);
    1
}
pub extern "win64" fn pt_in_region(handle: usize, x: i32, y: i32) -> i32 {
    let map = regions().lock().unwrap();
    let Some(rect) = map.get(&handle).copied() else {
        return 0;
    };
    if rect.empty() {
        return 0;
    }
    if x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom {
        1
    } else {
        0
    }
}
