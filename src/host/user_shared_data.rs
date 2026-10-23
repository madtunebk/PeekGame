//! Emulation of Windows KUSER_SHARED_DATA at fixed user-mode address 0x7ffe0000.
//! In 32-bit and 64-bit Windows, this 4KB page is always mapped in every process
//! to share system version, ticks, processor features, and system paths without
//! kernel transitions. Many protectors (e.g. Blizzard / Arxan) read tick count
//! multiplier and timestamps directly from 0x7ffe0004 / 0x7ffe0000.

use std::{ffi::c_void, sync::OnceLock};

pub const SHARED_USER_DATA_ADDR: usize = 0x7ffe_0000;
pub const SHARED_USER_DATA_SIZE: usize = 0x1000;

unsafe extern "C" {
    fn mmap(
        address: *mut c_void,
        size: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        offset: i64,
    ) -> *mut c_void;
}

const PROT_READ: i32 = 1;
const PROT_WRITE: i32 = 2;
const MAP_PRIVATE_ANONYMOUS: i32 = 0x22;
const MAP_FIXED: i32 = 0x10;

pub fn initialize() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        unsafe {
            let ptr = mmap(
                SHARED_USER_DATA_ADDR as *mut c_void,
                SHARED_USER_DATA_SIZE,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE_ANONYMOUS | MAP_FIXED,
                -1,
                0,
            );
            if ptr as isize == -1 {
                eprintln!("PeekGame: warning: failed to map KUSER_SHARED_DATA at 0x7ffe0000");
                return;
            }

            let slice = std::slice::from_raw_parts_mut(ptr as *mut u8, SHARED_USER_DATA_SIZE);

            let put_u16 = |b: &mut [u8], at: usize, v: u16| {
                b[at..at + 2].copy_from_slice(&v.to_le_bytes());
            };
            let put_u32 = |b: &mut [u8], at: usize, v: u32| {
                b[at..at + 4].copy_from_slice(&v.to_le_bytes());
            };
            let put_u64 = |b: &mut [u8], at: usize, v: u64| {
                b[at..at + 8].copy_from_slice(&v.to_le_bytes());
            };

            // 0x000: TickCountLowDeprecated
            put_u32(slice, 0x000, 1000);
            // 0x004: TickCountMultiplier (1 << 24 = 0x01000000)
            put_u32(slice, 0x004, 1 << 24);

            // 0x008: InterruptTime (KSYSTEM_TIME)
            put_u32(slice, 0x008, 10000);
            put_u32(slice, 0x00c, 0);
            put_u32(slice, 0x010, 0);

            // 0x014: SystemTime (KSYSTEM_TIME)
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            let now_ft = crate::api::filesystem::files::filetime(
                now.as_secs() as i64,
                now.subsec_nanos() as i64,
            );
            put_u32(slice, 0x014, (now_ft & 0xffff_ffff) as u32);
            put_u32(slice, 0x018, (now_ft >> 32) as u32);
            put_u32(slice, 0x01c, (now_ft >> 32) as u32);

            // 0x020: TimeZoneBias
            put_u64(slice, 0x020, 0);

            // 0x02c: ImageNumberLow = 0x8664 (IMAGE_FILE_MACHINE_AMD64)
            put_u16(slice, 0x02c, 0x8664);
            // 0x02e: ImageNumberHigh = 0x8664
            put_u16(slice, 0x02e, 0x8664);

            // 0x030: NtSystemRoot (WCHAR[260]) = "C:\\Windows\0"
            let root = "C:\\Windows\0".encode_utf16().collect::<Vec<u16>>();
            for (i, &ch) in root.iter().enumerate() {
                put_u16(slice, 0x030 + i * 2, ch);
            }

            // 0x244: LargePageMinimum (2MB)
            put_u32(slice, 0x244, 0x200000);

            // 0x260: NtBuildNumber = 22631 (Win 11 23H2)
            put_u32(slice, 0x260, 22631);
            // 0x264: NtProductType = 1 (NtProductWinNt)
            put_u32(slice, 0x264, 1);
            // 0x268: ProductTypeIsValid = 1
            slice[0x268] = 1;

            // 0x26a: NativeProcessorArchitecture = 9 (PROCESSOR_ARCHITECTURE_AMD64)
            put_u16(slice, 0x26a, 9);

            // 0x26c: NtMajorVersion = 10
            put_u32(slice, 0x26c, 10);
            // 0x270: NtMinorVersion = 0
            put_u32(slice, 0x270, 0);

            // 0x274: ProcessorFeatures (BOOLEAN[64])
            for feature in [
                2,  // PF_COMPARE_EXCHANGE_DOUBLE
                3,  // PF_MMX_INSTRUCTIONS_AVAILABLE
                6,  // PF_XMMI_INSTRUCTIONS_AVAILABLE (SSE)
                8,  // PF_RDTSC_INSTRUCTION_AVAILABLE
                9,  // PF_PAE_ENABLED
                10, // PF_XMMI64_INSTRUCTIONS_AVAILABLE (SSE2)
                11, // PF_SSE_DAZ_MODE_AVAILABLE
                12, // PF_NX_ENABLED
                13, // PF_SSE3_INSTRUCTIONS_AVAILABLE
                14, // PF_COMPARE_EXCHANGE128
                15, // PF_COMPARE64_EXCHANGE128
                17, // PF_XSAVE_ENABLED
                20, // PF_SECOND_LEVEL_ADDRESS_TRANSLATION
                21, // PF_VIRT_FIRMWARE_ENABLED
                22, // PF_RDWRFSGSBASE_AVAILABLE
                23, // PF_FASTFAIL_AVAILABLE
                28, // PF_RDRAND_INSTRUCTION_AVAILABLE
                29, // PF_AVX_INSTRUCTIONS_AVAILABLE
                30, // PF_AVX2_INSTRUCTIONS_AVAILABLE
            ] {
                slice[0x274 + feature] = 1;
            }

            // 0x2d4: KdDebuggerEnabled = 0
            slice[0x2d4] = 0;

            // 0x2d8: ActiveConsoleId = 1
            put_u32(slice, 0x2d8, 1);

            // 0x300: SharedDataFlags = 0x2e (DbgErrorPortPresent, ElevationEnabled, etc.)
            put_u32(slice, 0x300, 0x2e);

            // 0x320: TickCount (KSYSTEM_TIME)
            put_u32(slice, 0x320, 1000);
            put_u32(slice, 0x324, 0);
            put_u32(slice, 0x328, 0);

            // 0x3c0: QpcFrequency = 10_000_000 (10 MHz constant in Win 10/11)
            put_u64(slice, 0x3c0, 10_000_000);
        }
    });
}
