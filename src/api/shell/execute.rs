//! ShellExecute open/explore through the host's registered URI/file applications.
use crate::{api::system::error, host::paths};
use std::ffi::{c_char, c_void, CString};
#[repr(C)]
struct GError {
    domain: u32,
    code: i32,
    message: *mut c_char,
}
#[link(name = "gio-2.0")]
unsafe extern "C" {
    fn g_app_info_launch_default_for_uri(
        uri: *const c_char,
        context: *mut c_void,
        error: *mut *mut GError,
    ) -> i32;
    fn g_io_error_quark() -> u32;
}
#[link(name = "glib-2.0")]
unsafe extern "C" {
    fn g_error_free(error: *mut GError);
}
fn failed(windows: u32, shell: usize) -> usize {
    error::set(windows);
    shell
}
fn uri(value: &str, directory: Option<&str>) -> Result<String, u32> {
    if let Some((scheme, _)) = value.split_once(':') {
        if scheme.len() > 1
            && scheme.bytes().enumerate().all(|(i, c)| {
                c.is_ascii_alphabetic() || (i > 0 && (c.is_ascii_digit() || b"+.-".contains(&c)))
            })
        {
            return Ok(value.to_owned());
        }
    }
    let mut path = paths::host(value)?;
    if !path.is_absolute() {
        let base = match directory {
            Some(p) => paths::full(p)?,
            None => std::env::current_dir().map_err(|e| paths::error(&e))?,
        };
        path = base.join(path);
    }
    let metadata = std::fs::metadata(&path).map_err(|e| paths::error(&e))?;
    if metadata.is_file()
        && path
            .extension()
            .is_some_and(|s| s.eq_ignore_ascii_case("exe") || s.eq_ignore_ascii_case("dll"))
    {
        return Err(193);
    }
    let mut result = String::from("file://");
    for c in path.as_os_str().as_encoded_bytes() {
        if c.is_ascii_alphanumeric() || b"/-._~".contains(c) {
            result.push(*c as char);
        } else {
            use std::fmt::Write;
            write!(result, "%{c:02X}").unwrap();
        }
    }
    Ok(result)
}
pub unsafe extern "win64" fn execute_w(
    _window: usize,
    operation: *const u16,
    file: *const u16,
    parameters: *const u16,
    directory: *const u16,
    _show: i32,
) -> usize {
    let verb = if operation.is_null() {
        "open".to_owned()
    } else {
        match unsafe { paths::wide(operation) } {
            Ok(v) => v,
            Err(e) => return failed(e, 0),
        }
    };
    if !verb.eq_ignore_ascii_case("open") && !verb.eq_ignore_ascii_case("explore") {
        return failed(50, 31);
    }
    if !parameters.is_null() {
        match unsafe { paths::wide(parameters) } {
            Ok(p) if p.is_empty() => {}
            Ok(_) => return failed(50, 31),
            Err(e) => return failed(e, 0),
        }
    }
    let file = match unsafe { paths::wide(file) } {
        Ok(v) if !v.is_empty() => v,
        _ => return failed(87, 0),
    };
    let directory = if directory.is_null() {
        None
    } else {
        match unsafe { paths::wide(directory) } {
            Ok(d) if d.is_empty() => None,
            Ok(d) => Some(d),
            Err(e) => return failed(e, 0),
        }
    };
    let uri = match uri(&file, directory.as_deref()) {
        Ok(uri) => uri,
        Err(e) => {
            return failed(
                e,
                match e {
                    2 | 3 | 5 => e as usize,
                    193 => 11,
                    _ => 0,
                },
            )
        }
    };
    let Ok(uri) = CString::new(uri) else {
        return failed(87, 0);
    };
    let mut native_error = std::ptr::null_mut();
    let success = unsafe {
        g_app_info_launch_default_for_uri(uri.as_ptr(), std::ptr::null_mut(), &mut native_error)
    };
    if success != 0 {
        return 33;
    } // Documented success sentinel, not a process/module handle.
    let windows = if native_error.is_null() {
        31
    } else {
        unsafe {
            let value = if (*native_error).domain == g_io_error_quark() {
                match (*native_error).code {
                    1 => 2,
                    14 => 5,
                    15 => 1155,
                    _ => 31,
                }
            } else {
                31
            };
            g_error_free(native_error);
            value
        }
    };
    failed(
        windows,
        match windows {
            2 | 5 => windows as usize,
            1155 => 31,
            _ => 0,
        },
    )
}

#[repr(C)]
pub struct ShellExecuteInfoW {
    pub cb_size: u32,
    pub f_mask: u32,
    pub hwnd: usize,
    pub lp_verb: *const u16,
    pub lp_file: *const u16,
    pub lp_parameters: *const u16,
    pub lp_directory: *const u16,
    pub n_show: i32,
    pub h_inst_app: usize,
    pub lp_id_list: *mut c_void,
    pub lp_class: *const u16,
    pub hkey_class: usize,
    pub dw_hot_key: u32,
    pub h_icon_or_monitor: usize,
    pub h_process: usize,
}

pub unsafe extern "win64" fn execute_ex_w(info: *mut ShellExecuteInfoW) -> i32 {
    if info.is_null() {
        error::set(87);
        return 0;
    }
    let p = unsafe { &mut *info };
    if p.cb_size < std::mem::size_of::<ShellExecuteInfoW>() as u32 {
        error::set(87);
        return 0;
    }
    let ret = unsafe {
        execute_w(
            p.hwnd,
            p.lp_verb,
            p.lp_file,
            p.lp_parameters,
            p.lp_directory,
            p.n_show,
        )
    };
    p.h_inst_app = ret;
    if ret > 32 {
        1
    } else {
        0
    }
}

pub unsafe extern "win64" fn execute_a(
    window: usize,
    operation: *const std::ffi::c_char,
    file: *const std::ffi::c_char,
    parameters: *const std::ffi::c_char,
    directory: *const std::ffi::c_char,
    show: i32,
) -> usize {
    let to_u16 = |p: *const std::ffi::c_char| -> Option<Vec<u16>> {
        if p.is_null() {
            None
        } else {
            let s = unsafe { std::ffi::CStr::from_ptr(p) }.to_str().ok()?;
            Some(s.encode_utf16().chain(std::iter::once(0)).collect())
        }
    };
    let op_w = to_u16(operation);
    let file_w = to_u16(file);
    let param_w = to_u16(parameters);
    let dir_w = to_u16(directory);

    let op_ptr = op_w.as_ref().map(|v| v.as_ptr()).unwrap_or(std::ptr::null());
    let file_ptr = file_w.as_ref().map(|v| v.as_ptr()).unwrap_or(std::ptr::null());
    let param_ptr = param_w.as_ref().map(|v| v.as_ptr()).unwrap_or(std::ptr::null());
    let dir_ptr = dir_w.as_ref().map(|v| v.as_ptr()).unwrap_or(std::ptr::null());

    let op_str = (!operation.is_null()).then(|| unsafe { std::ffi::CStr::from_ptr(operation) }.to_string_lossy());
    let file_str = (!file.is_null()).then(|| unsafe { std::ffi::CStr::from_ptr(file) }.to_string_lossy());
    let param_str = (!parameters.is_null()).then(|| unsafe { std::ffi::CStr::from_ptr(parameters) }.to_string_lossy());
    eprintln!("ShellExecuteA: op={op_str:?} file={file_str:?} param={param_str:?}");

    unsafe { execute_w(window, op_ptr, file_ptr, param_ptr, dir_ptr, show) }
}
