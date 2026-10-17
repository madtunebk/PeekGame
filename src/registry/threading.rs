use super::*;
use crate::api::threading::apc;
use crate::api::threading::events;
use crate::api::threading::objects;
use crate::api::threading::suspend;
use crate::api::threading::threads;
use crate::api::threading::timers;
use crate::api::threading::{address, fls, once, slist, srw};
use crate::api::threading::{critical, tls};
exports! {
    implemented!("GetThreadTimes", "threading", "partial", "real task creation/CPU times; retained accounting after runtime thread exit", crate::api::threading::thread_times::get),
    export!("avrt.dll", "AvSetMmThreadCharacteristicsW", "threading", "partial", "MMCSS unavailable: NULL/ERROR_NOT_SUPPORTED; normal scheduling preserved", crate::api::threading::avrt::characteristics),
    export!("avrt.dll", "AvSetMmThreadCharacteristicsA", "threading", "partial", "MMCSS unavailable: NULL/ERROR_NOT_SUPPORTED; normal scheduling preserved", crate::api::threading::avrt::characteristics),
    export!("avrt.dll", "AvRevertMmThreadCharacteristics", "threading", "partial", "no registered MMCSS handles: ERROR_INVALID_HANDLE", crate::api::threading::avrt::revert),
    export!("avrt.dll", "AvSetMmThreadPriority", "threading", "partial", "no registered MMCSS handles: ERROR_INVALID_HANDLE", crate::api::threading::avrt::priority),

    implemented!("OpenMutexW", "threading", "partial", "process-local named mutexes with independent handle lifetimes", crate::api::threading::objects::open_mutex_w),
    implemented!("ExitThread", "threading", "partial", "Windows-created threads; TLS detach and exit status", crate::host::thread_entry::exit),
    implemented!("FreeLibraryAndExitThread", "threading", "partial", "release module reference then terminate calling Windows thread", crate::host::thread_entry::free_and_exit),
    implemented!(
        "SuspendThread",
        "threading",
        "partial",
        "stops the thread in a signal handler",
        suspend::suspend
    ),
    implemented!(
        "GetThreadContext",
        "threading",
        "partial",
        "suspended threads",
        suspend::get_context
    ),
    implemented!(
        "SetThreadContext",
        "threading",
        "partial",
        "suspended threads; applied on resume",
        suspend::set_context
    ),
    implemented!(
        "Wow64SuspendThread",
        "threading",
        "partial",
        "as SuspendThread",
        suspend::suspend
    ),
    implemented!(
        "CreateWaitableTimerExW",
        "threading",
        "partial",
        "event signalled by a timer thread",
        timers::create_ex_w
    ),
    implemented!(
        "CreateWaitableTimerW",
        "threading",
        "partial",
        "event signalled by a timer thread",
        timers::create_w
    ),
    implemented!(
        "CreateWaitableTimerA",
        "threading",
        "partial",
        "event signalled by a timer thread",
        timers::create_w
    ),
    implemented!(
        "SetWaitableTimer",
        "threading",
        "partial",
        "relative/absolute due time, period; no completion routines",
        timers::set
    ),
    implemented!(
        "SetWaitableTimerEx",
        "threading",
        "partial",
        "as SetWaitableTimer",
        timers::set_ex
    ),
    implemented!(
        "CancelWaitableTimer",
        "threading",
        "partial",
        "complete",
        timers::cancel
    ),
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
        suspend::resume
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
        "ConvertThreadToFiber",
        "threading",
        "partial",
        "converts calling thread to fiber and sets gs:0x20",
        threads::convert_thread_to_fiber
    ),
    implemented!(
        "ConvertThreadToFiberEx",
        "threading",
        "partial",
        "converts calling thread to fiber and sets gs:0x20",
        threads::convert_thread_to_fiber_ex
    ),
    implemented!(
        "ConvertFiberToThread",
        "threading",
        "partial",
        "clears fiber state at gs:0x20",
        threads::convert_fiber_to_thread
    ),
    implemented!(
        "OpenEventW",
        "threading",
        "partial",
        "process-local named events",
        events::open_w
    ),
    implemented!(
        "OpenEventA",
        "threading",
        "partial",
        "process-local named events",
        events::open_a
    ),
    implemented!(
        "CreateEventA",
        "threading",
        "partial",
        "unnamed and process-local named events; security attributes accepted (inheritance has no effect)",
        events::create_a
    ),
    implemented!(
        "CreateEventW",
        "threading",
        "partial",
        "unnamed and process-local named events; security attributes accepted (inheritance has no effect)",
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
        "SetCriticalSectionSpinCount",
        "threading",
        "partial",
        "recursive host mutex; update spin hint",
        critical::set_spin_count
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
    implemented!(
        "FlushProcessWriteBuffers",
        "threading",
        "implemented",
        "SeqCst memory fence",
        threads::flush_process_write_buffers
    ),
    implemented!(
        "GetCurrentProcessorNumber",
        "threading",
        "implemented",
        "sched_getcpu",
        threads::current_processor_number
    ),
}
