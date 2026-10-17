//! Windows packed WAVEFORMATEX/EXTENSIBLE, independent of host ABI.
use super::*;
#[derive(Clone, Copy)]
pub(super) struct Format {
    pub rate: u32,
    pub channels: u16,
    pub align: u32,
    pub pulse: i32,
    pub silence: u8,
}
pub(super) unsafe fn parse(p: *const u8) -> Option<Format> {
    if p.is_null() {
        return None;
    }
    let u16at = |n| unsafe { p.add(n).cast::<u16>().read_unaligned() };
    let u32at = |n| unsafe { p.add(n).cast::<u32>().read_unaligned() };
    let mut tag = u16at(0);
    let channels = u16at(2);
    let rate = u32at(4);
    let align = u16at(12) as u32;
    let bits = u16at(14);
    if tag == 0xfffe {
        if u16at(16) < 22 {
            return None;
        }
        let guid = unsafe { std::slice::from_raw_parts(p.add(24), 16) };
        if guid[4..] != [0, 0, 0x10, 0, 0x80, 0, 0, 0xaa, 0, 0x38, 0x9b, 0x71] {
            return None;
        }
        tag = u32at(24).try_into().ok()?;
        if u16at(18) != bits {
            return None;
        } // reduced valid-bit containers not yet supported
        let mask = u32at(20);
        if mask != 0 && mask != if channels == 1 { 4 } else { 3 } {
            return None;
        }
    }
    if !(1..=2).contains(&channels)
        || !(8000..=192000).contains(&rate)
        || align != channels as u32 * bits as u32 / 8
        || u32at(8) != rate * align
    {
        return None;
    }
    let pulse = match (tag, bits) {
        (1, 8) => 0,
        (1, 16) => 3,
        (1, 24) => 9,
        (1, 32) => 7,
        (3, 32) => 5,
        _ => return None,
    };
    Some(Format {
        rate,
        channels,
        align,
        pulse,
        silence: if bits == 8 { 128 } else { 0 },
    })
}
pub(super) fn mix() -> *mut c_void {
    let p = crate::api::com::apartment::allocate(18).cast::<u8>();
    if p.is_null() {
        return p.cast();
    }
    let bytes: [u8; 18] = [
        3, 0, 2, 0, 0x80, 0xbb, 0, 0, 0, 0xdc, 5, 0, 8, 0, 32, 0, 0, 0,
    ];
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), p, 18) };
    p.cast()
}
