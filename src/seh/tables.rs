//! x64 function tables, the data Windows unwinds with (ported from peek's
//! function_tables.inc): each PE image's exception directory (.pdata, sorted
//! RUNTIME_FUNCTIONs) and the tables JIT compilers register at run time
//! (RtlAddFunctionTable, RtlAddGrowableFunctionTable, RtlInstallFunctionTableCallback).
use crate::images;
use std::sync::Mutex;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RuntimeFunction {
    pub begin: u32,
    pub end: u32,
    pub unwind: u32,
}
type Callback = extern "win64" fn(u64, usize) -> *mut RuntimeFunction;
enum Entries {
    /// Caller-owned sorted array: (pointer, count, maximum).
    Array(usize, u32, u32),
    /// RtlInstallFunctionTableCallback: asked for each pc in the range.
    Callback(Callback, usize),
}
struct Table {
    /// The identifier the program deletes it by (array address, table object or
    /// callback identifier).
    id: usize,
    entries: Entries,
    base: u64,
    begin: u64,
    end: u64,
    growable: bool,
}
static TABLES: Mutex<Vec<Table>> = Mutex::new(Vec::new());

/// Binary search: the entry containing `rva`.
unsafe fn search(
    entries: *const RuntimeFunction,
    count: u32,
    rva: u32,
) -> Option<*mut RuntimeFunction> {
    let (mut lo, mut hi) = (0u32, count);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let e = unsafe { &*entries.add(mid as usize) };
        if rva < e.begin {
            hi = mid;
        } else if rva >= e.end {
            lo = mid + 1;
        } else {
            return Some(unsafe { entries.add(mid as usize) } as *mut _);
        }
    }
    None
}
/// RtlLookupFunctionEntry: registered tables first, then the image's .pdata. A
/// leaf function (no entry) gives NULL.
pub unsafe extern "win64" fn lookup(
    pc: u64,
    base: *mut u64,
    _history: usize,
) -> *mut RuntimeFunction {
    if !base.is_null() {
        unsafe { *base = 0 };
    }
    let found = {
        let tables = TABLES.lock().unwrap();
        tables
            .iter()
            .filter(|t| pc >= t.begin && pc < t.end)
            .find_map(|t| {
                let entry = match t.entries {
                    Entries::Array(p, count, _) => {
                        let rva = u32::try_from(pc - t.base).ok()?;
                        unsafe { search(p as *const RuntimeFunction, count, rva) }?
                    }
                    Entries::Callback(callback, context) => {
                        let e = callback(pc, context);
                        (!e.is_null()).then_some(e)?
                    }
                };
                Some((entry, t.base))
            })
    };
    if let Some((entry, table_base)) = found {
        if !base.is_null() {
            unsafe { *base = table_base };
        }
        return entry;
    }
    let Some(module) = images::by_address(pc as usize) else {
        return std::ptr::null_mut();
    };
    let Some((rva, size)) = module.image.directory(3) else {
        return std::ptr::null_mut();
    };
    let count = size / size_of::<RuntimeFunction>() as u32;
    let entries = (module.base + rva as usize) as *const RuntimeFunction;
    match unsafe { search(entries, count, (pc as usize - module.base) as u32) } {
        Some(e) => {
            if !base.is_null() {
                unsafe { *base = module.base as u64 };
            }
            e
        }
        None => std::ptr::null_mut(),
    }
}
/// RtlAddFunctionTable: `count` entries relative to `base`; the covered range is
/// that of the entries.
pub unsafe extern "win64" fn add(entries: *mut RuntimeFunction, count: u32, base: u64) -> u8 {
    if entries.is_null() || count == 0 {
        return 0;
    }
    let list = unsafe { std::slice::from_raw_parts(entries, count as usize) };
    let lo = list.iter().map(|e| e.begin).min().unwrap_or(0);
    let hi = list.iter().map(|e| e.end).max().unwrap_or(0);
    TABLES.lock().unwrap().push(Table {
        id: entries as usize,
        entries: Entries::Array(entries as usize, count, count),
        base,
        begin: base + lo as u64,
        end: base + hi as u64,
        growable: false,
    });
    1
}
/// RtlDeleteFunctionTable: an RtlAddFunctionTable array or an
/// RtlInstallFunctionTableCallback identifier.
pub extern "win64" fn delete(id: usize) -> u8 {
    let mut tables = TABLES.lock().unwrap();
    match tables.iter().position(|t| t.id == id && !t.growable) {
        Some(i) => {
            tables.remove(i);
            1
        }
        None => 0,
    }
}
/// RtlInstallFunctionTableCallback: `id` has its low two bits set (3); `callback`
/// supplies the entry for any pc in [base, base + length).
pub extern "win64" fn install_callback(
    id: u64,
    base: u64,
    length: u32,
    callback: Option<Callback>,
    context: usize,
    _dll: usize,
) -> u8 {
    let Some(callback) = callback else {
        return 0;
    };
    if id & 3 != 3 {
        return 0;
    }
    TABLES.lock().unwrap().push(Table {
        id: id as usize,
        entries: Entries::Callback(callback, context),
        base,
        begin: base,
        end: base + length as u64,
        growable: false,
    });
    1
}
static NEXT_GROWABLE: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0x6a00_0000);
/// RtlAddGrowableFunctionTable: a sorted table that RtlGrowFunctionTable extends in
/// place, covering [begin, end). NTSTATUS result.
pub unsafe extern "win64" fn add_growable(
    out: *mut usize,
    entries: *mut RuntimeFunction,
    count: u32,
    maximum: u32,
    begin: u64,
    end: u64,
) -> i32 {
    if out.is_null() || entries.is_null() || count > maximum || end <= begin {
        return 0xC000000Du32 as i32;
    }
    let id = NEXT_GROWABLE.fetch_add(16, std::sync::atomic::Ordering::Relaxed);
    TABLES.lock().unwrap().push(Table {
        id,
        entries: Entries::Array(entries as usize, count, maximum),
        base: begin,
        begin,
        end,
        growable: true,
    });
    unsafe { *out = id };
    0
}
pub extern "win64" fn grow(table: usize, count: u32) {
    let mut tables = TABLES.lock().unwrap();
    if let Some(t) = tables.iter_mut().find(|t| t.id == table && t.growable) {
        if let Entries::Array(p, current, maximum) = t.entries {
            if count >= current && count <= maximum {
                t.entries = Entries::Array(p, count, maximum);
            }
        }
    }
}
pub extern "win64" fn delete_growable(table: usize) {
    TABLES
        .lock()
        .unwrap()
        .retain(|t| !(t.id == table && t.growable));
}
/// RtlPcToFileHeader: the base of the image containing `pc`.
pub unsafe extern "win64" fn pc_to_file_header(pc: usize, base: *mut usize) -> usize {
    let found = images::by_address(pc).map_or(0, |m| m.base);
    if !base.is_null() {
        unsafe { *base = found };
    }
    found
}
