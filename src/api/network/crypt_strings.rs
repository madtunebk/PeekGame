//! CRYPT32 Base64/PEM encoding with Windows character-count and buffer contracts.
use crate::api::system::error;
fn encode(input: &[u8], flags: u32) -> Result<Vec<u8>, u32> {
    if flags & !0xc000ffff != 0 {
        return Err(87);
    }
    let kind = flags & 0xffff;
    if !matches!(kind, 0 | 1) {
        return Err(87);
    }
    let eol: &[u8] = if flags & 0x40000000 != 0 {
        b""
    } else if flags & 0x80000000 != 0 {
        b"\n"
    } else {
        b"\r\n"
    };
    let chars = input.len().div_ceil(3).checked_mul(4).ok_or(8u32)?;
    let lines = chars.div_ceil(64);
    let size = chars
        .checked_add(lines.checked_mul(eol.len()).ok_or(8u32)?)
        .and_then(|n| n.checked_add(if kind == 0 { 52 + 2 * eol.len() } else { 0 }))
        .ok_or(8u32)?;
    if size >= u32::MAX as usize {
        return Err(8);
    }
    let mut out = Vec::new();
    out.try_reserve(size).map_err(|_| 8u32)?;
    if kind == 0 {
        out.extend_from_slice(b"-----BEGIN CERTIFICATE-----");
        out.extend_from_slice(eol);
    }
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut column = 0;
    for part in input.chunks(3) {
        let v = ((part[0] as u32) << 16)
            | ((part.get(1).copied().unwrap_or(0) as u32) << 8)
            | part.get(2).copied().unwrap_or(0) as u32;
        out.push(ALPHABET[((v >> 18) & 63) as usize]);
        out.push(ALPHABET[((v >> 12) & 63) as usize]);
        out.push(if part.len() > 1 {
            ALPHABET[((v >> 6) & 63) as usize]
        } else {
            b'='
        });
        out.push(if part.len() > 2 {
            ALPHABET[(v & 63) as usize]
        } else {
            b'='
        });
        column += 4;
        if column == 64 {
            out.extend_from_slice(eol);
            column = 0;
        }
    }
    if column != 0 {
        out.extend_from_slice(eol);
    }
    if kind == 0 {
        out.extend_from_slice(b"-----END CERTIFICATE-----");
        out.extend_from_slice(eol);
    }
    Ok(out)
}
unsafe fn convert<T: Copy + From<u8>>(
    input: *const u8,
    length: u32,
    flags: u32,
    out: *mut T,
    size: *mut u32,
) -> i32 {
    let fail = |code| {
        error::set(code);
        0
    };
    if size.is_null() || (input.is_null() && length != 0) {
        return fail(87);
    }
    let input = if length == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(input, length as usize) }
    };
    let bytes = match encode(input, flags) {
        Ok(bytes) => bytes,
        Err(code) => return fail(code),
    };
    let capacity = unsafe { *size };
    let needed = bytes.len() as u32 + 1;
    if out.is_null() {
        unsafe {
            *size = needed;
        }
        return 1;
    }
    if capacity < needed {
        unsafe {
            *size = needed;
        }
        return fail(234);
    }
    for (i, &byte) in bytes.iter().enumerate() {
        unsafe {
            *out.add(i) = T::from(byte);
        }
    }
    unsafe {
        *out.add(bytes.len()) = T::from(0);
        *size = bytes.len() as u32;
    }
    1
}
pub unsafe extern "win64" fn binary_to_string_a(
    input: *const u8,
    length: u32,
    flags: u32,
    out: *mut u8,
    size: *mut u32,
) -> i32 {
    unsafe { convert(input, length, flags, out, size) }
}
pub unsafe extern "win64" fn binary_to_string_w(
    input: *const u8,
    length: u32,
    flags: u32,
    out: *mut u16,
    size: *mut u32,
) -> i32 {
    unsafe { convert(input, length, flags, out, size) }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct DataBlob {
    pub cb_data: u32,
    pub pb_data: *mut u8,
}

pub unsafe extern "win64" fn crypt_protect_data(
    data_in: *const DataBlob,
    _sz_data_descr: *const u16,
    _optional_entropy: *const DataBlob,
    _reserved: usize,
    _prompt_struct: usize,
    _flags: u32,
    data_out: *mut DataBlob,
) -> i32 {
    if data_in.is_null() || data_out.is_null() {
        error::set(87);
        return 0;
    }
    let in_len = unsafe { (*data_in).cb_data as usize };
    let in_ptr = unsafe { (*data_in).pb_data };
    if in_len > 0 && in_ptr.is_null() {
        error::set(87);
        return 0;
    }

    let out_ptr = crate::api::memory::local::allocate(0, in_len);
    if in_len > 0 && out_ptr == 0 {
        error::set(8);
        return 0;
    }
    if in_len > 0 {
        unsafe {
            std::ptr::copy_nonoverlapping(in_ptr, out_ptr as *mut u8, in_len);
        }
    }
    unsafe {
        (*data_out).cb_data = in_len as u32;
        (*data_out).pb_data = out_ptr as *mut u8;
    }
    1
}

pub unsafe extern "win64" fn crypt_unprotect_data(
    data_in: *const DataBlob,
    ppsz_data_descr: *mut *mut u16,
    _optional_entropy: *const DataBlob,
    _reserved: usize,
    _prompt_struct: usize,
    _flags: u32,
    data_out: *mut DataBlob,
) -> i32 {
    if data_in.is_null() || data_out.is_null() {
        error::set(87);
        return 0;
    }
    let in_len = unsafe { (*data_in).cb_data as usize };
    let in_ptr = unsafe { (*data_in).pb_data };
    if in_len > 0 && in_ptr.is_null() {
        error::set(87);
        return 0;
    }

    let out_ptr = crate::api::memory::local::allocate(0, in_len);
    if in_len > 0 && out_ptr == 0 {
        error::set(8);
        return 0;
    }
    if in_len > 0 {
        unsafe {
            std::ptr::copy_nonoverlapping(in_ptr, out_ptr as *mut u8, in_len);
        }
    }
    if !ppsz_data_descr.is_null() {
        unsafe { *ppsz_data_descr = std::ptr::null_mut() };
    }
    unsafe {
        (*data_out).cb_data = in_len as u32;
        (*data_out).pb_data = out_ptr as *mut u8;
    }
    1
}

// CryptInstallOIDFunctionAddress: a program registers OID handler functions at
// runtime (Chromium/CEF does this during certificate setup). The entries are
// recorded faithfully in a process-global table; PeekGame's own CRYPT32
// encode/decode paths do not yet dispatch through installed handlers, so this is
// an honest registration, not a claim that custom OID processing is active.
struct InstalledOidFunctions {
    encoding_type: u32,
    func_name: String,
    // Each entry's pszOID is kept as a raw value: per the CRYPT32 convention it
    // may be a numeric OID id (high 16 bits zero), not a string pointer, so it is
    // never dereferenced. The second value is the handler address.
    entries: Vec<(usize, usize)>,
}
static OID_FUNCTIONS: std::sync::Mutex<Vec<InstalledOidFunctions>> =
    std::sync::Mutex::new(Vec::new());

#[repr(C)]
struct CryptOidFuncEntry {
    psz_oid: usize,
    pv_func_addr: usize,
}

pub unsafe extern "win64" fn install_oid_function_address(
    _h_module: usize,
    encoding_type: u32,
    psz_func_name: *const std::ffi::c_char,
    c_func_entry: u32,
    rg_func_entry: *const u8,
    _flags: u32,
) -> i32 {
    if psz_func_name.is_null() || (c_func_entry != 0 && rg_func_entry.is_null()) {
        error::set(87);
        return 0;
    }
    // pszFuncName is a real ANSI string (e.g. "CryptDllEncodeObject"); guard the
    // numeric-id form defensively so an odd caller cannot fault us.
    let func_name = if (psz_func_name as usize) >> 16 == 0 {
        format!("#{}", psz_func_name as usize)
    } else {
        unsafe { std::ffi::CStr::from_ptr(psz_func_name) }
            .to_string_lossy()
            .into_owned()
    };
    let table = rg_func_entry as *const CryptOidFuncEntry;
    let entries = (0..c_func_entry as usize)
        .map(|i| {
            let e = unsafe { &*table.add(i) };
            (e.psz_oid, e.pv_func_addr)
        })
        .collect();
    let record = InstalledOidFunctions {
        encoding_type,
        func_name,
        entries,
    };
    if std::env::var_os("PEEKGAME_TRACE_CRYPTO").is_some() {
        eprintln!(
            "[crypto] CryptInstallOIDFunctionAddress {} (encoding {:#x}) {} entr{}",
            record.func_name,
            record.encoding_type,
            record.entries.len(),
            if record.entries.len() == 1 { "y" } else { "ies" }
        );
    }
    OID_FUNCTIONS.lock().unwrap().push(record);
    1
}
