use super::*;
use crate::api::filesystem::files;
use crate::api::filesystem::paths;
exports! {
    implemented!("CreateDirectoryA", "filesystem", "partial", "host mkdir; UTF-8 path", files::create_directory_a),
    implemented!("DeleteFileA", "filesystem", "partial", "host unlink; UTF-8 path", files::delete_a),
    implemented!("RemoveDirectoryA", "filesystem", "partial", "host rmdir; UTF-8 path", files::remove_directory_a),

    implemented!("GetFileAttributesExA", "filesystem", "partial", "WIN32_FILE_ATTRIBUTE_DATA; UTF-8 path", files::attributes_ex_a),
    implemented!("CopyFileA", "filesystem", "partial", "host copy; fail-if-exists; UTF-8 paths", files::copy_a),
    implemented!("GetVolumeInformationA", "filesystem", "partial", "NTFS info for the root volume; ASCII", files::get_volume_information_a),
    implemented!("SearchPathW", "filesystem", "partial", "EXE dir then current dir (or the given path list); default extension", files::search_path_w),
    implemented!("SearchPathA", "filesystem", "partial", "EXE dir then current dir (or the given path list); default extension", files::search_path_a),

    export!("ntdll.dll", "NtWriteFile", "filesystem", "partial", "synchronous current-position file/pipe/standard stream writes; APC, event and explicit offsets unsupported", crate::api::filesystem::nt_io::write),
    export!("ntdll.dll", "NtReadFile", "filesystem", "partial", "synchronous current-position file/pipe/standard stream reads; APC, event and explicit offsets unsupported", crate::api::filesystem::nt_io::read),
    export!("ntdll.dll", "NtCreateFile", "filesystem", "partial", "NT path and disposition mapping to host files", crate::api::filesystem::nt_io::create),
    export!("ntdll.dll", "NtOpenFile", "filesystem", "partial", "NT open existing file", crate::api::filesystem::nt_io::open),
    export!("ntdll.dll", "NtCancelIoFileEx", "filesystem", "partial", "NT cancel pending asynchronous I/O requests", crate::api::filesystem::nt_io::cancel_io_file_ex),
    export!("ntdll.dll", "NtDeviceIoControlFile", "filesystem", "partial", "AFD socket polling and device IO control", crate::api::filesystem::nt_io::device_io_control_file),
    implemented!("MoveFileW", "filesystem", "partial", "atomic same-filesystem rename, no destination replacement, UTF-16 Windows paths", files::move_w),
    export!("ntdll.dll", "NtQueryDirectoryFile", "filesystem", "partial", "synchronous directory records, filtering and restart", crate::api::filesystem::directory::query),
    export!("ntdll.dll", "NtQueryInformationFile", "filesystem", "partial", "basic, standard, internal, position, network open file info", crate::api::filesystem::nt_io::query_information_file),
    export!("ntdll.dll", "NtQueryVolumeInformationFile", "filesystem", "partial", "disk device information; unsupported classes return STATUS_INVALID_INFO_CLASS", crate::api::filesystem::nt_io::query_volume_information_file),
    export!("ntdll.dll", "NtSetInformationFile", "filesystem", "partial", "basic, position, allocation, end-of-file info", crate::api::filesystem::nt_io::set_information_file),
    export!("ntdll.dll", "RtlNtStatusToDosError", "filesystem", "partial", "known file/IO statuses; unknown returns ERROR_MR_MID_NOT_FOUND", crate::api::filesystem::directory::status_to_dos),
    implemented!("SetNamedPipeHandleState", "file", "partial", "local Unix socket byte-stream pipe clients", crate::api::filesystem::pipes::set_state),
    implemented!("CreateNamedPipeW", "file", "implemented", "named pipe server backed by Unix domain socket", crate::api::filesystem::pipes::create_named_pipe_w),
    implemented!("CreateNamedPipeA", "file", "implemented", "named pipe server backed by Unix domain socket", crate::api::filesystem::pipes::create_named_pipe_a),
    implemented!("ConnectNamedPipe", "file", "implemented", "accepts client connection on named pipe server", crate::api::filesystem::pipes::connect_named_pipe),
    implemented!("DisconnectNamedPipe", "file", "implemented", "disconnects client from named pipe server", crate::api::filesystem::pipes::disconnect_named_pipe),
    implemented!("WaitNamedPipeW", "file", "implemented", "checks named pipe availability", crate::api::filesystem::pipes::wait_named_pipe_w),
    implemented!("WaitNamedPipeA", "file", "implemented", "checks named pipe availability", crate::api::filesystem::pipes::wait_named_pipe_a),
    implemented!("GetNamedPipeClientProcessId", "file", "implemented", "returns connected peer process ID", crate::api::filesystem::pipes::get_pipe_process_id),
    implemented!("GetNamedPipeServerProcessId", "file", "implemented", "returns connected peer process ID", crate::api::filesystem::pipes::get_pipe_process_id),
    implemented!("GetLogicalDrives", "filesystem", "partial", "returns drive C: availability mask", files::get_logical_drives),
    implemented!("GetVolumeInformationW", "filesystem", "partial", "NTFS filesystem info for root volume", files::get_volume_information_w),
    implemented!("SetFileInformationByHandle", "filesystem", "partial", "sets file length and allocation info", files::set_file_information_by_handle),
    implemented!("SetFileInformationByHandleW", "filesystem", "partial", "sets file length and allocation info", files::set_file_information_by_handle),
    implemented!(
        "CreateFileW",
        "filesystem",
        "partial",
        "host files on Z:; dispositions, append, directories with backup semantics; share modes ignored",
        files::create_w
    ),
    implemented!(
        "CreateFileA",
        "filesystem",
        "partial",
        "host files on Z:; dispositions, append, directories with backup semantics; share modes ignored",
        files::create_a
    ),
    implemented!(
        "ReadFile",
        "filesystem",
        "partial",
        "files, pipes, standard input; overlapped completes at once",
        files::read
    ),
    implemented!(
        "ReadFileScatter",
        "filesystem",
        "partial",
        "overlapped no-buffering file handles; page-sized segments, preadv at the OVERLAPPED offset, completes at once",
        files::read_scatter
    ),
    implemented!(
        "SetFilePointer",
        "filesystem",
        "implemented",
        "lseek with high part",
        files::set_pointer
    ),
    implemented!(
        "SetFilePointerEx",
        "filesystem",
        "implemented",
        "lseek",
        files::set_pointer_ex
    ),
    implemented!(
        "SetEndOfFile",
        "filesystem",
        "implemented",
        "truncate or extend at the file pointer",
        files::set_end_of_file
    ),
    implemented!(
        "GetFileSize",
        "filesystem",
        "implemented",
        "fstat size",
        files::size
    ),
    implemented!(
        "GetFileSizeEx",
        "filesystem",
        "implemented",
        "fstat size",
        files::size_ex
    ),
    implemented!(
        "FlushFileBuffers",
        "filesystem",
        "implemented",
        "fsync",
        files::flush
    ),
    implemented!(
        "GetFileAttributesExW",
        "filesystem",
        "partial",
        "stat: directory/normal/read-only/hidden, times, size",
        files::attributes_ex_w
    ),
    implemented!(
        "GetFileInformationByHandle",
        "filesystem",
        "implemented",
        "fstat; ctime as creation time",
        files::information_by_handle
    ),
    implemented!(
        "GetFileInformationByHandleEx",
        "filesystem",
        "partial",
        "basic, standard, attribute tag, id classes",
        files::information_by_handle_ex
    ),
    implemented!(
        "GetFileInformationByHandleExW",
        "filesystem",
        "partial",
        "basic, standard, attribute tag, id classes",
        files::information_by_handle_ex
    ),
    implemented!(
        "GetFileTime",
        "filesystem",
        "implemented",
        "creation, access and write times from handle metadata",
        files::get_time
    ),
    implemented!(
        "SetFileTime",
        "filesystem",
        "partial",
        "access and write times; creation time ignored",
        files::set_time
    ),
    implemented!(
        "LockFile",
        "filesystem",
        "partial",
        "advisory OFD lock (per open file); exclusive, fail-immediately",
        files::lock_file
    ),
    implemented!(
        "LockFileEx",
        "filesystem",
        "partial",
        "advisory OFD lock: shared/exclusive, fail-immediately or blocking wait; completes at once",
        files::lock_file_ex
    ),
    implemented!(
        "UnlockFile",
        "filesystem",
        "partial",
        "releases an OFD lock range (unlocked ranges are not reported as ERROR_NOT_LOCKED)",
        files::unlock_file
    ),
    implemented!(
        "UnlockFileEx",
        "filesystem",
        "partial",
        "releases an OFD lock range at the OVERLAPPED offset",
        files::unlock_file_ex
    ),
    implemented!(
        "FindFirstFileExW",
        "filesystem",
        "partial",
        "wildcards, directories-only search; no short names",
        files::find_first_ex_w
    ),
    implemented!(
        "FindFirstFileW",
        "filesystem",
        "partial",
        "wildcards; no short names",
        files::find_first_w
    ),
    implemented!(
        "FindNextFileW",
        "filesystem",
        "implemented",
        "next match",
        files::find_next_w
    ),
    implemented!(
        "FindFirstFileExA",
        "filesystem",
        "partial",
        "ANSI wrapper over FindFirstFileExW",
        files::find_first_ex_a
    ),
    implemented!(
        "FindFirstFileA",
        "filesystem",
        "partial",
        "ANSI wrapper over FindFirstFileW",
        files::find_first_a
    ),
    implemented!(
        "FindNextFileA",
        "filesystem",
        "implemented",
        "ANSI wrapper over FindNextFileW",
        files::find_next_a
    ),
    implemented!(
        "FindClose",
        "filesystem",
        "implemented",
        "search handles",
        files::find_close
    ),
    implemented!(
        "FindFirstChangeNotificationW",
        "filesystem",
        "implemented",
        "creates change notification event handle",
        files::find_first_change_notification_w
    ),
    implemented!(
        "FindFirstChangeNotificationA",
        "filesystem",
        "implemented",
        "creates change notification event handle",
        files::find_first_change_notification_a
    ),
    implemented!(
        "FindNextChangeNotification",
        "filesystem",
        "implemented",
        "resets change notification handle",
        files::find_next_change_notification
    ),
    implemented!(
        "FindCloseChangeNotification",
        "filesystem",
        "implemented",
        "stops change notification monitoring",
        files::find_close_change_notification
    ),
    implemented!(
        "CreateDirectoryW",
        "filesystem",
        "partial",
        "mkdir; security descriptors ignored",
        files::create_directory_w
    ),
    implemented!(
        "DeleteFileW",
        "filesystem",
        "implemented",
        "unlink",
        files::delete_w
    ),
    implemented!(
        "RemoveDirectoryW",
        "filesystem",
        "implemented",
        "rmdir",
        files::remove_directory_w
    ),
    implemented!(
        "MoveFileExW",
        "filesystem",
        "partial",
        "rename, MOVEFILE_REPLACE_EXISTING",
        files::move_ex_w
    ),
    implemented!(
        "ReplaceFileW",
        "filesystem",
        "partial",
        "replace without backup",
        files::replace_w
    ),
    implemented!(
        "CopyFileW",
        "filesystem",
        "implemented",
        "copy, fail-if-exists",
        files::copy_w
    ),
    implemented!("CreateFile2", "filesystem", "partial", "CreateFileW with CREATEFILE2_EXTENDED_PARAMETERS attributes/flags/security/template", files::create_2),
    implemented!("CopyFile2", "filesystem", "partial", "COPY_FILE_FAIL_IF_EXISTS; progress/cancel/other flags HRESULT ERROR_NOT_SUPPORTED", files::copy_2),
    implemented!("CreateSymbolicLinkW", "filesystem", "implemented", "host symlink; relative targets stay relative", files::create_symbolic_link_w),
    implemented!("CreateHardLinkW", "filesystem", "implemented", "host hard link", files::create_hard_link_w),
    implemented!(
        "SetFileAttributesW",
        "filesystem",
        "partial",
        "read-only bit; hidden/system unsupported",
        files::set_attributes_w
    ),
    implemented!(
        "GetTempFileNameW",
        "filesystem",
        "implemented",
        "PREFIX+hex.tmp, created when unique is 0",
        files::temp_file_name_w
    ),
    implemented!(
        "GetDiskFreeSpaceA",
        "filesystem",
        "implemented",
        "statvfs cluster and sector metrics",
        files::disk_free_space_a
    ),
    implemented!(
        "GetDiskFreeSpaceW",
        "filesystem",
        "implemented",
        "statvfs cluster and sector metrics",
        files::disk_free_space_w
    ),
    implemented!(
        "GetDiskFreeSpaceExW",
        "filesystem",
        "implemented",
        "statvfs",
        files::disk_free_space_ex_w
    ),
    implemented!(
        "GetDriveTypeA",
        "filesystem",
        "partial",
        "Z: fixed, other drives absent",
        files::drive_type_a
    ),
    implemented!(
        "GetDriveTypeW",
        "filesystem",
        "partial",
        "Z: fixed, other drives absent",
        files::drive_type_w
    ),
    implemented!(
        "CreatePipe",
        "filesystem",
        "partial",
        "anonymous pipe; security ignored",
        files::create_pipe
    ),
    implemented!(
        "PeekNamedPipe",
        "filesystem",
        "partial",
        "bytes available only",
        files::peek_named_pipe
    ),
    implemented!(
        "CancelIo",
        "filesystem",
        "implemented",
        "nothing is ever pending",
        files::cancel_io
    ),
    implemented!(
        "CancelIoEx",
        "filesystem",
        "implemented",
        "nothing is ever pending: ERROR_NOT_FOUND",
        files::cancel_io_ex
    ),
    implemented!(
        "GetOverlappedResult",
        "filesystem",
        "implemented",
        "transfers complete at once",
        files::overlapped_result
    ),
    implemented!(
        "GetOverlappedResultEx",
        "filesystem",
        "implemented",
        "alertable overlapped result check",
        files::overlapped_result_ex
    ),
    implemented!(
        "ReadFileEx",
        "filesystem",
        "implemented",
        "overlapped read with APC completion routine",
        files::read_ex
    ),
    implemented!(
        "WriteFileEx",
        "filesystem",
        "implemented",
        "overlapped write with APC completion routine",
        files::write_ex
    ),
    implemented!(
        "CancelSynchronousIo",
        "filesystem",
        "implemented",
        "nothing is ever pending: ERROR_NOT_FOUND",
        files::cancel_synchronous_io
    ),
    implemented!(
        "GetFinalPathNameByHandleW",
        "filesystem",
        "partial",
        "returns \\\\?\\ Windows path for open file handles",
        files::final_path_name_by_handle_w
    ),
    implemented!(
        "GetLongPathNameW",
        "filesystem",
        "partial",
        "existing paths without synthetic 8.3 aliases",
        paths::long_w
    ),
    implemented!(
        "GetCurrentDirectoryW",
        "filesystem",
        "partial",
        "host current directory through Z: root mapping",
        paths::current_w
    ),
    implemented!(
        "SetCurrentDirectoryW",
        "filesystem",
        "partial",
        "host current directory through Z: root mapping",
        paths::set_current_w
    ),
    implemented!(
        "GetFullPathNameW",
        "filesystem",
        "partial",
        "lexical absolute path through Z: mapping",
        paths::full_w
    ),
    implemented!(
        "GetFullPathNameA",
        "filesystem",
        "partial",
        "lexical absolute path through Z: mapping",
        paths::full_a
    ),
    implemented!(
        "GetModuleFileNameW",
        "filesystem",
        "partial",
        "EXE and loaded PE modules, Z: paths",
        crate::api::system::info::module_file_name_w
    ),
    implemented!(
        "GetFileAttributesW",
        "filesystem",
        "partial",
        "host file/directory/read-only attributes",
        paths::attributes_w
    ),
    implemented!(
        "GetFileAttributesA",
        "filesystem",
        "partial",
        "host file/directory/read-only attributes; UTF-8",
        paths::attributes_a
    ),
}
