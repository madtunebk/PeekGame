//! Performance Data Helper (pdh.dll): query lifecycle, performance counters,
//! and metric formatting for CPU, memory, and subsystem diagnostics.

use std::{
    collections::HashMap,
    ffi::{c_char, CStr},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex, OnceLock,
    },
};

const ERROR_SUCCESS: i32 = 0;
const PDH_CSTATUS_VALID_DATA: u32 = 0x00000000;
const PDH_MORE_DATA: i32 = 0x800007D2u32 as i32;
const PDH_INVALID_ARGUMENT: i32 = 0xC0000BB3u32 as i32;
const PDH_INVALID_HANDLE: i32 = 0xC0000BBCu32 as i32;

const PDH_FMT_LONG: u32 = 0x00000100;
const PDH_FMT_DOUBLE: u32 = 0x00000200;
const PDH_FMT_LARGE: u32 = 0x00000400;

const PERF_COUNTER_COUNTER: u32 = 0x10410400;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PdhFmtCountervalue {
    pub c_status: u32,
    pub _pad: u32,
    pub value: u64,
}

#[derive(Clone, Debug)]
enum CounterKind {
    CpuTime,
    MemoryAvailable,
    Generic(#[allow(dead_code)] String),
}

struct Counter {
    query_id: usize,
    kind: CounterKind,
    _user_data: usize,
}

struct Query {
    _user_data: usize,
    counters: Vec<usize>,
    last_cpu_work: u64,
    last_cpu_total: u64,
    current_cpu_percent: f64,
}

struct PdhState {
    queries: HashMap<usize, Query>,
    counters: HashMap<usize, Counter>,
}

impl PdhState {
    fn new() -> Self {
        Self {
            queries: HashMap::new(),
            counters: HashMap::new(),
        }
    }
}

static STATE: OnceLock<Mutex<PdhState>> = OnceLock::new();
static NEXT_QUERY_ID: AtomicUsize = AtomicUsize::new(1);
static NEXT_COUNTER_ID: AtomicUsize = AtomicUsize::new(1);

fn state() -> &'static Mutex<PdhState> {
    STATE.get_or_init(|| Mutex::new(PdhState::new()))
}

fn allocate_query_handle() -> usize {
    0x7068_0000 + (NEXT_QUERY_ID.fetch_add(1, Ordering::Relaxed) & 0x0000_ffff)
}

fn allocate_counter_handle() -> usize {
    0x7063_0000 + (NEXT_COUNTER_ID.fetch_add(1, Ordering::Relaxed) & 0x0000_ffff)
}

fn read_cpu_ticks() -> Option<(u64, u64)> {
    let stat = std::fs::read_to_string("/proc/stat").ok()?;
    let first_line = stat.lines().next()?;
    let mut parts = first_line.split_whitespace();
    if parts.next()? != "cpu" {
        return None;
    }
    let user: u64 = parts.next()?.parse().ok()?;
    let nice: u64 = parts.next()?.parse().ok()?;
    let system: u64 = parts.next()?.parse().ok()?;
    let idle: u64 = parts.next()?.parse().ok()?;
    let iowait: u64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let irq: u64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let softirq: u64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let steal: u64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);

    let work = user + nice + system + irq + softirq + steal;
    let total = work + idle + iowait;
    Some((work, total))
}

fn meminfo_mb(key: &str) -> f64 {
    std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|t| {
            t.lines().find_map(|l| {
                l.strip_prefix(key)?
                    .strip_prefix(':')?
                    .trim()
                    .split(' ')
                    .next()?
                    .parse::<f64>()
                    .ok()
            })
        })
        .map(|kib| kib / 1024.0)
        .unwrap_or(0.0)
}

fn classify_path(path: &str) -> CounterKind {
    let lower = path.to_ascii_lowercase();
    if lower.contains("processor") || lower.contains("cpu") {
        CounterKind::CpuTime
    } else if lower.contains("memory") && lower.contains("available") {
        CounterKind::MemoryAvailable
    } else {
        CounterKind::Generic(path.to_owned())
    }
}

unsafe fn wide_to_string(ptr: *const u16) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let mut len = 0;
    while unsafe { *ptr.add(len) } != 0 {
        len += 1;
    }
    String::from_utf16(unsafe { std::slice::from_raw_parts(ptr, len) }).ok()
}

unsafe fn narrow_to_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .ok()
        .map(str::to_owned)
}

pub unsafe extern "win64" fn open_query_a(
    _data_source: *const c_char,
    user_data: usize,
    query: *mut usize,
) -> i32 {
    if query.is_null() {
        return PDH_INVALID_ARGUMENT;
    }
    let (work, total) = read_cpu_ticks().unwrap_or((0, 0));
    let handle = allocate_query_handle();
    state().lock().unwrap().queries.insert(
        handle,
        Query {
            _user_data: user_data,
            counters: Vec::new(),
            last_cpu_work: work,
            last_cpu_total: total,
            current_cpu_percent: 0.0,
        },
    );
    unsafe { *query = handle };
    ERROR_SUCCESS
}

pub unsafe extern "win64" fn open_query_w(
    _data_source: *const u16,
    user_data: usize,
    query: *mut usize,
) -> i32 {
    unsafe { open_query_a(std::ptr::null(), user_data, query) }
}

pub unsafe extern "win64" fn open_query_h(
    _data_source: usize,
    user_data: usize,
    query: *mut usize,
) -> i32 {
    unsafe { open_query_a(std::ptr::null(), user_data, query) }
}

pub unsafe extern "win64" fn add_english_counter_w(
    query_handle: usize,
    counter_path: *const u16,
    user_data: usize,
    counter: *mut usize,
) -> i32 {
    if counter.is_null() {
        return PDH_INVALID_ARGUMENT;
    }
    let path = match unsafe { wide_to_string(counter_path) } {
        Some(p) => p,
        None => return PDH_INVALID_ARGUMENT,
    };
    let mut s = state().lock().unwrap();
    let Some(query) = s.queries.get_mut(&query_handle) else {
        return PDH_INVALID_HANDLE;
    };
    let handle = allocate_counter_handle();
    query.counters.push(handle);
    let kind = classify_path(&path);
    s.counters.insert(
        handle,
        Counter {
            query_id: query_handle,
            kind,
            _user_data: user_data,
        },
    );
    unsafe { *counter = handle };
    ERROR_SUCCESS
}

pub unsafe extern "win64" fn add_counter_w(
    query_handle: usize,
    counter_path: *const u16,
    user_data: usize,
    counter: *mut usize,
) -> i32 {
    unsafe { add_english_counter_w(query_handle, counter_path, user_data, counter) }
}

pub unsafe extern "win64" fn add_counter_a(
    query_handle: usize,
    counter_path: *const c_char,
    user_data: usize,
    counter: *mut usize,
) -> i32 {
    if counter.is_null() {
        return PDH_INVALID_ARGUMENT;
    }
    let path = match unsafe { narrow_to_string(counter_path) } {
        Some(p) => p,
        None => return PDH_INVALID_ARGUMENT,
    };
    let mut s = state().lock().unwrap();
    let Some(query) = s.queries.get_mut(&query_handle) else {
        return PDH_INVALID_HANDLE;
    };
    let handle = allocate_counter_handle();
    query.counters.push(handle);
    let kind = classify_path(&path);
    s.counters.insert(
        handle,
        Counter {
            query_id: query_handle,
            kind,
            _user_data: user_data,
        },
    );
    unsafe { *counter = handle };
    ERROR_SUCCESS
}

pub extern "win64" fn collect_query_data(query_handle: usize) -> i32 {
    let mut s = state().lock().unwrap();
    let Some(query) = s.queries.get_mut(&query_handle) else {
        return PDH_INVALID_HANDLE;
    };
    if let Some((work, total)) = read_cpu_ticks() {
        if query.last_cpu_total > 0 && total > query.last_cpu_total {
            let d_work = work.saturating_sub(query.last_cpu_work);
            let d_total = total - query.last_cpu_total;
            query.current_cpu_percent = (d_work as f64 / d_total as f64) * 100.0;
        }
        query.last_cpu_work = work;
        query.last_cpu_total = total;
    }
    ERROR_SUCCESS
}

pub unsafe extern "win64" fn get_formatted_counter_value(
    counter_handle: usize,
    format: u32,
    counter_type: *mut u32,
    value: *mut PdhFmtCountervalue,
) -> i32 {
    if value.is_null() {
        return PDH_INVALID_ARGUMENT;
    }
    let s = state().lock().unwrap();
    let Some(counter) = s.counters.get(&counter_handle) else {
        return PDH_INVALID_HANDLE;
    };
    let query = s.queries.get(&counter.query_id);

    if !counter_type.is_null() {
        unsafe { *counter_type = PERF_COUNTER_COUNTER };
    }

    let double_val = match &counter.kind {
        CounterKind::CpuTime => query.map(|q| q.current_cpu_percent).unwrap_or(0.0),
        CounterKind::MemoryAvailable => meminfo_mb("MemAvailable"),
        CounterKind::Generic(_) => 0.0,
    };

    let mut out = PdhFmtCountervalue {
        c_status: PDH_CSTATUS_VALID_DATA,
        _pad: 0,
        value: 0,
    };

    if format & PDH_FMT_DOUBLE != 0 {
        out.value = f64::to_bits(double_val);
    } else if format & PDH_FMT_LONG != 0 {
        out.value = (double_val as i64) as u64;
    } else if format & PDH_FMT_LARGE != 0 {
        out.value = double_val as u64;
    } else {
        out.value = f64::to_bits(double_val);
    }

    unsafe { *value = out };
    ERROR_SUCCESS
}

pub extern "win64" fn remove_counter(counter_handle: usize) -> i32 {
    let mut s = state().lock().unwrap();
    let Some(counter) = s.counters.remove(&counter_handle) else {
        return PDH_INVALID_HANDLE;
    };
    if let Some(query) = s.queries.get_mut(&counter.query_id) {
        query.counters.retain(|&h| h != counter_handle);
    }
    ERROR_SUCCESS
}

pub extern "win64" fn close_query(query_handle: usize) -> i32 {
    let mut s = state().lock().unwrap();
    let Some(query) = s.queries.remove(&query_handle) else {
        return PDH_INVALID_HANDLE;
    };
    for counter_handle in query.counters {
        s.counters.remove(&counter_handle);
    }
    ERROR_SUCCESS
}

pub unsafe extern "win64" fn enum_objects_a(
    _data_source: *const c_char,
    _machine_name: *const c_char,
    msz_object_list: *mut c_char,
    pcch_buffer_size: *mut u32,
    _detail_level: u32,
    _refresh: i32,
) -> i32 {
    if pcch_buffer_size.is_null() {
        return PDH_INVALID_ARGUMENT;
    }
    let objects = b"Processor\0Memory\0Process\0\0";
    let needed = objects.len() as u32;
    let buf_len = unsafe { *pcch_buffer_size };
    if msz_object_list.is_null() || buf_len < needed {
        unsafe { *pcch_buffer_size = needed };
        return PDH_MORE_DATA;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(objects.as_ptr().cast(), msz_object_list, objects.len());
        *pcch_buffer_size = needed;
    }
    ERROR_SUCCESS
}

pub unsafe extern "win64" fn enum_objects_w(
    _data_source: *const u16,
    _machine_name: *const u16,
    msz_object_list: *mut u16,
    pcch_buffer_size: *mut u32,
    _detail_level: u32,
    _refresh: i32,
) -> i32 {
    if pcch_buffer_size.is_null() {
        return PDH_INVALID_ARGUMENT;
    }
    let objects: &[u16] = &[
        b'P' as u16, b'r' as u16, b'o' as u16, b'c' as u16, b'e' as u16, b's' as u16, b's' as u16,
        b'o' as u16, b'r' as u16, 0, b'M' as u16, b'e' as u16, b'm' as u16, b'o' as u16,
        b'r' as u16, b'y' as u16, 0, b'P' as u16, b'r' as u16, b'o' as u16, b'c' as u16,
        b'e' as u16, b's' as u16, b's' as u16, 0, 0,
    ];
    let needed = objects.len() as u32;
    let buf_len = unsafe { *pcch_buffer_size };
    if msz_object_list.is_null() || buf_len < needed {
        unsafe { *pcch_buffer_size = needed };
        return PDH_MORE_DATA;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(objects.as_ptr(), msz_object_list, objects.len());
        *pcch_buffer_size = needed;
    }
    ERROR_SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdh_query_and_counter_lifecycle() {
        let mut query = 0;
        assert_eq!(
            unsafe { open_query_a(std::ptr::null(), 0, std::ptr::null_mut()) },
            PDH_INVALID_ARGUMENT
        );
        assert_eq!(
            unsafe { open_query_a(std::ptr::null(), 42, &mut query) },
            ERROR_SUCCESS
        );
        assert_ne!(query, 0);

        let path_w: Vec<u16> = "\\Processor(_Total)\\% Processor Time\0"
            .encode_utf16()
            .collect();
        let mut counter = 0;
        assert_eq!(
            unsafe {
                add_english_counter_w(
                    0x1234, // invalid handle
                    path_w.as_ptr(),
                    0,
                    &mut counter,
                )
            },
            PDH_INVALID_HANDLE
        );
        assert_eq!(
            unsafe { add_english_counter_w(query, path_w.as_ptr(), 0, &mut counter) },
            ERROR_SUCCESS
        );
        assert_ne!(counter, 0);

        assert_eq!(collect_query_data(query), ERROR_SUCCESS);

        let mut val = PdhFmtCountervalue {
            c_status: 1,
            _pad: 0,
            value: 0,
        };
        let mut ctype = 0;
        assert_eq!(
            unsafe {
                get_formatted_counter_value(counter, PDH_FMT_DOUBLE, &mut ctype, &mut val)
            },
            ERROR_SUCCESS
        );
        assert_eq!(val.c_status, PDH_CSTATUS_VALID_DATA);
        assert_eq!(ctype, PERF_COUNTER_COUNTER);

        assert_eq!(remove_counter(counter), ERROR_SUCCESS);
        assert_eq!(remove_counter(counter), PDH_INVALID_HANDLE);
        assert_eq!(close_query(query), ERROR_SUCCESS);
        assert_eq!(close_query(query), PDH_INVALID_HANDLE);
    }

    #[test]
    fn test_pdh_enum_objects() {
        let mut needed = 0;
        assert_eq!(
            unsafe {
                enum_objects_a(
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    &mut needed,
                    0,
                    0,
                )
            },
            PDH_MORE_DATA
        );
        assert!(needed > 0);

        let mut buf = vec![0i8; needed as usize];
        assert_eq!(
            unsafe {
                enum_objects_a(
                    std::ptr::null(),
                    std::ptr::null(),
                    buf.as_mut_ptr(),
                    &mut needed,
                    0,
                    0,
                )
            },
            ERROR_SUCCESS
        );
        assert_eq!(buf[0] as u8, b'P');
    }
}
