use super::*;
use crate::api::threading::apc;
use crate::api::threading::events;
use crate::api::threading::objects;
use crate::api::threading::threads;
use crate::api::threading::{address, fls, once, slist, srw};
use crate::api::threading::{critical, tls};
exports! {
    implemented!(
        "QueueUserAPC",
        "threading",
        "partial",
        "runs in the thread's next alertable wait",
        apc::queue
    ),
    implemented!(
        "OpenThread",
        "threading",
        "partial",
        "threads of this process by ID",
        threads::open
    ),
    implemented!(
        "GetThreadId",
        "threading",
        "partial",
        "thread handles and the current-thread pseudo handle",
        threads::thread_id_of
    ),
    implemented!(
        "ResumeThread",
        "threading",
        "partial",
        "starts CREATE_SUSPENDED threads; SuspendThread pending",
        threads::resume
    ),
    implemented!(
        "CreateSemaphoreExW",
        "threading",
        "partial",
        "process-local names; security ignored",
        objects::create_semaphore_ex_w
    ),
    implemented!(
        "CreateSemaphoreW",
        "threading",
        "partial",
        "process-local names; security ignored",
        objects::create_semaphore_w
    ),
    implemented!(
        "ReleaseSemaphore",
        "threading",
        "implemented",
        "count with maximum check",
        objects::release_semaphore
    ),
    implemented!(
        "CreateMutexW",
        "threading",
        "partial",
        "recursive ownership; process-local names",
        objects::create_mutex_w
    ),
    implemented!(
        "CreateMutexA",
        "threading",
        "partial",
        "recursive ownership; process-local names",
        objects::create_mutex_a
    ),
    implemented!(
        "ReleaseMutex",
        "threading",
        "implemented",
        "owner thread only",
        objects::release_mutex
    ),
    implemented!(
        "CreateEventExW",
        "threading",
        "partial",
        "manual/initial flags; unnamed only",
        objects::create_event_ex_w
    ),
    implemented!(
        "WaitForMultipleObjects",
        "threading",
        "partial",
        "events, threads, semaphores, mutexes; any or all",
        objects::wait_multiple
    ),
    implemented!(
        "WaitForMultipleObjectsEx",
        "threading",
        "partial",
        "as WaitForMultipleObjects; no APCs",
        objects::wait_multiple_ex
    ),
    implemented!(
        "InitializeSListHead",
        "threading",
        "implemented",
        "16-byte aligned header",
        slist::initialize
    ),
    implemented!(
        "InterlockedPushEntrySList",
        "threading",
        "implemented",
        "lock-free cmpxchg16b with ABA sequence",
        slist::push
    ),
    implemented!(
        "InterlockedPopEntrySList",
        "threading",
        "implemented",
        "lock-free cmpxchg16b with ABA sequence",
        slist::pop
    ),
    implemented!(
        "InterlockedFlushSList",
        "threading",
        "implemented",
        "lock-free cmpxchg16b with ABA sequence",
        slist::flush
    ),
    implemented!(
        "InterlockedPushListSListEx",
        "threading",
        "implemented",
        "pre-linked chain",
        slist::push_list
    ),
    implemented!(
        "QueryDepthSList",
        "threading",
        "implemented",
        "depth field",
        slist::depth
    ),
    implemented!(
        "FlsAlloc",
        "threading",
        "partial",
        "per-thread slots; callbacks at thread exit and FlsFree (caller thread)",
        fls::alloc
    ),
    implemented!(
        "FlsSetValue",
        "threading",
        "implemented",
        "per-thread value",
        fls::set
    ),
    implemented!(
        "FlsGetValue",
        "threading",
        "implemented",
        "per-thread value",
        fls::get
    ),
    implemented!(
        "FlsGetValue2",
        "threading",
        "implemented",
        "per-thread value, last error untouched",
        fls::get2
    ),
    implemented!(
        "FlsFree",
        "threading",
        "partial",
        "runs the callback for the caller thread only",
        fls::free
    ),
    implemented!(
        "InitializeSRWLock",
        "threading",
        "implemented",
        "state lives in the lock word",
        srw::initialize
    ),
    implemented!(
        "AcquireSRWLockExclusive",
        "threading",
        "implemented",
        "futex on the lock word",
        srw::acquire_exclusive
    ),
    implemented!(
        "AcquireSRWLockShared",
        "threading",
        "implemented",
        "futex on the lock word",
        srw::acquire_shared
    ),
    implemented!(
        "ReleaseSRWLockExclusive",
        "threading",
        "implemented",
        "futex on the lock word",
        srw::release_exclusive
    ),
    implemented!(
        "ReleaseSRWLockShared",
        "threading",
        "implemented",
        "futex on the lock word",
        srw::release_shared
    ),
    implemented!(
        "TryAcquireSRWLockExclusive",
        "threading",
        "implemented",
        "lock word",
        srw::try_acquire_exclusive
    ),
    implemented!(
        "TryAcquireSRWLockShared",
        "threading",
        "implemented",
        "lock word",
        srw::try_acquire_shared
    ),
    implemented!(
        "InitializeConditionVariable",
        "threading",
        "implemented",
        "wake sequence word",
        srw::initialize_condition
    ),
    implemented!(
        "WakeConditionVariable",
        "threading",
        "implemented",
        "futex wake",
        srw::wake_condition
    ),
    implemented!(
        "WakeAllConditionVariable",
        "threading",
        "implemented",
        "futex wake",
        srw::wake_all_condition
    ),
    implemented!(
        "SleepConditionVariableSRW",
        "threading",
        "implemented",
        "exclusive or shared SRW, timeout",
        srw::sleep_condition_srw
    ),
    implemented!(
        "SleepConditionVariableCS",
        "threading",
        "implemented",
        "critical sections, timeout",
        srw::sleep_condition_cs
    ),
    implemented!(
        "InitializeCriticalSectionEx",
        "threading",
        "partial",
        "as InitializeCriticalSectionAndSpinCount; flags ignored",
        srw::initialize_critical_ex
    ),
    implemented!(
        "InitOnceInitialize",
        "threading",
        "implemented",
        "resets the once field",
        once::initialize
    ),
    implemented!(
        "InitOnceBeginInitialize",
        "threading",
        "implemented",
        "synchronous, asynchronous and check-only",
        once::begin
    ),
    implemented!(
        "InitOnceComplete",
        "threading",
        "implemented",
        "complete or failed",
        once::complete
    ),
    implemented!(
        "InitOnceExecuteOnce",
        "threading",
        "implemented",
        "callback once per field",
        once::execute
    ),
    implemented!(
        "WaitOnAddress",
        "threading",
        "implemented",
        "1/2/4/8-byte comparison, timeout",
        address::wait
    ),
    implemented!(
        "WakeByAddressSingle",
        "threading",
        "implemented",
        "one waiter on the exact address",
        address::wake_single
    ),
    implemented!(
        "WakeByAddressAll",
        "threading",
        "implemented",
        "all waiters on the exact address",
        address::wake_all
    ),
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
}
