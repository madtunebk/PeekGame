//! In-place heapsort and binary search; comparators are Windows ABI callbacks.
use std::ffi::c_void;
pub type Compare = unsafe extern "win64" fn(*const c_void, *const c_void) -> i32;
pub unsafe extern "win64" fn qsort(
    base: *mut u8,
    count: usize,
    size: usize,
    compare: Option<Compare>,
) {
    if count < 2 {
        return;
    }
    let Some(compare) = compare else {
        unsafe {
            *super::stdio::errno() = 22;
        }
        return;
    };
    if base.is_null() || size == 0 || count.checked_mul(size).is_none() {
        unsafe {
            *super::stdio::errno() = 22;
        }
        return;
    }
    unsafe fn sift(base: *mut u8, size: usize, mut root: usize, end: usize, cmp: Compare) {
        while root < end / 2 {
            let mut child = root * 2 + 1;
            if child + 1 < end
                && unsafe {
                    cmp(
                        base.add(child * size).cast(),
                        base.add((child + 1) * size).cast(),
                    )
                } < 0
            {
                child += 1;
            }
            if unsafe { cmp(base.add(root * size).cast(), base.add(child * size).cast()) } >= 0 {
                break;
            }
            unsafe {
                std::ptr::swap_nonoverlapping(base.add(root * size), base.add(child * size), size);
            }
            root = child;
        }
    }
    for start in (0..count / 2).rev() {
        unsafe {
            sift(base, size, start, count, compare);
        }
    }
    for end in (1..count).rev() {
        unsafe {
            std::ptr::swap_nonoverlapping(base, base.add(end * size), size);
            sift(base, size, 0, end, compare);
        }
    }
}
pub unsafe extern "win64" fn bsearch(
    key: *const c_void,
    base: *const u8,
    count: usize,
    size: usize,
    compare: Option<Compare>,
) -> *mut c_void {
    if count == 0 {
        return std::ptr::null_mut();
    }
    let Some(compare) = compare else {
        unsafe {
            *super::stdio::errno() = 22;
        }
        return std::ptr::null_mut();
    };
    if key.is_null() || base.is_null() || size == 0 || count.checked_mul(size).is_none() {
        unsafe {
            *super::stdio::errno() = 22;
        }
        return std::ptr::null_mut();
    }
    let (mut low, mut high) = (0, count);
    while low < high {
        let mid = low + (high - low) / 2;
        let element = unsafe { base.add(mid * size) };
        match unsafe { compare(key, element.cast()) }.cmp(&0) {
            std::cmp::Ordering::Less => high = mid,
            std::cmp::Ordering::Greater => low = mid + 1,
            std::cmp::Ordering::Equal => return element.cast_mut().cast(),
        }
    }
    std::ptr::null_mut()
}
