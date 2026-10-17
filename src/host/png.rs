//! Host-only libpng simplified API; no host structures cross the Windows boundary.
use std::ffi::c_void;
#[repr(C)]
struct Image {
    opaque: *mut c_void,
    version: u32,
    width: u32,
    height: u32,
    format: u32,
    flags: u32,
    entries: u32,
    status: u32,
    message: [u8; 64],
}
#[link(name = "png16")]
unsafe extern "C" {
    fn png_image_begin_read_from_memory(image: *mut Image, data: *const u8, size: usize) -> i32;
    fn png_image_finish_read(
        image: *mut Image,
        background: *const c_void,
        out: *mut u8,
        stride: i32,
        colormap: *mut c_void,
    ) -> i32;
    fn png_image_free(image: *mut Image);
}
pub(crate) fn decode(data: &[u8]) -> Result<(u32, u32, Vec<u32>), u32> {
    let mut image = Image {
        opaque: std::ptr::null_mut(),
        version: 1,
        width: 0,
        height: 0,
        format: 0,
        flags: 0,
        entries: 0,
        status: 0,
        message: [0; 64],
    };
    if unsafe { png_image_begin_read_from_memory(&mut image, data.as_ptr(), data.len()) } == 0 {
        unsafe {
            png_image_free(&mut image);
        }
        return Err(87);
    }
    let (w, h) = (image.width, image.height);
    if w == 0 || h == 0 || w > 4096 || h > 4096 {
        unsafe {
            png_image_free(&mut image);
        }
        return Err(50);
    }
    image.format = 3; // PNG_FORMAT_RGBA, 8-bit sRGB components.
    let mut rgba = vec![0u8; w as usize * h as usize * 4];
    let ok = unsafe {
        png_image_finish_read(
            &mut image,
            std::ptr::null(),
            rgba.as_mut_ptr(),
            0,
            std::ptr::null_mut(),
        )
    };
    unsafe {
        png_image_free(&mut image);
    }
    if ok == 0 {
        return Err(87);
    }
    Ok((
        w,
        h,
        rgba.chunks_exact(4)
            .map(|p| u32::from_be_bytes([p[3], p[0], p[1], p[2]]))
            .collect(),
    ))
}
