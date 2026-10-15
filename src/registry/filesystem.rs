use super::*;
use crate::api::filesystem::files;
use crate::api::filesystem::paths;
exports! {
    implemented!("SetNamedPipeHandleState", "file", "partial", "local Unix socket byte-stream pipe clients", crate::api::filesystem::pipes::set_state),
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
        "SetFileTime",
        "filesystem",
        "partial",
        "access and write times; creation time ignored",
        files::set_time
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
        "FindClose",
        "filesystem",
        "implemented",
        "search handles",
        files::find_close
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
        "GetDiskFreeSpaceExW",
        "filesystem",
        "implemented",
        "statvfs",
        files::disk_free_space_ex_w
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
