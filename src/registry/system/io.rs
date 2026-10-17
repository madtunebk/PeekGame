//! Export declarations for io. Implementation stays in api/.
use crate::api::filesystem::write;
use crate::registry::Export;

exports! {
    implemented!("GetQueuedCompletionStatus", "threading", "partial", "FIFO completion packets, timeout and close wakeup", crate::api::threading::iocp::get_status),
    implemented!("GetQueuedCompletionStatusEx", "threading", "partial", "batch dequeue and alertable waits", crate::api::threading::iocp::get_status_ex),
    implemented!("PostQueuedCompletionStatus", "threading", "partial", "opaque user completion packets", crate::api::threading::iocp::post),
    implemented!("SetFileCompletionNotificationModes", "file", "partial", "overlapped files: skip synchronous success and event flags", crate::api::threading::iocp::notification_modes),
    implemented!(
        "CreateIoCompletionPort",
        "system",
        "partial",
        "completion queues and overlapped file association",
        crate::api::threading::iocp::create
    ),
    implemented!(
        "WriteFile",
        "filesystem",
        "partial",
        "synchronous inherited standard streams only; overlapped I/O unsupported",
        write::write_file
    ),
}
