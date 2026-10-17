//! Resource PNG/32-bit DIB icons and native X11 window icon properties.
use super::state;
use crate::api::system::error;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, OnceLock,
    },
};
struct Icon {
    width: u32,
    height: u32,
    pixels: Vec<u32>,
}
static ICONS: OnceLock<Mutex<HashMap<usize, Arc<Icon>>>> = OnceLock::new();
static NEXT: AtomicUsize = AtomicUsize::new(0x6f00_0000);
fn icons() -> &'static Mutex<HashMap<usize, Arc<Icon>>> {
    ICONS.get_or_init(|| Mutex::new(HashMap::new()))
}
fn dib(data: &[u8]) -> Result<(u32, u32, Vec<u32>), u32> {
    if data.len() < 40 {
        return Err(87);
    }
    let u32_at = |n| u32::from_le_bytes(data[n..n + 4].try_into().unwrap());
    let u16_at = |n| u16::from_le_bytes(data[n..n + 2].try_into().unwrap());
    let header = u32_at(0) as usize;
    let w = u32_at(4) as i32;
    let doubled = u32_at(8) as i32;
    if header < 40 || header > data.len() || w <= 0 || doubled <= 0 || doubled % 2 != 0 {
        return Err(87);
    }
    if u16_at(12) != 1 || u16_at(14) != 32 || u32_at(16) != 0 {
        return Err(50);
    }
    let h = doubled / 2;
    if w > 4096 || h > 4096 {
        return Err(50);
    }
    let count = w as usize * h as usize;
    let end = header + count * 4;
    if end > data.len() {
        return Err(87);
    }
    let mut pixels = vec![0u32; count];
    for y in 0..h as usize {
        for x in 0..w as usize {
            let p = header + ((h as usize - 1 - y) * w as usize + x) * 4;
            pixels[y * w as usize + x] = u32_at(p);
        }
    }
    if pixels.iter().all(|p| p >> 24 == 0) {
        let stride = (w as usize).div_ceil(32) * 4;
        if end + stride * h as usize > data.len() {
            return Err(87);
        }
        for y in 0..h as usize {
            for x in 0..w as usize {
                if data[end + (h as usize - 1 - y) * stride + x / 8] & (0x80 >> (x % 8)) == 0 {
                    pixels[y * w as usize + x] |= 0xff000000;
                }
            }
        }
    }
    Ok((w as u32, h as u32, pixels))
}
pub unsafe extern "win64" fn create(
    bits: *const u8,
    size: u32,
    is_icon: i32,
    version: u32,
    cx: i32,
    cy: i32,
    flags: u32,
) -> usize {
    let failed = |code| {
        error::set(code);
        0
    };
    if bits.is_null() || size < 8 || version != 0x30000 || cx < 0 || cy < 0 {
        return failed(87);
    }
    if is_icon == 0 || flags & !0x40 != 0 {
        return failed(50);
    }
    let data = unsafe { std::slice::from_raw_parts(bits, size as usize) };
    let decoded = if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        crate::host::png::decode(data)
    } else {
        dib(data)
    };
    let (w, h, mut pixels) = match decoded {
        Ok(decoded) => decoded,
        Err(code) => return failed(code),
    };
    let width = if cx != 0 {
        cx as u32
    } else if flags & 0x40 != 0 {
        32
    } else {
        w
    };
    let height = if cy != 0 {
        cy as u32
    } else if flags & 0x40 != 0 {
        32
    } else {
        h
    };
    if width > 4096 || height > 4096 {
        return failed(50);
    }
    if width != w || height != h {
        pixels = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .map(|(x, y)| {
                pixels[((y as u64 * h as u64 / height as u64) as usize) * w as usize
                    + (x as u64 * w as u64 / width as u64) as usize]
            })
            .collect();
    }
    let handle = NEXT.fetch_add(16, Ordering::Relaxed);
    icons().lock().unwrap().insert(
        handle,
        Arc::new(Icon {
            width,
            height,
            pixels,
        }),
    );
    handle
}
pub unsafe extern "win64" fn create_icon(
    _instance: usize,
    width: i32,
    height: i32,
    planes: u8,
    bits_pixel: u8,
    and_bits: *const u8,
    xor_bits: *const u8,
) -> usize {
    if width <= 0 || height <= 0 || planes != 1 || (bits_pixel != 1 && bits_pixel != 32) || and_bits.is_null() || xor_bits.is_null() {
        error::set(87);
        return 0;
    }
    if width > 4096 || height > 4096 {
        error::set(50);
        return 0;
    }
    let (w, h) = (width as u32, height as u32);
    let count = w as usize * h as usize;
    let mut pixels = vec![0u32; count];
    if bits_pixel == 32 {
        unsafe {
            std::ptr::copy_nonoverlapping(xor_bits as *const u32, pixels.as_mut_ptr(), count);
        }
    } else {
        let stride = (w as usize).div_ceil(16) * 2;
        for y in 0..h as usize {
            for x in 0..w as usize {
                let byte_idx = y * stride + x / 8;
                let bit_mask = 0x80 >> (x % 8);
                let and_b = unsafe { *and_bits.add(byte_idx) } & bit_mask;
                let xor_b = unsafe { *xor_bits.add(byte_idx) } & bit_mask;
                pixels[y * w as usize + x] = if and_b != 0 {
                    0x00000000
                } else if xor_b != 0 {
                    0xffffffff
                } else {
                    0xff000000
                };
            }
        }
    }
    let handle = NEXT.fetch_add(16, Ordering::Relaxed);
    icons().lock().unwrap().insert(handle, Arc::new(Icon { width: w, height: h, pixels }));
    handle
}
pub extern "win64" fn destroy(handle: usize) -> i32 {
    if icons().lock().unwrap().remove(&handle).is_some() {
        1
    } else {
        error::set(1414);
        0
    }
}
pub(super) fn set(hwnd: usize, which: usize, handle: usize) -> isize {
    if which > 1 {
        error::set(87);
        return 0;
    }
    let icon = if handle == 0 {
        None
    } else {
        let Some(icon) = icons().lock().unwrap().get(&handle).cloned() else {
            error::set(1414);
            return 0;
        };
        Some(icon)
    };
    let (native, old) = match state::with_window(hwnd, |w| {
        let old = w.icons[which];
        w.icons[which] = handle;
        (w.native, old)
    }) {
        Ok(value) => value,
        Err(code) => {
            error::set(code);
            return 0;
        }
    };
    if native != 0 && which == 1 {
        if let Ok(c) = state::connection() {
            if let Some(icon) = icon {
                c.set_icon(native, icon.width, icon.height, &icon.pixels);
            } else {
                c.set_icon(native, 0, 0, &[]);
            }
        }
    }
    old as isize
}
