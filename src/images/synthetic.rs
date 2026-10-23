//! Synthesizes in-memory PE32+ images for builtin DLLs (e.g. kernel32.dll, ntdll.dll).
//!
//! Windows programs, anti-cheat modules (Warden, BattlEye, EAC), and packers frequently
//! inspect PEB->Ldr->InLoadOrderModuleList, traverse to kernel32.dll / ntdll.dll,
//! parse their export directories directly in memory, and invoke exports such as
//! GetProcAddress, VirtualAlloc, and GetModuleHandleExA.
//!
//! This module builds valid PE32+ DLL images containing real IMAGE_EXPORT_DIRECTORY
//! structures and callable x86-64 jump thunks pointing to PeekGame's implementations.

use crate::{images::{map::Mapping, Module}, pe, registry};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU32},
        Arc, Mutex,
    },
};

fn put_u16(buf: &mut [u8], offset: usize, val: u16) {
    buf[offset..offset + 2].copy_from_slice(&val.to_le_bytes());
}

fn put_u32(buf: &mut [u8], offset: usize, val: u32) {
    buf[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
}

fn put_u64(buf: &mut [u8], offset: usize, val: u64) {
    buf[offset..offset + 8].copy_from_slice(&val.to_le_bytes());
}

/// Generates PE32+ file bytes for a builtin DLL containing the specified exports.
pub fn build_builtin_pe(dll_name: &str, exports: &[(&str, usize)]) -> Result<Vec<u8>, String> {
    let mut sorted_exports: Vec<(&str, usize)> = exports.to_vec();
    sorted_exports.sort_by(|a, b| a.0.cmp(b.0));
    sorted_exports.dedup_by(|a, b| a.0 == b.0);

    let n = sorted_exports.len();
    if n == 0 {
        return Err(format!("cannot build builtin PE for {dll_name}: no exports"));
    }

    // .text section: 16 bytes per thunk
    let text_raw_size = n * 16;
    let text_size_aligned = text_raw_size.next_multiple_of(4096);
    let rdata_rva = (0x1000 + text_size_aligned) as u32;

    // Estimate .rdata size:
    // Header (40) + Functions (4*n) + Names (4*n) + Ordinals (2*n) + dll_name + export_names
    let names_total_len: usize = sorted_exports.iter().map(|(name, _)| name.len() + 1).sum();
    let rdata_raw_size = 40 + (10 * n) + dll_name.len() + 1 + names_total_len;
    let rdata_size_aligned = rdata_raw_size.next_multiple_of(4096);

    let total_image_size = (0x1000 + text_size_aligned + rdata_size_aligned) as u32;
    let mut bytes = vec![0u8; total_image_size as usize];

    // 1. DOS Header
    bytes[0..2].copy_from_slice(b"MZ");
    put_u32(&mut bytes, 0x3c, 0x80); // e_lfanew

    // 2. PE Header
    bytes[0x80..0x84].copy_from_slice(b"PE\0\0");

    // COFF File Header
    put_u16(&mut bytes, 0x84, 0x8664); // Machine AMD64
    put_u16(&mut bytes, 0x86, 2); // NumberOfSections: .text and .rdata
    put_u32(&mut bytes, 0x88, 0x60000000); // TimeDateStamp
    put_u16(&mut bytes, 0x94, 240); // SizeOfOptionalHeader = 0xf0
    put_u16(&mut bytes, 0x96, 0x2022); // Characteristics: EXECUTABLE_IMAGE | LARGE_ADDRESS_AWARE | DLL

    // Optional Header (PE32+)
    put_u16(&mut bytes, 0x98, 0x020b); // Magic PE32+
    bytes[0x9a] = 14; // MajorLinkerVersion
    bytes[0x9b] = 0; // MinorLinkerVersion
    put_u32(&mut bytes, 0x9c, text_size_aligned as u32); // SizeOfCode
    put_u32(&mut bytes, 0xa0, rdata_size_aligned as u32); // SizeOfInitializedData
    put_u32(&mut bytes, 0xa8, 0); // AddressOfEntryPoint
    put_u32(&mut bytes, 0xac, 0x1000); // BaseOfCode
    put_u64(&mut bytes, 0xb0, 0); // ImageBase (relocated on mapping)
    put_u32(&mut bytes, 0xb8, 0x1000); // SectionAlignment
    put_u32(&mut bytes, 0xbc, 0x1000); // FileAlignment
    put_u16(&mut bytes, 0xc0, 10); // MajorOperatingSystemVersion
    put_u16(&mut bytes, 0xc2, 0); // MinorOperatingSystemVersion
    put_u16(&mut bytes, 0xc4, 10); // MajorImageVersion
    put_u16(&mut bytes, 0xc6, 0); // MinorImageVersion
    put_u16(&mut bytes, 0xc8, 10); // MajorSubsystemVersion
    put_u16(&mut bytes, 0xca, 0); // MinorSubsystemVersion
    put_u32(&mut bytes, 0xd0, total_image_size); // SizeOfImage
    put_u32(&mut bytes, 0xd4, 0x1000); // SizeOfHeaders
    put_u16(&mut bytes, 0xdc, 3); // Subsystem: Windows CUI
    put_u16(&mut bytes, 0xde, 0x0160); // DllCharacteristics: DYNAMIC_BASE | NX_COMPAT | HIGH_ENTROPY_VA
    put_u64(&mut bytes, 0xe0, 0x100000); // SizeOfStackReserve
    put_u64(&mut bytes, 0xe8, 0x1000); // SizeOfStackCommit
    put_u64(&mut bytes, 0xf0, 0x100000); // SizeOfHeapReserve
    put_u64(&mut bytes, 0xf8, 0x1000); // SizeOfHeapCommit
    put_u32(&mut bytes, 0x104, 16); // NumberOfRvaAndSizes

    // Data Directory 0: Export Directory
    put_u32(&mut bytes, 0x108, rdata_rva);
    put_u32(&mut bytes, 0x10c, rdata_raw_size as u32);

    // Section Headers
    // Section 0: .text
    bytes[0x188..0x190].copy_from_slice(b".text\0\0\0");
    put_u32(&mut bytes, 0x190, text_raw_size as u32); // VirtualSize
    put_u32(&mut bytes, 0x194, 0x1000); // VirtualAddress
    put_u32(&mut bytes, 0x198, text_size_aligned as u32); // SizeOfRawData
    put_u32(&mut bytes, 0x19c, 0x1000); // PointerToRawData
    put_u32(&mut bytes, 0x1ac, 0x60000020); // Characteristics: CODE | EXECUTE | READ

    // Section 1: .rdata
    bytes[0x1b0..0x1b8].copy_from_slice(b".rdata\0\0");
    put_u32(&mut bytes, 0x1b8, rdata_raw_size as u32); // VirtualSize
    put_u32(&mut bytes, 0x1bc, rdata_rva); // VirtualAddress
    put_u32(&mut bytes, 0x1c0, rdata_size_aligned as u32); // SizeOfRawData
    put_u32(&mut bytes, 0x1c4, rdata_rva); // PointerToRawData
    put_u32(&mut bytes, 0x1d4, 0x40000040); // Characteristics: INITIALIZED_DATA | READ

    // 3. Populate .text with jump thunks:
    // ff 25 00 00 00 00 [8 bytes target address] cc cc
    for (i, (_, target_address)) in sorted_exports.iter().enumerate() {
        let thunk_offset = 0x1000 + i * 16;
        bytes[thunk_offset..thunk_offset + 6].copy_from_slice(&[0xff, 0x25, 0x00, 0x00, 0x00, 0x00]);
        put_u64(&mut bytes, thunk_offset + 6, *target_address as u64);
        bytes[thunk_offset + 14] = 0xcc;
        bytes[thunk_offset + 15] = 0xcc;
    }

    // 4. Populate .rdata with IMAGE_EXPORT_DIRECTORY and tables
    let rdata_offset = rdata_rva as usize;
    let functions_table_offset = rdata_offset + 40;
    let names_table_offset = functions_table_offset + n * 4;
    let ordinals_table_offset = names_table_offset + n * 4;
    let mut string_cursor = ordinals_table_offset + n * 2;

    // DLL name string
    let dll_name_rva = (string_cursor - rdata_offset) as u32 + rdata_rva;
    bytes[string_cursor..string_cursor + dll_name.len()].copy_from_slice(dll_name.as_bytes());
    string_cursor += dll_name.len();
    bytes[string_cursor] = 0;
    string_cursor += 1;

    // IMAGE_EXPORT_DIRECTORY
    put_u32(&mut bytes, rdata_offset + 4, 0x60000000); // TimeDateStamp
    put_u32(&mut bytes, rdata_offset + 12, dll_name_rva); // Name RVA
    put_u32(&mut bytes, rdata_offset + 16, 1); // Base = 1
    put_u32(&mut bytes, rdata_offset + 20, n as u32); // NumberOfFunctions
    put_u32(&mut bytes, rdata_offset + 24, n as u32); // NumberOfNames
    put_u32(&mut bytes, rdata_offset + 28, (functions_table_offset - rdata_offset) as u32 + rdata_rva);
    put_u32(&mut bytes, rdata_offset + 32, (names_table_offset - rdata_offset) as u32 + rdata_rva);
    put_u32(&mut bytes, rdata_offset + 36, (ordinals_table_offset - rdata_offset) as u32 + rdata_rva);

    for (i, (name, _)) in sorted_exports.iter().enumerate() {
        // Function RVA (points to thunk i in .text)
        put_u32(&mut bytes, functions_table_offset + i * 4, 0x1000 + (i * 16) as u32);

        // Ordinal bias (0-based index into function table)
        put_u16(&mut bytes, ordinals_table_offset + i * 2, i as u16);

        // Name RVA
        let name_rva = (string_cursor - rdata_offset) as u32 + rdata_rva;
        put_u32(&mut bytes, names_table_offset + i * 4, name_rva);

        bytes[string_cursor..string_cursor + name.len()].copy_from_slice(name.as_bytes());
        string_cursor += name.len();
        bytes[string_cursor] = 0;
        string_cursor += 1;
    }

    Ok(bytes)
}

/// Collects all implemented exports for `dll` from PeekGame's registry.
pub fn exports_for_dll(dll: &str) -> Vec<(&'static str, usize)> {
    let mut map = std::collections::BTreeMap::new();
    for entry in registry::entries() {
        if entry.dll.eq_ignore_ascii_case(dll) {
            if let Some(get_addr) = entry.address {
                map.insert(entry.name, get_addr() as usize);
            }
        }
    }
    map.into_iter().collect()
}

/// Instantiates, maps and protects a synthetic PE module for a builtin library.
pub fn create_builtin_module(dll_name: &str, exports: &[(&str, usize)]) -> Result<Arc<Module>, String> {
    let bytes = build_builtin_pe(dll_name, exports)?;
    let image = pe::parse(&bytes)?;
    let mapping = Mapping::new(&bytes, &image)?;
    let (base, size) = (mapping.base(), mapping.size());
    mapping.protect(&image)?;

    let path = PathBuf::from(format!(r"C:\Windows\System32\{dll_name}"));
    let module = Arc::new(Module {
        base,
        size,
        name: dll_name.to_owned(),
        path,
        image,
        _mapping: Some(mapping),
        refs: AtomicU32::new(1),
        pinned: AtomicBool::new(true),
        attached: AtomicBool::new(true),
        thread_calls: AtomicBool::new(false),
        tls_slot: None,
        tls_callbacks: Vec::new(),
        deps: Mutex::new(Vec::new()),
    });

    Ok(module)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_pe_parses_and_exports_match() {
        let dummy_addrs = [0x12345678usize, 0x87654321usize, 0xdeadbeefusize];
        let exports = [
            ("VirtualAlloc", dummy_addrs[0]),
            ("GetProcAddress", dummy_addrs[1]),
            ("GetModuleHandleExA", dummy_addrs[2]),
        ];

        let bytes = build_builtin_pe("kernel32.dll", &exports).expect("build_builtin_pe must succeed");
        let image = pe::parse(&bytes).expect("pe::parse must succeed on synthetic PE");

        assert_eq!(image.sections.len(), 2);
        assert_eq!(image.sections[0].name, ".text");
        assert_eq!(image.sections[1].name, ".rdata");

        let exp = image.exports.as_ref().expect("exports must be present");
        assert_eq!(exp.dll_name.as_deref(), Some("kernel32.dll"));
        assert_eq!(exp.entries.len(), 3);

        // Verify sorted order
        assert_eq!(exp.entries[0].name.as_deref(), Some("GetModuleHandleExA"));
        assert_eq!(exp.entries[1].name.as_deref(), Some("GetProcAddress"));
        assert_eq!(exp.entries[2].name.as_deref(), Some("VirtualAlloc"));
    }

    #[test]
    fn synthetic_module_mapping_and_thunks() {
        extern "win64" fn test_fn_add(a: u64, b: u64) -> u64 {
            a + b
        }

        let exports = [("TestAdd", test_fn_add as *const () as usize)];
        let module = create_builtin_module("testmod.dll", &exports).expect("create_builtin_module must succeed");

        assert!(module.base > 0);
        let exp = module.image.exports.as_ref().unwrap();
        let target = &exp.entries[0].target;
        if let pe::ExportTarget::Rva(rva) = target {
            let thunk_ptr = module.base + *rva as usize;
            let thunk_fn: extern "win64" fn(u64, u64) -> u64 = unsafe { std::mem::transmute(thunk_ptr) };
            assert_eq!(thunk_fn(40, 2), 42);
        } else {
            panic!("expected RVA target");
        }
    }
}
