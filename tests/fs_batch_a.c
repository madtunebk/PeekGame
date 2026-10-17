/* Filesystem audit batch A: read-only create dispositions, byte-range locks, no IOCP
   packet for an immediately failed read, NtCreateFile RootDirectory/options/info. */
#include <windows.h>
#include <winternl.h>

void *memcpy(void *d, const void *s, size_t n) { unsigned char *p = d; const unsigned char *q = s; while (n--) *p++ = *q++; return d; }
void *memset(void *d, int c, size_t n) { unsigned char *p = d; while (n--) *p++ = (unsigned char)c; return d; }
#define DIR L"Z:\\tmp\\peekgame-fs-a\\"
static DWORD size_of(const WCHAR *path) {
    WIN32_FILE_ATTRIBUTE_DATA d;
    return GetFileAttributesExW(path, GetFileExInfoStandard, &d) ? d.nFileSizeLow : 0xFFFFFFFF;
}
static int check(void) {
    CreateDirectoryW(DIR, NULL);
    DeleteFileW(DIR L"ro1.txt"); DeleteFileW(DIR L"ro2.txt"); DeleteFileW(DIR L"lock.bin"); DeleteFileW(DIR L"child.txt");
    /* F5 */
    HANDLE f = CreateFileW(DIR L"ro1.txt", GENERIC_READ, 0, NULL, OPEN_ALWAYS, 0, NULL);
    if (f == INVALID_HANDLE_VALUE || GetLastError() != 0) return 1;
    CloseHandle(f);
    f = CreateFileW(DIR L"ro2.txt", GENERIC_READ, 0, NULL, CREATE_NEW, 0, NULL);
    if (f == INVALID_HANDLE_VALUE) return 2;
    CloseHandle(f);
    if (CreateFileW(DIR L"ro2.txt", GENERIC_READ, 0, NULL, CREATE_NEW, 0, NULL) != INVALID_HANDLE_VALUE || GetLastError() != ERROR_FILE_EXISTS) return 3;
    DWORD n;
    f = CreateFileW(DIR L"ro2.txt", GENERIC_WRITE, 0, NULL, OPEN_EXISTING, 0, NULL);
    WriteFile(f, "data", 4, &n, NULL); CloseHandle(f);
    f = CreateFileW(DIR L"ro2.txt", GENERIC_READ, 0, NULL, CREATE_ALWAYS, 0, NULL);
    if (f == INVALID_HANDLE_VALUE || GetLastError() != ERROR_ALREADY_EXISTS) return 4;
    CloseHandle(f);
    if (size_of(DIR L"ro2.txt") != 0) return 5;
    if (CreateFileW(DIR L"ro2.txt", GENERIC_READ, 0, NULL, TRUNCATE_EXISTING, 0, NULL) != INVALID_HANDLE_VALUE || GetLastError() != ERROR_INVALID_PARAMETER) return 6;
    /* F6 */
    HANDLE a = CreateFileW(DIR L"lock.bin", GENERIC_READ | GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE, NULL, CREATE_ALWAYS, 0, NULL);
    HANDLE b = CreateFileW(DIR L"lock.bin", GENERIC_READ | GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE, NULL, OPEN_EXISTING, 0, NULL);
    OVERLAPPED ov = {0};
    if (!LockFileEx(a, LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY, 0, 100, 0, &ov)) return 10;
    if (LockFileEx(b, LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY, 0, 10, 0, &ov) || GetLastError() != ERROR_LOCK_VIOLATION) return 11;
    if (LockFileEx(b, LOCKFILE_FAIL_IMMEDIATELY, 0, 10, 0, &ov) || GetLastError() != ERROR_LOCK_VIOLATION) return 12;
    ov.Offset = 200;
    if (!LockFileEx(b, LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY, 0, 10, 0, &ov)) return 13; /* other range */
    ov.Offset = 0;
    if (!UnlockFileEx(a, 0, 100, 0, &ov)) return 14;
    if (!LockFileEx(b, LOCKFILE_FAIL_IMMEDIATELY, 0, 50, 0, &ov) || !LockFileEx(a, LOCKFILE_FAIL_IMMEDIATELY, 0, 50, 0, &ov)) return 15; /* shared + shared */
    if (LockFile(a, 0, 0, 10, 0) || GetLastError() != ERROR_LOCK_VIOLATION) return 16;
    UnlockFileEx(a, 0, 50, 0, &ov); UnlockFileEx(b, 0, 50, 0, &ov);
    if (!LockFile(a, 0, 0, 0, 0)) return 17; /* zero length: nothing locked */
    HANDLE dup;
    DuplicateHandle(GetCurrentProcess(), a, GetCurrentProcess(), &dup, 0, FALSE, DUPLICATE_SAME_ACCESS);
    if (!LockFile(a, 500, 0, 10, 0) || !UnlockFile(dup, 500, 0, 10, 0) || !LockFile(b, 500, 0, 10, 0)) return 18;
    CloseHandle(dup); CloseHandle(a); CloseHandle(b);
    /* An overlapped read at EOF fails with ERROR_HANDLE_EOF (its completion packet is
       covered by test_iocp.py). */
    f = CreateFileW(DIR L"ro1.txt", GENERIC_READ, FILE_SHARE_READ, NULL, OPEN_EXISTING, FILE_FLAG_OVERLAPPED, NULL);
    HANDLE port = CreateIoCompletionPort(f, NULL, 5, 0);
    char buffer[16];
    OVERLAPPED eof = {0};
    eof.Offset = 4096;
    if (ReadFile(f, buffer, sizeof buffer, &n, &eof) || GetLastError() != ERROR_HANDLE_EOF) return 20;
    ULONG_PTR key; LPOVERLAPPED got;
    if (!GetQueuedCompletionStatus(port, &n, &key, &got, 50) && got != &eof) return 21;
    CloseHandle(f); CloseHandle(port);
    /* F10: NtCreateFile relative to a directory handle */
    HANDLE dir = CreateFileW(DIR, GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE, NULL, OPEN_EXISTING, FILE_FLAG_BACKUP_SEMANTICS, NULL);
    if (dir == INVALID_HANDLE_VALUE) return 30;
    UNICODE_STRING name = {sizeof L"child.txt" - 2, sizeof L"child.txt", (PWSTR)L"child.txt"};
    OBJECT_ATTRIBUTES oa;
    InitializeObjectAttributes(&oa, &name, 0, dir, NULL);
    IO_STATUS_BLOCK io;
    HANDLE child;
    if (NtCreateFile(&child, GENERIC_WRITE, &oa, &io, NULL, 0, 0, 3 /*FILE_OPEN_IF*/, 0x40, NULL, 0) != 0 || io.Information != 2) return 31;
    CloseHandle(child);
    if (GetFileAttributesW(DIR L"child.txt") == INVALID_FILE_ATTRIBUTES) return 32;
    if (NtCreateFile(&child, GENERIC_READ, &oa, &io, NULL, 0, 0, 3, 0, NULL, 0) != 0 || io.Information != 1) return 33;
    CloseHandle(child);
    if (NtCreateFile(&child, GENERIC_READ, &oa, &io, NULL, 0, 0, 1, 1 /*FILE_DIRECTORY_FILE*/, NULL, 0) != (NTSTATUS)0xC0000103) return 34;
    UNICODE_STRING self = {0, 0, (PWSTR)L""};
    UNICODE_STRING tmpdir = {sizeof L"\\??\\Z:\\tmp" - 2, sizeof L"\\??\\Z:\\tmp", (PWSTR)L"\\??\\Z:\\tmp"};
    OBJECT_ATTRIBUTES absolute;
    InitializeObjectAttributes(&absolute, &tmpdir, 0, NULL, NULL);
    if (NtCreateFile(&child, GENERIC_READ, &absolute, &io, NULL, 0, 0, 1, 0x40 /*NON_DIRECTORY*/, NULL, 0) != (NTSTATUS)0xC00000BA) return 35;
    if (NtCreateFile(&child, GENERIC_READ, &absolute, &io, NULL, 0, 0, 1, 1, NULL, 0) != 0) return 36;
    CloseHandle(child);
    if (NtCreateFile(&child, GENERIC_READ, &oa, &io, NULL, 0, 0, 9, 0, NULL, 0) != (NTSTATUS)0xC000000D) return 37;
    (void)self;
    CloseHandle(dir);
    DeleteFileW(DIR L"ro1.txt"); DeleteFileW(DIR L"ro2.txt"); DeleteFileW(DIR L"lock.bin"); DeleteFileW(DIR L"child.txt");
    RemoveDirectoryW(DIR);
    return 0;
}
void entry(void) { ExitProcess(check()); }
