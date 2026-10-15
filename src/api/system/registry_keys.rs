//! The Windows registry (ported from peek's registry.inc).
//! - HKEY_CURRENT_USER is a real, persistent key tree, kept in memory and saved
//!   after every change to APPDATA/registry/user.reg (text: a "K path" line per
//!   key followed by its "V name type hex" lines). Programs keep settings there
//!   (Unity's PlayerPrefs). Volatile keys (REG_OPTION_VOLATILE) are not saved.
//! - HKEY_LOCAL_MACHINE and the other roots hold no keys: opening a subkey fails
//!   with ERROR_FILE_NOT_FOUND and writing with ERROR_ACCESS_DENIED, as for a
//!   standard user. Nothing is invented there.
//!
//! Key and value names match case-insensitively. Strings are stored as UTF-16 (the
//! W form); the A functions convert from and to UTF-8, the ANSI code page here.
use crate::api::{shell::folders, text};
use std::{
    collections::HashMap,
    ffi::{c_char, CStr},
    fmt::Write as _,
    io::Write as _,
    path::PathBuf,
    sync::{Mutex, MutexGuard},
};
const ERROR_FILE_NOT_FOUND: i32 = 2;
const ERROR_ACCESS_DENIED: i32 = 5;
const ERROR_INVALID_HANDLE: i32 = 6;
const ERROR_INVALID_PARAMETER: i32 = 87;
const ERROR_MORE_DATA: i32 = 234;
const ERROR_NO_MORE_ITEMS: i32 = 259;
const ERROR_NOACCESS: i32 = 998;
const ERROR_KEY_DELETED: i32 = 1018;
const ERROR_UNSUPPORTED_TYPE: i32 = 1630;

struct Value {
    name: String,
    kind: u32,
    data: Vec<u8>,
}
struct Key {
    parent: Option<usize>,
    name: String,
    children: Vec<usize>,
    values: Vec<Value>,
    deleted: bool,
    volatile: bool,
    written: u64,
}
struct Tree {
    keys: Vec<Key>,
    handles: HashMap<usize, (usize, bool)>,
    next_handle: usize,
}
const USER: usize = 0;
const MACHINE: usize = 1;
static TREE: Mutex<Option<Tree>> = Mutex::new(None);

fn now() -> u64 {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    (t.as_secs() + 11_644_473_600) * 10_000_000 + t.subsec_nanos() as u64 / 100
}
fn is_string(kind: u32) -> bool {
    matches!(kind, 1 | 2 | 7)
}
fn root(name: &str) -> Key {
    Key {
        parent: None,
        name: name.into(),
        children: Vec::new(),
        values: Vec::new(),
        deleted: false,
        volatile: false,
        written: now(),
    }
}
fn file() -> Option<PathBuf> {
    Some(folders::appdata()?.join("registry/user.reg"))
}
/// Names in the file: bytes up to space, '%', '\\' and DEL are %XX-encoded.
fn escape(out: &mut String, name: &str) {
    for &b in name.as_bytes() {
        if b <= 0x20 || b == b'%' || b == b'\\' || b == 0x7f {
            let _ = write!(out, "%{b:02X}");
        } else {
            out.push(b as char);
        }
    }
}
fn unescape(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&text[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
impl Tree {
    fn new() -> Tree {
        let mut tree = Tree {
            keys: vec![root("HKEY_CURRENT_USER"), root("HKEY_LOCAL_MACHINE")],
            handles: HashMap::new(),
            next_handle: 0x5245_0000,
        };
        tree.load();
        tree
    }
    fn child(&self, k: usize, name: &str) -> Option<usize> {
        self.keys[k]
            .children
            .iter()
            .copied()
            .find(|&c| !self.keys[c].deleted && self.keys[c].name.eq_ignore_ascii_case(name))
    }
    fn add_child(&mut self, k: usize, name: &str) -> usize {
        let index = self.keys.len();
        let mut key = root(name);
        key.parent = Some(k);
        self.keys.push(key);
        self.keys[k].children.push(index);
        index
    }
    /// Follows `path` ("A\B\C"; empty components skipped), creating missing keys
    /// with `create`; the bool tells whether the last one is new.
    fn walk(&mut self, mut k: usize, path: &str, create: bool) -> Option<(usize, bool)> {
        let mut created = false;
        for part in path.split('\\').filter(|p| !p.is_empty()) {
            if part.len() >= 256 {
                return None;
            }
            k = match self.child(k, part) {
                Some(c) => {
                    created = false;
                    c
                }
                None if create => {
                    created = true;
                    self.add_child(k, part)
                }
                None => return None,
            };
        }
        Some((k, created))
    }
    fn set(&mut self, k: usize, name: &str, kind: u32, data: &[u8]) {
        let key = &mut self.keys[k];
        match key
            .values
            .iter_mut()
            .find(|v| v.name.eq_ignore_ascii_case(name))
        {
            Some(v) => {
                v.kind = kind;
                v.data = data.to_vec();
            }
            None => key.values.push(Value {
                name: name.into(),
                kind,
                data: data.to_vec(),
            }),
        }
        key.written = now();
    }
    fn path_of(&self, k: usize, out: &mut String) {
        let Some(parent) = self.keys[k].parent else {
            return;
        };
        if self.keys[parent].parent.is_some() {
            self.path_of(parent, out);
            out.push('\\');
        }
        escape(out, &self.keys[k].name);
    }
    fn save_key(&self, k: usize, out: &mut String) {
        let key = &self.keys[k];
        if key.deleted || key.volatile {
            return;
        }
        out.push_str("K ");
        self.path_of(k, out);
        out.push('\n');
        for v in &key.values {
            out.push_str("V ");
            if v.name.is_empty() {
                out.push('-');
            } else {
                escape(out, &v.name);
            }
            let _ = write!(out, " {} ", v.kind);
            for b in &v.data {
                let _ = write!(out, "{b:02x}");
            }
            out.push('\n');
        }
        for &c in &key.children {
            self.save_key(c, out);
        }
    }
    /// Writes the whole HKCU tree to a temporary file renamed over user.reg.
    fn save(&self) {
        let Some(path) = file() else {
            eprintln!("registry: no APPDATA directory; HKCU changes are not saved");
            return;
        };
        let mut text = String::from("peek registry 1\n");
        self.save_key(USER, &mut text);
        let temporary = path.with_extension("reg.tmp");
        let result = (|| -> std::io::Result<()> {
            std::fs::create_dir_all(path.parent().unwrap())?;
            let mut f = std::fs::File::create(&temporary)?;
            f.write_all(text.as_bytes())?;
            f.sync_all()?;
            std::fs::rename(&temporary, &path)
        })();
        if let Err(e) = result {
            eprintln!("registry: cannot save {}: {e}", path.display());
        }
    }
    fn load(&mut self) {
        let Some(text) = file().and_then(|p| std::fs::read_to_string(p).ok()) else {
            return;
        };
        let mut current = None;
        for line in text.lines() {
            if let Some(path) = line.strip_prefix("K ") {
                // Components are escaped separately ('\\' in a name is %5C).
                let mut k = USER;
                for part in path.split('\\').filter(|p| !p.is_empty()) {
                    let name = unescape(part);
                    k = match self.child(k, &name) {
                        Some(c) => c,
                        None => self.add_child(k, &name),
                    };
                }
                current = Some(k);
            } else if let (Some(rest), Some(k)) = (line.strip_prefix("V "), current) {
                let mut fields = rest.splitn(3, ' ');
                let (Some(name), Some(kind), hex) =
                    (fields.next(), fields.next(), fields.next().unwrap_or(""))
                else {
                    continue;
                };
                let name = if name == "-" {
                    String::new()
                } else {
                    unescape(name)
                };
                let Ok(kind) = kind.parse::<u32>() else {
                    continue;
                };
                let data: Vec<u8> = (0..hex.len() / 2)
                    .filter_map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).ok())
                    .collect();
                self.set(k, &name, kind, &data);
            }
        }
    }
    /// The key a handle names, and whether it may be written.
    fn key(&self, handle: usize) -> Result<(usize, bool), i32> {
        if let Some(which) = predefined(handle) {
            return Ok(if which == 0x8000_0001 {
                (USER, true)
            } else {
                (MACHINE, false)
            });
        }
        match self.handles.get(&handle) {
            Some(&(k, _)) if self.keys[k].deleted => Err(ERROR_KEY_DELETED),
            Some(&(k, writable)) => Ok((k, writable)),
            None => Err(ERROR_INVALID_HANDLE),
        }
    }
    fn new_handle(&mut self, k: usize, writable: bool) -> usize {
        let handle = self.next_handle;
        self.next_handle += 16;
        self.handles.insert(handle, (k, writable));
        handle
    }
    fn mark_deleted(&mut self, k: usize) {
        self.keys[k].deleted = true;
        for c in self.keys[k].children.clone() {
            self.mark_deleted(c);
        }
    }
}
fn tree() -> MutexGuard<'static, Option<Tree>> {
    let mut guard = TREE.lock().unwrap();
    if guard.is_none() {
        *guard = Some(Tree::new());
    }
    guard
}
fn with<T>(f: impl FnOnce(&mut Tree) -> T) -> T {
    f(tree().as_mut().unwrap())
}
/// HKEY_CLASSES_ROOT 0x80000000 .. HKEY_CURRENT_CONFIG 0x80000005 (and
/// DYN_DATA 6), as 32-bit or sign-extended values.
fn predefined(handle: usize) -> Option<u32> {
    let high = handle >> 32;
    if high != 0 && high != 0xffff_ffff {
        return None;
    }
    let low = handle as u32;
    (0x8000_0000..=0x8000_0006).contains(&low).then_some(low)
}
unsafe fn wide(p: *const u16) -> Option<String> {
    unsafe { text::string(p) }
}
unsafe fn narrow(p: *const c_char) -> Option<String> {
    (!p.is_null()).then(|| unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned())
}

fn open(root: usize, path: Option<&str>, options: u32, out: *mut usize) -> i32 {
    if out.is_null() {
        return ERROR_INVALID_PARAMETER;
    }
    unsafe { *out = 0 };
    if options & !8 != 0 {
        return ERROR_INVALID_PARAMETER; // REG_OPTION_OPEN_LINK is accepted (no links exist)
    }
    with(|t| {
        let (base, writable) = match t.key(root) {
            Ok(k) => k,
            Err(e) => return e,
        };
        let Some((k, _)) = t.walk(base, path.unwrap_or(""), false) else {
            return ERROR_FILE_NOT_FOUND;
        };
        unsafe { *out = t.new_handle(k, writable) };
        0
    })
}
pub unsafe extern "win64" fn open_key_ex_w(
    root: usize,
    key: *const u16,
    options: u32,
    _access: u32,
    out: *mut usize,
) -> i32 {
    open(root, unsafe { wide(key) }.as_deref(), options, out)
}
pub unsafe extern "win64" fn open_key_ex_a(
    root: usize,
    key: *const c_char,
    options: u32,
    _access: u32,
    out: *mut usize,
) -> i32 {
    open(root, unsafe { narrow(key) }.as_deref(), options, out)
}
pub unsafe extern "win64" fn open_key_w(root: usize, key: *const u16, out: *mut usize) -> i32 {
    open(root, unsafe { wide(key) }.as_deref(), 0, out)
}
pub unsafe extern "win64" fn open_key_a(root: usize, key: *const c_char, out: *mut usize) -> i32 {
    open(root, unsafe { narrow(key) }.as_deref(), 0, out)
}
/// RegCreateKeyEx: REG_OPTION_NON_VOLATILE (0) or VOLATILE (1); *disposition
/// REG_CREATED_NEW_KEY (1) or REG_OPENED_EXISTING_KEY (2). Security attributes with
/// a descriptor are not supported.
unsafe fn create(
    root: usize,
    path: &str,
    options: u32,
    security: *const u8,
    out: *mut usize,
    disposition: *mut u32,
) -> i32 {
    if out.is_null() {
        return ERROR_INVALID_PARAMETER;
    }
    unsafe { *out = 0 };
    if options & !1 != 0 {
        return ERROR_INVALID_PARAMETER;
    }
    if !security.is_null() && unsafe { security.add(8).cast::<usize>().read_unaligned() } != 0 {
        eprintln!("RegCreateKeyEx: security descriptors are not supported");
        return 50;
    }
    with(|t| {
        let (base, writable) = match t.key(root) {
            Ok(k) => k,
            Err(e) => return e,
        };
        let existing = t.walk(base, path, false);
        if existing.is_none() && !writable {
            return ERROR_ACCESS_DENIED;
        }
        let Some((k, created)) = existing.or_else(|| t.walk(base, path, true)) else {
            return ERROR_INVALID_PARAMETER;
        };
        if created && options & 1 != 0 {
            t.keys[k].volatile = true;
        }
        if created && options & 1 == 0 {
            t.save();
        }
        unsafe {
            *out = t.new_handle(k, writable);
            if !disposition.is_null() {
                *disposition = if created { 1 } else { 2 };
            }
        }
        0
    })
}
pub unsafe extern "win64" fn create_key_ex_w(
    root: usize,
    key: *const u16,
    _reserved: u32,
    _class: *const u16,
    options: u32,
    _access: u32,
    security: *const u8,
    out: *mut usize,
    disposition: *mut u32,
) -> i32 {
    let path = unsafe { wide(key) }.unwrap_or_default();
    unsafe { create(root, &path, options, security, out, disposition) }
}
pub unsafe extern "win64" fn create_key_ex_a(
    root: usize,
    key: *const c_char,
    _reserved: u32,
    _class: *const c_char,
    options: u32,
    _access: u32,
    security: *const u8,
    out: *mut usize,
    disposition: *mut u32,
) -> i32 {
    let path = unsafe { narrow(key) }.unwrap_or_default();
    unsafe { create(root, &path, options, security, out, disposition) }
}
pub unsafe extern "win64" fn create_key_w(root: usize, key: *const u16, out: *mut usize) -> i32 {
    let path = unsafe { wide(key) }.unwrap_or_default();
    unsafe { create(root, &path, 0, std::ptr::null(), out, std::ptr::null_mut()) }
}
pub unsafe extern "win64" fn create_key_a(root: usize, key: *const c_char, out: *mut usize) -> i32 {
    let path = unsafe { narrow(key) }.unwrap_or_default();
    unsafe { create(root, &path, 0, std::ptr::null(), out, std::ptr::null_mut()) }
}
pub extern "win64" fn close_key(key: usize) -> i32 {
    if predefined(key).is_some() {
        return 0;
    }
    match with(|t| t.handles.remove(&key)) {
        Some(_) => 0,
        None => ERROR_INVALID_HANDLE,
    }
}
pub extern "win64" fn flush_key(key: usize) -> i32 {
    with(|t| match t.key(key) {
        Ok((_, writable)) => {
            if writable {
                t.save();
            }
            0
        }
        Err(e) => e,
    })
}

/// RegQueryValueEx: with data NULL only the size; ERROR_MORE_DATA (and the size)
/// when it does not fit. The A form returns strings converted to UTF-8.
unsafe fn query(
    key: usize,
    name: &str,
    reserved: *const u32,
    kind: *mut u32,
    data: *mut u8,
    size: *mut u32,
    wide_form: bool,
) -> i32 {
    if !reserved.is_null() {
        return ERROR_INVALID_PARAMETER;
    }
    if !data.is_null() && size.is_null() {
        return ERROR_NOACCESS;
    }
    let found = with(|t| {
        let (k, _) = t.key(key)?;
        t.keys[k]
            .values
            .iter()
            .find(|v| v.name.eq_ignore_ascii_case(name))
            .map(|v| (v.kind, v.data.clone()))
            .ok_or(ERROR_FILE_NOT_FOUND)
    });
    let (value_kind, mut bytes) = match found {
        Ok(v) => v,
        Err(e) => return e,
    };
    if !wide_form && is_string(value_kind) && bytes.len() >= 2 {
        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        bytes = String::from_utf16_lossy(&units).into_bytes();
    }
    unsafe {
        if !kind.is_null() {
            *kind = value_kind;
        }
        if !size.is_null() {
            let fits = (*size as usize) >= bytes.len();
            let length = bytes.len() as u32;
            if !data.is_null() {
                if !fits {
                    *size = length;
                    return ERROR_MORE_DATA;
                }
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), data, bytes.len());
            }
            *size = length;
        }
    }
    0
}
pub unsafe extern "win64" fn query_value_ex_w(
    key: usize,
    name: *const u16,
    reserved: *const u32,
    kind: *mut u32,
    data: *mut u8,
    size: *mut u32,
) -> i32 {
    let name = unsafe { wide(name) }.unwrap_or_default();
    unsafe { query(key, &name, reserved, kind, data, size, true) }
}
pub unsafe extern "win64" fn query_value_ex_a(
    key: usize,
    name: *const c_char,
    reserved: *const u32,
    kind: *mut u32,
    data: *mut u8,
    size: *mut u32,
) -> i32 {
    let name = unsafe { narrow(name) }.unwrap_or_default();
    unsafe { query(key, &name, reserved, kind, data, size, false) }
}
fn set_value(key: usize, name: &str, kind: u32, data: &[u8]) -> i32 {
    with(|t| {
        let (k, writable) = match t.key(key) {
            Ok(k) => k,
            Err(e) => return e,
        };
        if !writable {
            return ERROR_ACCESS_DENIED;
        }
        t.set(k, name, kind, data);
        if !t.keys[k].volatile {
            t.save();
        }
        0
    })
}
pub unsafe extern "win64" fn set_value_ex_w(
    key: usize,
    name: *const u16,
    _reserved: u32,
    kind: u32,
    data: *const u8,
    size: u32,
) -> i32 {
    if data.is_null() && size != 0 {
        return ERROR_NOACCESS;
    }
    let name = unsafe { wide(name) }.unwrap_or_default();
    let bytes = if size == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(data, size as usize) }
    };
    set_value(key, &name, kind, bytes)
}
/// RegSetValueExA: string data (UTF-8) is stored as UTF-16, as Windows stores it.
pub unsafe extern "win64" fn set_value_ex_a(
    key: usize,
    name: *const c_char,
    _reserved: u32,
    kind: u32,
    data: *const u8,
    size: u32,
) -> i32 {
    if data.is_null() && size != 0 {
        return ERROR_NOACCESS;
    }
    let name = unsafe { narrow(name) }.unwrap_or_default();
    let bytes = if size == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(data, size as usize) }
    };
    if !is_string(kind) || bytes.is_empty() {
        return set_value(key, &name, kind, bytes);
    }
    let wide: Vec<u8> = String::from_utf8_lossy(bytes)
        .encode_utf16()
        .flat_map(|u| u.to_le_bytes())
        .collect();
    set_value(key, &name, kind, &wide)
}
/// RegSetKeyValueW: RegSetValueEx on a subkey (created as needed).
pub unsafe extern "win64" fn set_key_value_w(
    key: usize,
    subkey: *const u16,
    name: *const u16,
    kind: u32,
    data: *const u8,
    size: u32,
) -> i32 {
    let mut target = key;
    let mut opened = 0usize;
    if !subkey.is_null() && unsafe { *subkey } != 0 {
        let rc = unsafe { create_key_w(key, subkey, &mut opened) };
        if rc != 0 {
            return rc;
        }
        target = opened;
    }
    let rc = unsafe { set_value_ex_w(target, name, 0, kind, data, size) };
    if opened != 0 {
        close_key(opened);
    }
    rc
}
fn delete_value(key: usize, name: &str) -> i32 {
    with(|t| {
        let (k, writable) = match t.key(key) {
            Ok(k) => k,
            Err(e) => return e,
        };
        if !writable {
            return ERROR_ACCESS_DENIED;
        }
        let values = &mut t.keys[k].values;
        let Some(i) = values
            .iter()
            .position(|v| v.name.eq_ignore_ascii_case(name))
        else {
            return ERROR_FILE_NOT_FOUND;
        };
        values.remove(i);
        t.keys[k].written = now();
        if !t.keys[k].volatile {
            t.save();
        }
        0
    })
}
pub unsafe extern "win64" fn delete_value_w(key: usize, name: *const u16) -> i32 {
    delete_value(key, &unsafe { wide(name) }.unwrap_or_default())
}
pub unsafe extern "win64" fn delete_value_a(key: usize, name: *const c_char) -> i32 {
    delete_value(key, &unsafe { narrow(name) }.unwrap_or_default())
}
/// RegDeleteKey fails with ERROR_ACCESS_DENIED while the key has subkeys; recursive
/// deletion (RegDeleteTree) removes them too. Open handles to a deleted key answer
/// ERROR_KEY_DELETED.
fn delete_key(key: usize, path: Option<&str>, recursive: bool) -> i32 {
    let Some(path) = path.filter(|p| !p.is_empty()) else {
        return ERROR_INVALID_PARAMETER;
    };
    with(|t| {
        let (base, writable) = match t.key(key) {
            Ok(k) => k,
            Err(e) => return e,
        };
        let Some((k, _)) = t.walk(base, path, false) else {
            return ERROR_FILE_NOT_FOUND;
        };
        if !writable {
            return ERROR_ACCESS_DENIED;
        }
        if !recursive && t.keys[k].children.iter().any(|&c| !t.keys[c].deleted) {
            return ERROR_ACCESS_DENIED;
        }
        t.mark_deleted(k);
        if let Some(parent) = t.keys[k].parent {
            t.keys[parent].written = now();
        }
        t.save();
        0
    })
}
pub unsafe extern "win64" fn delete_key_w(key: usize, path: *const u16) -> i32 {
    delete_key(key, unsafe { wide(path) }.as_deref(), false)
}
pub unsafe extern "win64" fn delete_key_a(key: usize, path: *const c_char) -> i32 {
    delete_key(key, unsafe { narrow(path) }.as_deref(), false)
}
pub unsafe extern "win64" fn delete_key_ex_w(
    key: usize,
    path: *const u16,
    _access: u32,
    _reserved: u32,
) -> i32 {
    delete_key(key, unsafe { wide(path) }.as_deref(), false)
}
/// RegDeleteTreeW (and SHDeleteKeyW): NULL deletes everything below the key, which
/// itself stays.
pub unsafe extern "win64" fn delete_tree_w(key: usize, path: *const u16) -> i32 {
    match unsafe { wide(path) }.filter(|p| !p.is_empty()) {
        Some(path) => delete_key(key, Some(&path), true),
        None => with(|t| {
            let (k, writable) = match t.key(key) {
                Ok(k) => k,
                Err(e) => return e,
            };
            if !writable {
                return ERROR_ACCESS_DENIED;
            }
            for c in t.keys[k].children.clone() {
                t.mark_deleted(c);
            }
            t.keys[k].values.clear();
            t.save();
            0
        }),
    }
}

/// Copies `name` as UTF-16 into out[*length] (characters, the NUL included on
/// input, without it on output). ERROR_MORE_DATA when it does not fit.
unsafe fn copy_name(name: &str, out: *mut u16, length: *mut u32) -> i32 {
    let units: Vec<u16> = name.encode_utf16().collect();
    if out.is_null() || length.is_null() || unsafe { *length } as usize <= units.len() {
        if !length.is_null() && out.is_null() {
            unsafe { *length = units.len() as u32 };
        }
        return ERROR_MORE_DATA;
    }
    unsafe {
        text::copy_out(&units, out, *length);
        *length = units.len() as u32;
    }
    0
}
pub unsafe extern "win64" fn enum_key_ex_w(
    key: usize,
    index: u32,
    name: *mut u16,
    name_length: *mut u32,
    reserved: *const u32,
    class: *mut u16,
    class_length: *mut u32,
    written: *mut u64,
) -> i32 {
    if !reserved.is_null() {
        return ERROR_INVALID_PARAMETER;
    }
    let found = with(|t| {
        let (k, _) = t.key(key)?;
        t.keys[k]
            .children
            .iter()
            .filter(|&&c| !t.keys[c].deleted)
            .nth(index as usize)
            .map(|&c| (t.keys[c].name.clone(), t.keys[c].written))
            .ok_or(ERROR_NO_MORE_ITEMS)
    });
    let (child, time) = match found {
        Ok(v) => v,
        Err(e) => return e,
    };
    let rc = unsafe { copy_name(&child, name, name_length) };
    if rc != 0 {
        return rc;
    }
    unsafe {
        if !class.is_null() && !class_length.is_null() && *class_length != 0 {
            *class = 0;
        }
        if !class_length.is_null() {
            *class_length = 0;
        }
        if !written.is_null() {
            *written = time;
        }
    }
    0
}
/// RegEnumKeyW: the old form, the name buffer's size in characters.
pub unsafe extern "win64" fn enum_key_w(
    key: usize,
    index: u32,
    name: *mut u16,
    capacity: u32,
) -> i32 {
    let mut length = capacity;
    unsafe {
        enum_key_ex_w(
            key,
            index,
            name,
            &mut length,
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    }
}
pub unsafe extern "win64" fn enum_value_w(
    key: usize,
    index: u32,
    name: *mut u16,
    name_length: *mut u32,
    reserved: *const u32,
    kind: *mut u32,
    data: *mut u8,
    data_length: *mut u32,
) -> i32 {
    if !reserved.is_null() {
        return ERROR_INVALID_PARAMETER;
    }
    let found = with(|t| {
        let (k, _) = t.key(key)?;
        t.keys[k]
            .values
            .get(index as usize)
            .map(|v| (v.name.clone(), v.kind, v.data.clone()))
            .ok_or(ERROR_NO_MORE_ITEMS)
    });
    let (value_name, value_kind, bytes) = match found {
        Ok(v) => v,
        Err(e) => return e,
    };
    let rc = unsafe { copy_name(&value_name, name, name_length) };
    if rc != 0 {
        return rc;
    }
    unsafe {
        if !kind.is_null() {
            *kind = value_kind;
        }
        if !data_length.is_null() {
            let fits = *data_length as usize >= bytes.len();
            if !data.is_null() {
                if !fits {
                    *data_length = bytes.len() as u32;
                    return ERROR_MORE_DATA;
                }
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), data, bytes.len());
            }
            *data_length = bytes.len() as u32;
        }
    }
    0
}
pub unsafe extern "win64" fn query_info_key_w(
    key: usize,
    class: *mut u16,
    class_length: *mut u32,
    reserved: *const u32,
    subkeys: *mut u32,
    max_subkey: *mut u32,
    max_class: *mut u32,
    values: *mut u32,
    max_value_name: *mut u32,
    max_value: *mut u32,
    security: *mut u32,
    written: *mut u64,
) -> i32 {
    if !reserved.is_null() {
        return ERROR_INVALID_PARAMETER;
    }
    let info = with(|t| {
        let (k, _) = t.key(key)?;
        let key = &t.keys[k];
        let live: Vec<usize> = key
            .children
            .iter()
            .copied()
            .filter(|&c| !t.keys[c].deleted)
            .collect();
        let longest_key = live
            .iter()
            .map(|&c| t.keys[c].name.encode_utf16().count())
            .max()
            .unwrap_or(0);
        let longest_name = key
            .values
            .iter()
            .map(|v| v.name.encode_utf16().count())
            .max()
            .unwrap_or(0);
        let largest = key.values.iter().map(|v| v.data.len()).max().unwrap_or(0);
        Ok::<_, i32>([
            live.len() as u32,
            longest_key as u32,
            key.values.len() as u32,
            longest_name as u32,
            largest as u32,
            0,
        ])
        .map(|v| (v, key.written))
    });
    let ([n_keys, longest_key, n_values, longest_name, largest, _], time) = match info {
        Ok(v) => v,
        Err(e) => return e,
    };
    unsafe {
        if !class.is_null() && !class_length.is_null() && *class_length != 0 {
            *class = 0;
        }
        for (out, v) in [
            (class_length, 0),
            (subkeys, n_keys),
            (max_subkey, longest_key),
            (max_class, 0),
            (values, n_values),
            (max_value_name, longest_name),
            (max_value, largest),
            (security, 0),
        ] {
            if !out.is_null() {
                *out = v;
            }
        }
        if !written.is_null() {
            *written = time;
        }
    }
    0
}
/// RegGetValueW: RRF_RT_* type filter, strings returned NUL-terminated,
/// RRF_ZEROONFAILURE. REG_EXPAND_SZ is expanded unless RRF_NOEXPAND: %VARIABLES%
/// come from the environment.
pub unsafe extern "win64" fn get_value_w(
    key: usize,
    subkey: *const u16,
    value: *const u16,
    flags: u32,
    kind: *mut u32,
    data: *mut u8,
    size: *mut u32,
) -> i32 {
    if flags & 0xffff == 0 || (!data.is_null() && size.is_null()) {
        return ERROR_INVALID_PARAMETER;
    }
    let mut opened = 0usize;
    let mut target = key;
    let result = (|| {
        if !subkey.is_null() && unsafe { *subkey } != 0 {
            let rc = unsafe { open_key_ex_w(key, subkey, 0, 1, &mut opened) };
            if rc != 0 {
                return rc;
            }
            target = opened;
        }
        let (mut value_kind, mut length) = (0u32, 0u32);
        let rc = unsafe {
            query_value_ex_w(
                target,
                value,
                std::ptr::null(),
                &mut value_kind,
                std::ptr::null_mut(),
                &mut length,
            )
        };
        if rc != 0 {
            return rc;
        }
        const BITS: [u32; 12] = [1, 2, 4, 8, 0x10, 0, 0, 0x20, 0, 0, 0, 0x40];
        if value_kind > 11 || flags & BITS[value_kind as usize] == 0 {
            return ERROR_UNSUPPORTED_TYPE;
        }
        let mut bytes = vec![0u8; length as usize];
        let mut got = length;
        let rc = unsafe {
            query_value_ex_w(
                target,
                value,
                std::ptr::null(),
                std::ptr::null_mut(),
                bytes.as_mut_ptr(),
                &mut got,
            )
        };
        if rc != 0 {
            return rc;
        }
        if is_string(value_kind) {
            let mut units: Vec<u16> = bytes
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            while units.last() == Some(&0) {
                units.pop();
            }
            if value_kind == 2 && flags & 0x1000_0000 == 0 {
                let expanded =
                    crate::api::system::environment::expand(&String::from_utf16_lossy(&units));
                units = expanded.encode_utf16().collect();
            }
            units.push(0);
            if value_kind == 7 {
                units.push(0);
            }
            bytes = units.iter().flat_map(|u| u.to_le_bytes()).collect();
        }
        unsafe {
            if !kind.is_null() {
                *kind = value_kind;
            }
            if data.is_null() {
                if !size.is_null() {
                    *size = bytes.len() as u32;
                }
                return 0;
            }
            if (*size as usize) < bytes.len() {
                *size = bytes.len() as u32;
                return ERROR_MORE_DATA;
            }
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), data, bytes.len());
            *size = bytes.len() as u32;
        }
        0
    })();
    if result != 0 && !data.is_null() && !size.is_null() && flags & 0x2000_0000 != 0 {
        unsafe { data.write_bytes(0, *size as usize) };
    }
    if opened != 0 {
        close_key(opened);
    }
    result
}
