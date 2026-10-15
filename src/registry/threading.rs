use super::*;
use crate::api::threading::events;
use crate::api::threading::threads;
use crate::api::threading::{critical, tls};
pub(super) static EXPORTS: &[Export] = &[
    implemented!(
        "GetThreadPriority",
        "threading",
        "partial",
        "host nice level mapped to Windows relative priority",
        threads::priority
    ),
    implemented!(
        "CreateThread",
        "threading",
        "partial",
        "host threads with Windows TLS; suspended creation unsupported",
        threads::create
    ),
    implemented!(
        "DuplicateHandle",
        "threading",
        "partial",
        "local event/thread handles; same-access non-inheritable only",
        threads::duplicate
    ),
    implemented!(
        "GetExitCodeThread",
        "threading",
        "partial",
        "created thread exit code or STILL_ACTIVE",
        threads::get_exit_code
    ),
    implemented!(
        "CreateEventA",
        "threading",
        "partial",
        "unnamed events; security descriptors and inheritance unsupported",
        events::create_a
    ),
    implemented!(
        "CreateEventW",
        "threading",
        "partial",
        "unnamed events; security descriptors and inheritance unsupported",
        events::create_w
    ),
    implemented!(
        "SetEvent",
        "threading",
        "partial",
        "signals real manual/auto reset event",
        events::set
    ),
    implemented!(
        "ResetEvent",
        "threading",
        "partial",
        "clears real event",
        events::reset
    ),
    implemented!(
        "WaitForSingleObject",
        "threading",
        "partial",
        "event handles; finite/infinite waits",
        events::wait
    ),
    implemented!(
        "WaitForSingleObjectEx",
        "threading",
        "partial",
        "event handles; non-alertable waits",
        events::wait_ex
    ),
    implemented!(
        "CloseHandle",
        "threading",
        "partial",
        "event handles only",
        events::close
    ),
    implemented!(
        "TlsAlloc",
        "threading",
        "partial",
        "1088 process slots; caller value initially zero",
        tls::alloc
    ),
    implemented!(
        "TlsFree",
        "threading",
        "partial",
        "frees allocated dynamic TLS index",
        tls::free
    ),
    implemented!(
        "TlsGetValue",
        "threading",
        "partial",
        "thread-local values; Win32 last-error behavior",
        tls::get
    ),
    implemented!(
        "TlsSetValue",
        "threading",
        "partial",
        "stores thread-local value for allocated index",
        tls::set
    ),
    implemented!(
        "InitializeCriticalSection",
        "threading",
        "partial",
        "recursive host mutex; Windows contention fields not exposed",
        critical::initialize
    ),
    implemented!(
        "InitializeCriticalSectionAndSpinCount",
        "threading",
        "partial",
        "recursive host mutex; spin hint recorded",
        critical::initialize_spin
    ),
    implemented!(
        "EnterCriticalSection",
        "threading",
        "partial",
        "recursive host mutex; validates object lifetime",
        critical::enter
    ),
    implemented!(
        "LeaveCriticalSection",
        "threading",
        "partial",
        "recursive host mutex; validates ownership",
        critical::leave
    ),
    implemented!(
        "TryEnterCriticalSection",
        "threading",
        "partial",
        "nonblocking recursive host mutex",
        critical::try_enter
    ),
    implemented!(
        "DeleteCriticalSection",
        "threading",
        "partial",
        "retires initialized critical section",
        critical::delete
    ),
];
