//! Resource management APIs: FindResourceW, LoadResource, LockResource, SizeofResource.
use crate::{api::{system::error, text}, images};
use std::sync::Arc;

enum ResKey {
    Id(u16),
    Name(String),
}

fn parse_key(ptr: usize) -> Option<ResKey> {
    if ptr == 0 {
        return None;
    }
    if ptr <= 0xffff {
        Some(ResKey::Id(ptr as u16))
    } else {
        let s = unsafe { text::string(ptr as *const u16) }?;
        if let Some(rest) = s.strip_prefix('#') {
            if let Ok(id) = rest.parse::<u16>() {
                return Some(ResKey::Id(id));
            }
        }
        Some(ResKey::Name(s))
    }
}

fn get_module(h_module: usize) -> Option<Arc<images::Module>> {
    if h_module == 0 {
        images::exe()
    } else {
        images::by_handle(h_module).or_else(|| images::by_address(h_module))
    }
}

unsafe fn search_dir(
    root: *const u8,
    dir_offset: u32,
    key: &ResKey,
) -> Option<u32> {
    let dir = unsafe { root.add(dir_offset as usize) };
    let named_count = unsafe { std::ptr::read_unaligned(dir.add(12) as *const u16) } as usize;
    let id_count = unsafe { std::ptr::read_unaligned(dir.add(14) as *const u16) } as usize;
    let entries = unsafe { dir.add(16) };

    for i in 0..(named_count + id_count) {
        let entry = unsafe { entries.add(i * 8) };
        let name_or_id = unsafe { std::ptr::read_unaligned(entry as *const u32) };
        let offset = unsafe { std::ptr::read_unaligned(entry.add(4) as *const u32) };

        let matches = if name_or_id & 0x8000_0000 != 0 {
            // String name
            let str_ptr = unsafe { root.add((name_or_id & 0x7fff_ffff) as usize) };
            let len = unsafe { std::ptr::read_unaligned(str_ptr as *const u16) } as usize;
            let slice = unsafe { std::slice::from_raw_parts(str_ptr.add(2) as *const u16, len) };
            let entry_name = String::from_utf16_lossy(slice);
            match key {
                ResKey::Name(n) => entry_name.eq_ignore_ascii_case(n),
                _ => false,
            }
        } else {
            // Integer ID
            match key {
                ResKey::Id(id) => *id == (name_or_id as u16),
                _ => false,
            }
        };

        if matches {
            return Some(offset);
        }
    }
    None
}

/// FindResourceW(hModule, lpName, lpType)
pub unsafe extern "win64" fn find_resource_w(
    h_module: usize,
    name: usize,
    res_type: usize,
) -> usize {
    unsafe { find_resource_ex_w(h_module, res_type, name, 0) }
}

/// FindResourceExW(hModule, lpType, lpName, wLanguage)
pub unsafe extern "win64" fn find_resource_ex_w(
    h_module: usize,
    res_type: usize,
    name: usize,
    _language: u16,
) -> usize {
    let Some(module) = get_module(h_module) else {
        error::set(images::ERROR_MOD_NOT_FOUND);
        return 0;
    };
    let Some((rsrc_rva, rsrc_size)) = module.image.directory(2) else {
        error::set(1812); // ERROR_RESOURCE_DATA_NOT_FOUND
        return 0;
    };
    if rsrc_size < 16 {
        error::set(1812);
        return 0;
    }

    let root = (module.base + rsrc_rva as usize) as *const u8;
    let Some(type_key) = parse_key(res_type) else {
        error::set(87);
        return 0;
    };
    let Some(name_key) = parse_key(name) else {
        error::set(87);
        return 0;
    };

    // Level 1: Type
    let Some(type_offset) = (unsafe { search_dir(root, 0, &type_key) }) else {
        error::set(1813); // ERROR_RESOURCE_TYPE_NOT_FOUND
        return 0;
    };
    if type_offset & 0x8000_0000 == 0 {
        error::set(1812);
        return 0;
    }

    // Level 2: Name
    let Some(name_offset) = (unsafe { search_dir(root, type_offset & 0x7fff_ffff, &name_key) }) else {
        error::set(1814); // ERROR_RESOURCE_NAME_NOT_FOUND
        return 0;
    };
    if name_offset & 0x8000_0000 == 0 {
        error::set(1812);
        return 0;
    }

    // Level 3: Language (take the first available language entry)
    let lang_dir = unsafe { root.add((name_offset & 0x7fff_ffff) as usize) };
    let named_count = unsafe { std::ptr::read_unaligned(lang_dir.add(12) as *const u16) } as usize;
    let id_count = unsafe { std::ptr::read_unaligned(lang_dir.add(14) as *const u16) } as usize;
    if named_count + id_count == 0 {
        error::set(1815); // ERROR_RESOURCE_LANG_NOT_FOUND
        return 0;
    }

    let first_entry = unsafe { lang_dir.add(16) };
    let data_entry_offset = unsafe { std::ptr::read_unaligned(first_entry.add(4) as *const u32) };
    if data_entry_offset & 0x8000_0000 != 0 {
        error::set(1812);
        return 0;
    }

    // Returns pointer to the IMAGE_RESOURCE_DATA_ENTRY
    unsafe { root.add((data_entry_offset & 0x7fff_ffff) as usize) as usize }
}

/// FindResourceA
pub unsafe extern "win64" fn find_resource_a(
    h_module: usize,
    name: usize,
    res_type: usize,
) -> usize {
    let name_w = if name > 0xffff {
        let s = unsafe { std::ffi::CStr::from_ptr(name as *const std::ffi::c_char) };
        let wide = text::wide(&s.to_string_lossy());
        wide.as_ptr() as usize
    } else {
        name
    };
    let type_w = if res_type > 0xffff {
        let s = unsafe { std::ffi::CStr::from_ptr(res_type as *const std::ffi::c_char) };
        let wide = text::wide(&s.to_string_lossy());
        wide.as_ptr() as usize
    } else {
        res_type
    };
    unsafe { find_resource_w(h_module, name_w, type_w) }
}

/// LoadResource(hModule, hResInfo) -> HGLOBAL
pub unsafe extern "win64" fn load_resource(h_module: usize, res_info: usize) -> usize {
    if res_info == 0 {
        error::set(87);
        return 0;
    }
    let Some(module) = get_module(h_module) else {
        error::set(images::ERROR_MOD_NOT_FOUND);
        return 0;
    };
    let data_rva = unsafe { std::ptr::read_unaligned(res_info as *const u32) };
    module.base + data_rva as usize
}

/// LockResource(hResData) -> LPVOID
pub extern "win64" fn lock_resource(res_data: usize) -> usize {
    res_data
}

/// SizeofResource(hModule, hResInfo) -> DWORD
pub unsafe extern "win64" fn sizeof_resource(_h_module: usize, res_info: usize) -> u32 {
    if res_info == 0 {
        return 0;
    }
    unsafe { std::ptr::read_unaligned((res_info + 4) as *const u32) }
}

/// FreeResource(hResData) -> BOOL
pub extern "win64" fn free_resource(_res_data: usize) -> i32 {
    0
}
