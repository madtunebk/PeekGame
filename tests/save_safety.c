/* Save-safety regressions: rename/delete/basic info through handles (kernel32 and NT),
   FILE_FLAG_DELETE_ON_CLOSE, SetFileTime special values. Exit 0 or the failing step. */
#include <windows.h>
#include <winternl.h>

#define NtSet(h, io, p, n, c) NtSetInformationFile(h, io, p, n, (FILE_INFORMATION_CLASS)(c))
void *memcpy(void *d, const void *s, size_t n) { unsigned char *p = d; const unsigned char *q = s; while (n--) *p++ = *q++; return d; }
void *memset(void *d, int c, size_t n) { unsigned char *p = d; while (n--) *p++ = (unsigned char)c; return d; }
#define DIR L"Z:\\tmp\\peekgame-save-safety\\"

static BOOL exists(const WCHAR *path) { return GetFileAttributesW(path) != INVALID_FILE_ATTRIBUTES; }
static HANDLE make(const WCHAR *path, const char *text, DWORD flags) {
    HANDLE f = CreateFileW(path, GENERIC_READ | GENERIC_WRITE | DELETE, FILE_SHARE_READ | FILE_SHARE_DELETE,
                           NULL, CREATE_ALWAYS, flags, NULL);
    DWORD n;
    if (f != INVALID_HANDLE_VALUE && text) WriteFile(f, text, lstrlenA(text), &n, NULL);
    return f;
}
static int content(const WCHAR *path, const char *expected) {
    char buffer[64] = {0};
    DWORD n = 0;
    HANDLE f = CreateFileW(path, GENERIC_READ, FILE_SHARE_READ, NULL, OPEN_EXISTING, 0, NULL);
    if (f == INVALID_HANDLE_VALUE) return 0;
    ReadFile(f, buffer, sizeof buffer - 1, &n, NULL);
    CloseHandle(f);
    return lstrcmpA(buffer, expected) == 0;
}
static BOOL rename_to(HANDLE f, const WCHAR *to, BOOL replace) {
    BYTE storage[sizeof(FILE_RENAME_INFO) + 600] = {0};
    FILE_RENAME_INFO *info = (FILE_RENAME_INFO *)storage;
    info->ReplaceIfExists = replace;
    info->FileNameLength = lstrlenW(to) * 2;
    CopyMemory(info->FileName, to, info->FileNameLength);
    return SetFileInformationByHandle(f, FileRenameInfo, info, sizeof storage);
}

static int check(void) {
    CreateDirectoryW(DIR, NULL);
    DeleteFileW(DIR L"save.dat"); DeleteFileW(DIR L"save.tmp"); DeleteFileW(DIR L"nt.dat");
    /* 1. atomic save: temp renamed over the old save through the open handle */
    HANDLE old = make(DIR L"save.dat", "old", 0);
    CloseHandle(old);
    HANDLE tmp = make(DIR L"save.tmp", "new save", 0);
    if (tmp == INVALID_HANDLE_VALUE) return 1;
    if (rename_to(tmp, DIR L"save.dat", FALSE) || GetLastError() != ERROR_ALREADY_EXISTS) return 2;
    if (!rename_to(tmp, DIR L"save.dat", TRUE)) return 3;
    DWORD n;
    if (!WriteFile(tmp, "!", 1, &n, NULL)) return 4; /* the handle follows the rename */
    CloseHandle(tmp);
    if (exists(DIR L"save.tmp") || !content(DIR L"save.dat", "new save!")) return 5;
    /* 2. delete disposition, cancelled and real */
    HANDLE f = make(DIR L"gone.dat", "x", 0);
    FILE_DISPOSITION_INFO d = {TRUE};
    if (!SetFileInformationByHandle(f, FileDispositionInfo, &d, sizeof d)) return 6;
    d.DeleteFile = FALSE;
    if (!SetFileInformationByHandle(f, FileDispositionInfo, &d, sizeof d)) return 7;
    CloseHandle(f);
    if (!exists(DIR L"gone.dat")) return 8;
    f = CreateFileW(DIR L"gone.dat", DELETE, FILE_SHARE_DELETE, NULL, OPEN_EXISTING, 0, NULL);
    d.DeleteFile = TRUE;
    if (!SetFileInformationByHandle(f, FileDispositionInfo, &d, sizeof d)) return 9;
    CloseHandle(f);
    if (exists(DIR L"gone.dat")) return 10;
    /* 3. FILE_FLAG_DELETE_ON_CLOSE survives a duplicate until the last close */
    f = make(DIR L"temp.dat", "t", FILE_FLAG_DELETE_ON_CLOSE);
    HANDLE dup;
    if (!DuplicateHandle(GetCurrentProcess(), f, GetCurrentProcess(), &dup, 0, FALSE, DUPLICATE_SAME_ACCESS)) return 11;
    CloseHandle(f);
    if (!exists(DIR L"temp.dat")) return 12;
    CloseHandle(dup);
    if (exists(DIR L"temp.dat")) return 13;
    /* 4. SetFileTime: "don't change" values and a time before 1970 */
    f = CreateFileW(DIR L"save.dat", GENERIC_READ | GENERIC_WRITE, 0, NULL, OPEN_EXISTING, 0, NULL);
    FILETIME keep = {0xFFFFFFFF, 0xFFFFFFFF}, old1965, written;
    if (!SetFileTime(f, NULL, &keep, &keep)) return 14;
    ULARGE_INTEGER t; t.QuadPart = 0x0170C53F5C2E8000ULL; /* 1965-01-01 */
    old1965.dwLowDateTime = t.LowPart; old1965.dwHighDateTime = t.HighPart;
    if (!SetFileTime(f, NULL, NULL, &old1965)) return 15;
    if (!GetFileTime(f, NULL, NULL, &written) || written.dwHighDateTime != old1965.dwHighDateTime ||
        written.dwLowDateTime / 10000000 != old1965.dwLowDateTime / 10000000) return 16;
    /* 5. FileBasicInfo: read-only on and off */
    FILE_BASIC_INFO basic = {0};
    basic.FileAttributes = FILE_ATTRIBUTE_READONLY;
    if (!SetFileInformationByHandle(f, FileBasicInfo, &basic, sizeof basic)) return 17;
    if (!(GetFileAttributesW(DIR L"save.dat") & FILE_ATTRIBUTE_READONLY)) return 18;
    basic.FileAttributes = FILE_ATTRIBUTE_NORMAL;
    if (!SetFileInformationByHandle(f, FileBasicInfo, &basic, sizeof basic)) return 19;
    if (GetFileAttributesW(DIR L"save.dat") & FILE_ATTRIBUTE_READONLY) return 20;
    /* 6. NT: rename with an NT path, unknown class, delete */
    IO_STATUS_BLOCK io;
    BYTE storage[600] = {0};
    const WCHAR target[] = L"\\??\\Z:\\tmp\\peekgame-save-safety\\nt.dat";
    *(BOOLEAN *)storage = FALSE;
    *(ULONG *)(storage + 16) = sizeof target - 2;
    CopyMemory(storage + 20, target, sizeof target - 2);
    if (NtSet(f, &io, storage, sizeof storage, 10) != 0) return 21;
    if (exists(DIR L"save.dat") || !exists(DIR L"nt.dat")) return 22;
    if (NtSet(f, &io, storage, sizeof storage, 99) != (NTSTATUS)0xC0000003) return 23;
    BOOLEAN del = TRUE;
    if (NtSet(f, &io, &del, 1, 13) != 0) return 24;
    CloseHandle(f);
    if (exists(DIR L"nt.dat")) return 25;
    RemoveDirectoryW(DIR);
    return 0;
}
void entry(void) { ExitProcess(check()); }
