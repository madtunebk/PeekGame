//! winhttp.dll without a network transport: session, connection and request
//! handles, options, timeouts and status callbacks behave as on Windows, and every
//! request fails as on a machine that cannot reach the server
//! (ERROR_WINHTTP_CANNOT_CONNECT). Asynchronous sessions report it through
//! WINHTTP_CALLBACK_STATUS_REQUEST_ERROR on a Windows worker thread. No proxy is
//! configured. HTTP requests made by a program's own stack (Unity's curl) are
//! unaffected.
use crate::api::system::error;
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
}
fn deliver(mut n: Notification) {
    let callback: Callback = unsafe { std::mem::transmute::<usize, Callback>(n.callback) };
    let length = n.info.len() as u32;
    let info = if n.info.is_empty() { std::ptr::null_mut() } else { n.info.as_mut_ptr() };
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
    let n = Notification { callback, handle, context, status, info };
    if asynchronous && !now {
        let _ = queue().lock().unwrap().send(n);
    } else {
        deliver(n);
    }
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
    NOTICE.call_once(|| eprintln!("WinHTTP: no network transport; requests fail with ERROR_WINHTTP_CANNOT_CONNECT"));
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
    _flags: u32,
) -> usize {
    let verb = unsafe { wide(verb) }.unwrap_or_else(|| "GET".into());
    let object = unsafe { wide(object) }.unwrap_or_else(|| "/".into());
    let host = with(|h| h.get(&connection).map(|c| c.target.clone())).unwrap_or_default();
    let handle = create_child(connection, Kind::Connect, Kind::Request, format!("{verb} {host}{object}"));
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
/// Headers are accepted onto the request; they would go out with it.
pub unsafe extern "win64" fn add_request_headers(
    handle: usize,
    headers: *const u16,
    _length: u32,
    _modifiers: u32,
) -> i32 {
    if headers.is_null() {
        return fail(ERROR_INVALID_PARAMETER, 0);
    }
    match request(handle) {
        Ok(()) => 1,
        Err(code) => fail(code, 0),
    }
}
/// The request cannot reach its server. Synchronous: FALSE with
/// ERROR_WINHTTP_CANNOT_CONNECT. Asynchronous: TRUE, then REQUEST_ERROR
/// (WINHTTP_ASYNC_RESULT { API_SEND_REQUEST, error }) with the request's context.
#[allow(clippy::too_many_arguments)]
pub extern "win64" fn send_request(
    handle: usize,
    _headers: *const u16,
    _headers_length: u32,
    _optional: *const u8,
    _optional_length: u32,
    _total_length: u32,
    context: usize,
) -> i32 {
    if let Err(code) = request(handle) {
        return fail(code, 0);
    }
    let (asynchronous, target) = with(|h| {
        let e = h.get_mut(&handle).unwrap();
        e.context = context;
        (e.asynchronous, e.target.clone())
    });
    if trace() {
        eprintln!("[http] WinHttpSendRequest {target}: cannot connect (no network transport)");
    }
    if !asynchronous {
        return fail(ERROR_WINHTTP_CANNOT_CONNECT, 0);
    }
    let mut result = API_SEND_REQUEST.to_le_bytes().to_vec();
    result.extend_from_slice(&(ERROR_WINHTTP_CANNOT_CONNECT as u64).to_le_bytes());
    notify(handle, STATUS_REQUEST_ERROR, result, false);
    error::set(0);
    1
}
/// Every later step needs a sent request, which never exists.
fn no_response(handle: usize) -> i32 {
    fail(request(handle).err().unwrap_or(ERROR_WINHTTP_INCORRECT_HANDLE_STATE), 0)
}
pub extern "win64" fn receive_response(handle: usize, _reserved: usize) -> i32 {
    no_response(handle)
}
pub extern "win64" fn query_headers(
    handle: usize,
    _level: u32,
    _name: usize,
    _buffer: usize,
    _length: usize,
    _index: usize,
) -> i32 {
    no_response(handle)
}
pub extern "win64" fn query_data_available(handle: usize, _available: usize) -> i32 {
    no_response(handle)
}
pub extern "win64" fn read_data(handle: usize, _buffer: usize, _length: u32, _read: usize) -> i32 {
    no_response(handle)
}
pub extern "win64" fn write_data(handle: usize, _buffer: usize, _length: u32, _written: usize) -> i32 {
    no_response(handle)
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
