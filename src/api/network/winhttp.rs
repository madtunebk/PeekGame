//! WinHTTP handle and callback layer. GET, HEAD and POST requests
//! use the host's TLS-verified libcurl transport. Asynchronous requests use
//! Windows worker threads and deliver completion callbacks. Streaming request
//! and unsupported response queries fail explicitly.
use crate::api::system::error;
mod transport;
mod url;
pub use url::crack_url;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Mutex, OnceLock,
    },
};

const ERROR_INVALID_HANDLE: u32 = 6;
const ERROR_INVALID_PARAMETER: u32 = 87;
const ERROR_WINHTTP_INVALID_OPTION: u32 = 12009;
const ERROR_WINHTTP_INCORRECT_HANDLE_TYPE: u32 = 12018;
const ERROR_WINHTTP_INCORRECT_HANDLE_STATE: u32 = 12019;
const ERROR_WINHTTP_CANNOT_CONNECT: u32 = 12029;
const ERROR_WINHTTP_AUTODETECTION_FAILED: u32 = 12180;
const FLAG_ASYNC: u32 = 0x1000_0000;
const STATUS_HANDLE_CREATED: u32 = 0x0000_0400;
const STATUS_HANDLE_CLOSING: u32 = 0x0000_0800;
const STATUS_REQUEST_ERROR: u32 = 0x0020_0000;
const STATUS_SENDREQUEST_COMPLETE: u32 = 0x0040_0000;
const STATUS_HEADERS_AVAILABLE: u32 = 0x0002_0000;
const STATUS_DATA_AVAILABLE: u32 = 0x0004_0000;
const STATUS_READ_COMPLETE: u32 = 0x0008_0000;
const STATUS_WRITE_COMPLETE: u32 = 0x0010_0000;
const API_SEND_REQUEST: usize = 5;
const OPTION_CONTEXT_VALUE: u32 = 45;
/// WINHTTP_INVALID_STATUS_CALLBACK.
const INVALID_CALLBACK: usize = usize::MAX;
/// Options that only shape a connection; they are kept per handle, but no
/// connection is ever made.
const CONNECTION_OPTIONS: &[u32] = &[
    2, 3, 4, 5, 6, 7, 12, 13, 31, 38, 41, 47, 58, 59, 63, 68, 73, 74, 77, 79, 80, 84, 88, 89,
    90, 91, 92, 118, 133, 0x1000, 0x1001, 0x1002, 0x1003,
];

type Callback = unsafe extern "win64" fn(usize, usize, u32, *mut u8, u32);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Session,
    Connect,
    Request,
}

struct Handle {
    kind: Kind,
    asynchronous: bool,
    callback: usize,
    notifications: u32,
    context: usize,
    target: String,
    host: String,
    port: u16,
    method: String,
    path: String,
    secure: bool,
    response: Option<transport::Response>,
    pending: Option<FetchJob>,
    headers: Vec<String>,
    options: HashMap<u32, Vec<u8>>,
}

static HANDLES: Mutex<Option<HashMap<usize, Handle>>> = Mutex::new(None);
static NEXT: AtomicUsize = AtomicUsize::new(0x000f_0010);

fn trace() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("PEEKGAME_TRACE_HTTP").is_some())
}
fn fail<T>(code: u32, value: T) -> T {
    error::set(code);
    value
}
fn with<T>(f: impl FnOnce(&mut HashMap<usize, Handle>) -> T) -> T {
    f(HANDLES.lock().unwrap().get_or_insert_with(HashMap::new))
}
unsafe fn trace_headers(source: &str, ptr: *const u16, length: u32) {
    if !trace() || ptr.is_null() || length == 0 { return; }
    let text = if length == u32::MAX {
        unsafe { wide(ptr) }
    } else if length <= 65536 {
        Some(String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(ptr, length as usize) }))
    } else {
        None
    };
    if let Some(text) = text {
        for line in text.split("\r\n").filter(|line| !line.is_empty()) {
            if let Some((name, _)) = line.split_once(':') {
                let name = name.trim();
                if !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
                    eprintln!("[http] {source} header {name}: <redacted>");
                }
            }
        }
    }
}
unsafe fn header_lines(ptr: *const u16, length: u32) -> Result<Vec<String>, u32> {
    if ptr.is_null() { return Err(ERROR_INVALID_PARAMETER); }
    let text = if length == u32::MAX {
        unsafe { wide(ptr) }.ok_or(ERROR_INVALID_PARAMETER)?
    } else if length <= 65536 {
        String::from_utf16(unsafe { std::slice::from_raw_parts(ptr, length as usize) })
            .map_err(|_| ERROR_INVALID_PARAMETER)?
    } else {
        return Err(ERROR_INVALID_PARAMETER);
    };
    let mut out = Vec::new();
    for line in text.split("\r\n").filter(|line| !line.is_empty()) {
        let (name, value) = line.split_once(':').ok_or(ERROR_INVALID_PARAMETER)?;
        if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || value.contains(['\r', '\n', '\0'])
        {
            return Err(ERROR_INVALID_PARAMETER);
        }
        out.push(format!("{name}:{}", value));
    }
    if out.is_empty() { return Err(ERROR_INVALID_PARAMETER); }
    Ok(out)
}
unsafe fn wide(ptr: *const u16) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let len = (0..65536).position(|i| unsafe { *ptr.add(i) } == 0)?;
    Some(String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(ptr, len) }))
}

// ---- status callbacks ------------------------------------------------------

/// A notification: the status information is owned here and outlives the call.
struct Notification {
    callback: usize,
    handle: usize,
    context: usize,
    status: u32,
    info: Vec<u8>,
    info_ptr: Option<usize>,
}
fn deliver(mut n: Notification) {
    let callback: Callback = unsafe { std::mem::transmute::<usize, Callback>(n.callback) };
    let length = n.info.len() as u32;
    let info = if let Some(pointer) = n.info_ptr {
        pointer as *mut u8
    } else if n.info.is_empty() {
        std::ptr::null_mut()
    } else {
        n.info.as_mut_ptr()
    };
    unsafe { callback(n.handle, n.context, n.status, info, length) };
}
/// Asynchronous notifications run in order on one Windows thread, never on the
/// caller's thread and without the handle table locked.
fn queue() -> &'static Mutex<mpsc::Sender<Notification>> {
    static QUEUE: OnceLock<Mutex<mpsc::Sender<Notification>>> = OnceLock::new();
    static RECEIVER: Mutex<Option<mpsc::Receiver<Notification>>> = Mutex::new(None);
    QUEUE.get_or_init(|| {
        let (send, receive) = mpsc::channel();
        *RECEIVER.lock().unwrap() = Some(receive);
        extern "win64" fn worker(_: usize) -> u32 {
            let receive = RECEIVER.lock().unwrap().take().unwrap();
            for n in receive {
                deliver(n);
            }
            0
        }
        let thread = unsafe {
            crate::api::threading::threads::create(std::ptr::null(), 0, Some(worker), 0, 0, std::ptr::null_mut())
        };
        if thread == 0 {
            eprintln!("WinHTTP: the callback thread could not start");
        } else {
            crate::api::threading::threads::close(thread);
        }
        Mutex::new(send)
    })
}
/// Sends `status` to the handle's callback if it asked for it: on the worker
/// thread for asynchronous handles, otherwise now.
fn notify(handle: usize, status: u32, info: Vec<u8>, now: bool) {
    let target = with(|h| {
        h.get(&handle).and_then(|e| {
            (e.callback != 0 && e.notifications & status != 0)
                .then_some((e.callback, e.context, e.asynchronous))
        })
    });
    let Some((callback, context, asynchronous)) = target else {
        return;
    };
    let n = Notification { callback, handle, context, status, info, info_ptr: None };
    if asynchronous && !now {
        let _ = queue().lock().unwrap().send(n);
    } else {
        deliver(n);
    }
}

/// For READ_COMPLETE, WinHTTP passes back the buffer supplied by the caller.
fn notify_read(handle: usize, buffer: usize, length: usize) {
    let target = with(|h| h.get(&handle).and_then(|e| {
        (e.callback != 0 && e.notifications & STATUS_READ_COMPLETE != 0)
            .then_some((e.callback, e.context))
    }));
    let Some((callback, context)) = target else { return; };
    let n = Notification {
        callback, handle, context, status: STATUS_READ_COMPLETE,
        info: vec![0; length], info_ptr: Some(buffer),
    };
    let _ = queue().lock().unwrap().send(n);
}

// ---- handles ---------------------------------------------------------------

/// A child of `parent` (which must be `parent_kind`), inheriting its mode and callback.
fn create_child(parent: usize, parent_kind: Kind, kind: Kind, target: String) -> usize {
    let handle = NEXT.fetch_add(16, Ordering::Relaxed);
    let created = with(|h| {
        let Some(p) = h.get(&parent) else {
            return Err(ERROR_INVALID_HANDLE);
        };
        if p.kind != parent_kind {
            return Err(ERROR_WINHTTP_INCORRECT_HANDLE_TYPE);
        }
        let child = Handle {
            kind,
            asynchronous: p.asynchronous,
            callback: p.callback,
            notifications: p.notifications,
            context: 0,
            target,
            host: p.host.clone(),
            port: p.port,
            method: String::new(),
            path: String::new(),
            secure: false,
            response: None,
            pending: None,
            headers: Vec::new(),
            options: HashMap::new(),
        };
        h.insert(handle, child);
        Ok(())
    });
    if let Err(code) = created {
        return fail(code, 0);
    }
    notify(handle, STATUS_HANDLE_CREATED, handle.to_le_bytes().to_vec(), true);
    error::set(0);
    handle
}
pub unsafe extern "win64" fn open(
    agent: *const u16,
    access: u32,
    _proxy: usize,
    _bypass: usize,
    flags: u32,
) -> usize {
    if flags & !(FLAG_ASYNC | 0x2000_0000) != 0 || access > 4 {
        return fail(ERROR_INVALID_PARAMETER, 0);
    }
    static NOTICE: std::sync::Once = std::sync::Once::new();
    NOTICE.call_once(|| eprintln!("WinHTTP: GET/HEAD transport enabled (synchronous and asynchronous)"));
    let handle = NEXT.fetch_add(16, Ordering::Relaxed);
    let agent = unsafe { wide(agent) }.unwrap_or_default();
    if trace() {
        eprintln!("[http] WinHttpOpen '{agent}' flags={flags:#x} -> {handle:#x}");
    }
    with(|h| {
        h.insert(
            handle,
            Handle {
                kind: Kind::Session,
                asynchronous: flags & FLAG_ASYNC != 0,
                callback: 0,
                notifications: 0,
                context: 0,
                target: agent,
                host: String::new(),
                port: 0,
                method: String::new(),
                path: String::new(),
                secure: false,
                response: None,
                pending: None,
                headers: Vec::new(),
                options: HashMap::new(),
            },
        )
    });
    error::set(0);
    handle
}
pub unsafe extern "win64" fn connect(session: usize, server: *const u16, port: u16, reserved: u32) -> usize {
    let Some(server) = (unsafe { wide(server) }).filter(|s| !s.is_empty()) else {
        return fail(ERROR_INVALID_PARAMETER, 0);
    };
    if reserved != 0 {
        return fail(ERROR_INVALID_PARAMETER, 0);
    }
    let handle = create_child(session, Kind::Session, Kind::Connect, format!("{server}:{port}"));
    with(|h| { if let Some(e) = h.get_mut(&handle) { e.host = server.clone(); e.port = port; } });
    if trace() {
        eprintln!("[http] WinHttpConnect {server}:{port} -> {handle:#x}");
    }
    handle
}
#[allow(clippy::too_many_arguments)]
pub unsafe extern "win64" fn open_request(
    connection: usize,
    verb: *const u16,
    object: *const u16,
    _version: *const u16,
    _referrer: *const u16,
    _accept_types: *const *const u16,
    flags: u32,
) -> usize {
    let verb = unsafe { wide(verb) }.unwrap_or_else(|| "GET".into());
    let object = unsafe { wide(object) }.unwrap_or_else(|| "/".into());
    let host = with(|h| h.get(&connection).map(|c| c.target.clone())).unwrap_or_default();
    let handle = create_child(connection, Kind::Connect, Kind::Request, format!("{verb} {host}{object}"));
    with(|h| { if let Some(e) = h.get_mut(&handle) { e.method = verb.clone(); e.path = object.clone(); e.secure = flags & 0x0080_0000 != 0; } });
    if trace() {
        eprintln!("[http] WinHttpOpenRequest {verb} {host}{object} -> {handle:#x}");
    }
    handle
}
pub extern "win64" fn close_handle(handle: usize) -> i32 {
    if with(|h| !h.contains_key(&handle)) {
        return fail(ERROR_INVALID_HANDLE, 0);
    }
    notify(handle, STATUS_HANDLE_CLOSING, handle.to_le_bytes().to_vec(), false);
    with(|h| h.remove(&handle));
    1
}
/// Returns the previous callback; child handles made afterwards inherit it.
pub extern "win64" fn set_status_callback(
    handle: usize,
    callback: usize,
    notifications: u32,
    _reserved: usize,
) -> usize {
    with(|h| match h.get_mut(&handle) {
        Some(e) => {
            let previous = e.callback;
            e.callback = callback;
            e.notifications = if callback == 0 { 0 } else { notifications };
            previous
        }
        None => fail(ERROR_INVALID_HANDLE, INVALID_CALLBACK),
    })
}
pub unsafe extern "win64" fn set_option(handle: usize, option: u32, buffer: *const u8, length: u32) -> i32 {
    if buffer.is_null() && length != 0 {
        return fail(ERROR_INVALID_PARAMETER, 0);
    }
    let bytes = if length == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(buffer, length as usize) }.to_vec()
    };
    let result = with(|h| {
        let Some(e) = h.get_mut(&handle) else {
            return Err(ERROR_INVALID_HANDLE);
        };
        if option == OPTION_CONTEXT_VALUE {
            if bytes.len() != 8 {
                return Err(ERROR_INVALID_PARAMETER);
            }
            e.context = usize::from_le_bytes(bytes[..8].try_into().unwrap());
        } else if CONNECTION_OPTIONS.contains(&option) {
            e.options.insert(option, bytes);
        } else {
            return Err(ERROR_WINHTTP_INVALID_OPTION);
        }
        Ok(())
    });
    if trace() {
        eprintln!("[http] WinHttpSetOption {handle:#x} option={option} length={length} -> {result:?}");
    }
    match result {
        Ok(()) => 1,
        Err(code) => fail(code, 0),
    }
}
pub extern "win64" fn set_timeouts(handle: usize, resolve: i32, connect: i32, send: i32, receive: i32) -> i32 {
    if [resolve, connect, send, receive].iter().any(|&t| t < -1) {
        return fail(ERROR_INVALID_PARAMETER, 0);
    }
    with(|h| match h.get_mut(&handle) {
        Some(e) => {
            for (option, value) in [(2, resolve), (3, connect), (5, send), (6, receive)] {
                e.options.insert(option, value.to_le_bytes().to_vec());
            }
            1
        }
        None => fail(ERROR_INVALID_HANDLE, 0),
    })
}
/// The request handle's state, or the Windows error for a wrong handle.
fn request(handle: usize) -> Result<(), u32> {
    with(|h| match h.get(&handle) {
        Some(e) if e.kind == Kind::Request => Ok(()),
        Some(_) => Err(ERROR_WINHTTP_INCORRECT_HANDLE_TYPE),
        None => Err(ERROR_INVALID_HANDLE),
    })
}
/// Add request headers to the handle; the transport sends them verbatim.
pub unsafe extern "win64" fn add_request_headers(
    handle: usize, headers: *const u16, length: u32, modifiers: u32,
) -> i32 {
    if let Err(code) = request(handle) { return fail(code, 0); }
    unsafe { trace_headers("WinHttpAddRequestHeaders", headers, length) };
    if trace() { eprintln!("[http] WinHttpAddRequestHeaders modifiers={modifiers:#x}"); }
    const ADD_IF_NEW: u32 = 0x1000_0000;
    const ADD: u32 = 0x2000_0000;
    const COMMA: u32 = 0x4000_0000;
    const SEMICOLON: u32 = 0x0100_0000;
    const REPLACE: u32 = 0x8000_0000;
    if modifiers & !(ADD_IF_NEW | ADD | COMMA | SEMICOLON | REPLACE) != 0 {
        return fail(50, 0);
    }
    let lines = match unsafe { header_lines(headers, length) } {
        Ok(lines) => lines,
        Err(code) => return fail(code, 0),
    };
    with(|h| {
        let stored = &mut h.get_mut(&handle).unwrap().headers;
        for line in lines {
            let name = line.split_once(':').unwrap().0;
            let existing = stored.iter().position(|old| old.split_once(':')
                .is_some_and(|(old_name, _)| old_name.eq_ignore_ascii_case(name)));
            if modifiers & ADD_IF_NEW != 0 && existing.is_some() { continue; }
            if modifiers & REPLACE != 0 {
                stored.retain(|old| !old.split_once(':')
                    .is_some_and(|(old_name, _)| old_name.eq_ignore_ascii_case(name)));
            } else if modifiers & (COMMA | SEMICOLON) != 0 {
                if let Some(index) = existing {
                    let separator = if modifiers & SEMICOLON != 0 { ";" } else { "," };
                    stored[index].push_str(separator);
                    stored[index].push_str(line.split_once(':').unwrap().1.trim());
                    continue;
                }
            }
            stored.push(line);
        }
    });
    error::set(0);
    1
}
struct FetchJob {
    handle: usize,
    target: String,
    host: String,
    port: u16,
    method: String,
    path: String,
    secure: bool,
    connect_ms: i32,
    total_ms: i32,
    headers: Vec<String>,
    body: Vec<u8>,
    total_length: u32,
    completion: u32,
}
extern "win64" fn fetch_worker(context: usize) -> u32 {
    let job = unsafe { Box::from_raw(context as *mut FetchJob) };
    match transport::fetch(&job.host, job.port, &job.method, &job.path, job.secure, job.connect_ms, job.total_ms, &job.headers, &job.body) {
        Ok(response) => {
            if trace() { eprintln!("[http] WinHttpSendRequest {}: HTTP {} ({} body bytes)", job.target, response.status, response.body.len()); }
            let alive = with(|h| {
                if let Some(e) = h.get_mut(&job.handle) {
                    e.response = Some(response);
                    true
                } else { false }
            });
            if alive { notify(job.handle, job.completion, Vec::new(), false); }
        }
        Err(code) => {
            if trace() { eprintln!("[http] WinHttpSendRequest {}: transport error {code}", job.target); }
            let mut result = API_SEND_REQUEST.to_le_bytes().to_vec();
            result.extend_from_slice(&(code as u64).to_le_bytes());
            notify(job.handle, STATUS_REQUEST_ERROR, result, true);
        }
    }
    0
}
#[allow(clippy::too_many_arguments)]
pub extern "win64" fn send_request(
    handle: usize,
    headers: *const u16,
    headers_length: u32,
    optional: *const u8,
    optional_length: u32,
    total_length: u32,
    context: usize,
) -> i32 {
    if let Err(code) = request(handle) { return fail(code, 0); }
    unsafe { trace_headers("WinHttpSendRequest", headers, headers_length) };
    if optional_length > 16 * 1024 * 1024 || (optional_length != 0 && optional.is_null()) {
        return fail(ERROR_INVALID_PARAMETER, 0);
    }
    if total_length < optional_length || total_length > 16 * 1024 * 1024 {
        return fail(ERROR_INVALID_PARAMETER, 0);
    }
    let body = if optional_length == 0 { Vec::new() } else {
        unsafe { std::slice::from_raw_parts(optional, optional_length as usize) }.to_vec()
    };
    let extra_headers = if headers.is_null() || headers_length == 0 {
        Vec::new()
    } else {
        match unsafe { header_lines(headers, headers_length) } {
            Ok(lines) => lines,
            Err(code) => return fail(code, 0),
        }
    };
    let (asynchronous, target, host, port, method, path, secure, connect_ms, total_ms, all_headers) = with(|h| {
        let e = h.get_mut(&handle).unwrap();
        e.context = context;
        e.headers.extend(extra_headers);
        e.response = None;
        e.pending = None;
        let timeout = |key| e.options.get(&key).and_then(|b| b.get(..4)).map(|b| i32::from_le_bytes(b.try_into().unwrap())).unwrap_or(0);
        (e.asynchronous, e.target.clone(), e.host.clone(), e.port, e.method.clone(), e.path.clone(), e.secure, timeout(3), timeout(6), e.headers.clone())
    });
    if method != "POST" && total_length != 0 { return fail(50, 0); }
    if method == "POST" {
        for header in &all_headers {
            if let Some((name, value)) = header.split_once(':') {
                if name.trim().eq_ignore_ascii_case("Content-Length") {
                    if value.trim().parse::<u32>().ok() != Some(total_length) {
                        return fail(ERROR_INVALID_PARAMETER, 0);
                    }
                }
            }
        }
    }
    let job = FetchJob { handle, target, host, port, method, path, secure, connect_ms, total_ms,
        headers: all_headers, body, total_length, completion: STATUS_SENDREQUEST_COMPLETE };
    if total_length != optional_length {
        with(|h| { h.get_mut(&handle).unwrap().pending = Some(job); });
        if trace() { eprintln!("[http] WinHttpSendRequest queued streaming body: {optional_length}/{total_length} bytes"); }
        if asynchronous { notify(handle, STATUS_SENDREQUEST_COMPLETE, Vec::new(), false); }
        error::set(0);
        return 1;
    }
    if !asynchronous {
        match transport::fetch(&job.host, job.port, &job.method, &job.path, job.secure,
            job.connect_ms, job.total_ms, &job.headers, &job.body) {
            Ok(response) => {
                if trace() { eprintln!("[http] WinHttpSendRequest {}: HTTP {} ({} body bytes)", job.target, response.status, response.body.len()); }
                with(|h| { if let Some(e) = h.get_mut(&handle) { e.response = Some(response); } });
                error::set(0);
                return 1;
            }
            Err(code) => {
                if trace() { eprintln!("[http] WinHttpSendRequest {}: transport error {code}", job.target); }
                return fail(code, 0);
            }
        }
    }
    let pointer = Box::into_raw(Box::new(job)) as usize;
    let thread = unsafe { crate::api::threading::threads::create(
        std::ptr::null(), 0, Some(fetch_worker), pointer, 0, std::ptr::null_mut(),
    ) };
    if thread == 0 {
        unsafe { drop(Box::from_raw(pointer as *mut FetchJob)); }
        return fail(ERROR_WINHTTP_CANNOT_CONNECT, 0);
    }
    crate::api::threading::threads::close(thread);
    error::set(0);
    1
}
/// The synchronous first slice stores a complete response after SendRequest.
fn no_response(handle: usize) -> i32 {
    fail(request(handle).err().unwrap_or(ERROR_WINHTTP_INCORRECT_HANDLE_STATE), 0)
}
pub extern "win64" fn receive_response(handle: usize, _reserved: usize) -> i32 {
    let state = with(|h| h.get_mut(&handle).map(|e| (e.asynchronous, e.response.is_some(), e.pending.take())));
    let Some((asynchronous, ready, pending)) = state else { return fail(ERROR_INVALID_HANDLE, 0); };
    if let Some(mut job) = pending {
        if job.body.len() != job.total_length as usize {
            with(|h| { if let Some(e) = h.get_mut(&handle) { e.pending = Some(job); } });
            return fail(ERROR_WINHTTP_INCORRECT_HANDLE_STATE, 0);
        }
        if asynchronous {
            job.completion = STATUS_HEADERS_AVAILABLE;
            let pointer = Box::into_raw(Box::new(job)) as usize;
            let thread = unsafe { crate::api::threading::threads::create(
                std::ptr::null(), 0, Some(fetch_worker), pointer, 0, std::ptr::null_mut(),
            ) };
            if thread == 0 {
                let job = unsafe { *Box::from_raw(pointer as *mut FetchJob) };
                with(|h| { if let Some(e) = h.get_mut(&handle) { e.pending = Some(job); } });
                return fail(ERROR_WINHTTP_CANNOT_CONNECT, 0);
            }
            crate::api::threading::threads::close(thread);
            error::set(0);
            return 1;
        }
        match transport::fetch(&job.host, job.port, &job.method, &job.path, job.secure,
            job.connect_ms, job.total_ms, &job.headers, &job.body) {
            Ok(response) => {
                if trace() { eprintln!("[http] WinHttpReceiveResponse {}: HTTP {} ({} body bytes)", job.target, response.status, response.body.len()); }
                with(|h| { if let Some(e) = h.get_mut(&handle) { e.response = Some(response); } });
                error::set(0);
                return 1;
            }
            Err(code) => return fail(code, 0),
        }
    }
    if !ready { return no_response(handle); }
    if asynchronous { notify(handle, STATUS_HEADERS_AVAILABLE, Vec::new(), false); }
    error::set(0);
    1
}
pub unsafe extern "win64" fn query_headers(
    handle: usize, level: u32, name: usize, buffer: usize, length: usize, index: usize,
) -> i32 {
    const HEADER_NOT_FOUND: u32 = 12150;
    const INSUFFICIENT_BUFFER: u32 = 122;
    const NUMBER: u32 = 0x2000_0000;
    const SYSTEMTIME: u32 = 0x4000_0000;
    let query = level & 0xffff;
    let flags = level & 0xffff_0000;
    if trace() {
        let label = match query {
            1 => "CONTENT_TYPE", 19 => "STATUS_CODE", 21 => "RAW_HEADERS",
            22 => "RAW_HEADERS_CRLF", 0xffff => "CUSTOM", _ => "OTHER",
        };
        let custom_name = if query == 0xffff && name != 0 {
            unsafe { wide(name as *const u16) }.unwrap_or_else(|| "<invalid>".into())
        } else { String::new() };
        eprintln!("[http] WinHttpQueryHeaders handle={handle:#x} level={level:#x} ({label}) name={custom_name:?} capacity={}",
            if length == 0 { 0 } else { unsafe { *(length as *const u32) } });
    }
    let result = with(|h| h.get(&handle).and_then(|e| e.response.as_ref()).map(|r| {
        let header = match query {
            1 => Some("content-type"), 5 => Some("content-length"),
            9 => Some("date"), 10 => Some("expires"), 11 => Some("last-modified"),
            29 => Some("content-encoding"), 33 => Some("if-modified-since"),
            63 => Some("transfer-encoding"),
            _ => None,
        };
        let text = match query {
            19 => Some(r.status.to_string()),
            20 => r.status_line.splitn(3, ' ').nth(2).map(str::to_string),
            21 | 22 => {
                let lines = r.headers.iter().map(|(k,v)| format!("{k}: {v}\r\n")).collect::<String>();
                Some(format!("{}\r\n{lines}\r\n", r.status_line))
            },
            0xffff if name != 0 => unsafe { wide(name as *const u16) }
                .and_then(|n| r.headers.iter().find(|(k,_)| k.eq_ignore_ascii_case(&n)).map(|(_,v)| v.clone())),
            _ => header.and_then(|n| r.headers.iter().find(|(k,_)| k == n).map(|(_,v)| v.clone())),
        };
        (r.status, text)
    }));
    let Some((status, value)) = result else { return no_response(handle); };
    if flags & SYSTEMTIME != 0 {
        if trace() { eprintln!("[http] WinHttpQueryHeaders SYSTEMTIME conversion unsupported -> ERROR_NOT_SUPPORTED (50)"); }
        return fail(50, 0);
    }
    if flags & !NUMBER != 0 { return fail(50, 0); }
    if length == 0 { return fail(ERROR_INVALID_PARAMETER, 0); }
    let Some(value) = value else {
        if trace() { eprintln!("[http] WinHttpQueryHeaders level={level:#x} -> HEADER_NOT_FOUND"); }
        return fail(HEADER_NOT_FOUND, 0);
    };
    if query == 1 && trace() { eprintln!("[http] WinHttpQueryHeaders Content-Type -> {value:?}"); }
    if flags & NUMBER != 0 {
        let number = if query == 19 { Some(status) } else { value.parse::<u32>().ok() };
        let Some(number) = number else { return fail(HEADER_NOT_FOUND, 0); };
        let capacity = unsafe { *(length as *mut u32) };
        unsafe { *(length as *mut u32) = 4; }
        if buffer == 0 || capacity < 4 { return fail(INSUFFICIENT_BUFFER, 0); }
        unsafe { *(buffer as *mut u32) = number; }
    } else {
        let wide: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();
        let needed = (wide.len() * 2) as u32;
        let capacity = unsafe { *(length as *mut u32) };
        unsafe { *(length as *mut u32) = needed; }
        if buffer == 0 || capacity < needed { return fail(INSUFFICIENT_BUFFER, 0); }
        unsafe { std::ptr::copy_nonoverlapping(wide.as_ptr(), buffer as *mut u16, wide.len()); }
    }
    if index != 0 { unsafe { *(index as *mut u32) = 0; } }
    if trace() { eprintln!("[http] WinHttpQueryHeaders level={level:#x} -> success"); }
    error::set(0);
    1
}
pub unsafe extern "win64" fn query_data_available(handle: usize, available: usize) -> i32 {
    let remaining = with(|h| h.get(&handle).and_then(|e| e.response.as_ref()).map(|r| r.body.len() - r.cursor));
    let Some(remaining) = remaining else { return no_response(handle); };
    let asynchronous = with(|h| h.get(&handle).is_some_and(|e| e.asynchronous));
    if asynchronous {
        notify(handle, STATUS_DATA_AVAILABLE, (remaining.min(u32::MAX as usize) as u32).to_le_bytes().to_vec(), false);
    } else {
        if available == 0 { return fail(ERROR_INVALID_PARAMETER, 0); }
        unsafe { *(available as *mut u32) = remaining.min(u32::MAX as usize) as u32; }
    }
    error::set(0);
    1
}
pub unsafe extern "win64" fn read_data(handle: usize, buffer: usize, length: u32, read: usize) -> i32 {
    if buffer == 0 && length != 0 { return fail(ERROR_INVALID_PARAMETER, 0); }
    let result = with(|h| {
        let Some(e) = h.get_mut(&handle) else { return None; };
        if !e.asynchronous && read == 0 { return Some(Err(ERROR_INVALID_PARAMETER)); }
        let Some(r) = e.response.as_mut() else { return None; };
        let n = (length as usize).min(r.body.len() - r.cursor);
        if n != 0 { unsafe { std::ptr::copy_nonoverlapping(r.body.as_ptr().add(r.cursor), buffer as *mut u8, n) }; }
        r.cursor += n;
        Some(Ok((n, e.asynchronous)))
    });
    let Some(result) = result else { return no_response(handle); };
    let (n, asynchronous) = match result {
        Ok(result) => result,
        Err(code) => return fail(code, 0),
    };
    if asynchronous {
        notify_read(handle, buffer, n);
    } else {
        unsafe { *(read as *mut u32) = n as u32; }
    }
    error::set(0);
    1
}
pub unsafe extern "win64" fn write_data(handle: usize, buffer: usize, length: u32, written: usize) -> i32 {
    if let Err(code) = request(handle) { return fail(code, 0); }
    if length != 0 && buffer == 0 { return fail(ERROR_INVALID_PARAMETER, 0); }
    let asynchronous = with(|h| h.get(&handle).unwrap().asynchronous);
    if !asynchronous && written == 0 { return fail(ERROR_INVALID_PARAMETER, 0); }
    let result = with(|h| {
        let e = h.get_mut(&handle).unwrap();
        let Some(job) = e.pending.as_mut() else { return Err(ERROR_WINHTTP_INCORRECT_HANDLE_STATE); };
        if length as usize > (job.total_length as usize).saturating_sub(job.body.len()) {
            return Err(ERROR_INVALID_PARAMETER);
        }
        if length != 0 {
            job.body.extend_from_slice(unsafe { std::slice::from_raw_parts(buffer as *const u8, length as usize) });
        }
        Ok(())
    });
    if let Err(code) = result { return fail(code, 0); }
    if asynchronous {
        notify(handle, STATUS_WRITE_COMPLETE, length.to_le_bytes().to_vec(), false);
    } else {
        unsafe { *(written as *mut u32) = length; }
    }
    error::set(0);
    1
}

// ---- proxy configuration --------------------------------------------------

/// WINHTTP_CURRENT_USER_IE_PROXY_CONFIG: fAutoDetect, then three string pointers;
/// all empty.
pub unsafe extern "win64" fn ie_proxy_config(out: *mut u8) -> i32 {
    if out.is_null() {
        return fail(ERROR_INVALID_PARAMETER, 0);
    }
    unsafe { out.write_bytes(0, 32) };
    1
}
/// WinHttpGetProxyForUrl: ERROR_WINHTTP_AUTODETECTION_FAILED (no proxy script).
pub extern "win64" fn proxy_for_url(_session: usize, _url: usize, _options: usize, _info: usize) -> i32 {
    fail(ERROR_WINHTTP_AUTODETECTION_FAILED, 0)
}
pub extern "win64" fn check_platform() -> i32 {
    1
}
