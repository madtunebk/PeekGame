use std::arch::x86_64::{__cpuid, __cpuid_count};
fn bit(leaf: u32, sub: u32, register: usize, bit: u32) -> i32 {
    {
        if __cpuid(leaf & 0x80000000).eax < leaf {
            return 0;
        }
        let r = __cpuid_count(leaf, sub);
        (([r.eax, r.ebx, r.ecx, r.edx][register] >> bit) & 1) as i32
    }
}
pub extern "win64" fn present(feature: u32) -> i32 {
    match feature {
        0 | 1 | 22 | 23 => 0,
        2 => bit(1, 0, 3, 8),
        3 => bit(1, 0, 3, 23),
        6 => bit(1, 0, 3, 25),
        8 => bit(1, 0, 3, 4),
        9 => bit(1, 0, 3, 6),
        10 | 11 => bit(1, 0, 3, 26),
        12 => bit(0x80000001, 0, 3, 20),
        13 => bit(1, 0, 2, 0),
        14 => bit(1, 0, 2, 13),
        17 => bit(1, 0, 2, 26) & bit(1, 0, 2, 27),
        28 => bit(1, 0, 2, 30),
        32 => bit(0x80000001, 0, 3, 27),
        33 => bit(7, 0, 2, 22),
        36 => bit(1, 0, 2, 9),
        37 => bit(1, 0, 2, 19),
        38 => bit(1, 0, 2, 20),
        39 => i32::from(std::is_x86_feature_detected!("avx")),
        40 => i32::from(std::is_x86_feature_detected!("avx2")),
        41 => i32::from(std::is_x86_feature_detected!("avx512f")),
        42 => bit(7, 0, 1, 9),
        _ => 0,
    }
}
