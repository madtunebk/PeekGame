/* Relink batch A: A-variant file APIs, SearchPath, global atoms, DOS date/time,
   GetProcessId, CompareStringA. Exit 0 or the failing step. */
#include <windows.h>

void *memset(void *d, int c, size_t n) { unsigned char *p = d; while (n--) *p++ = (unsigned char)c; return d; }
#define DIR "Z:\\tmp\\peekgame-batch-a\\"

static int check(void) {
    CreateDirectoryA(DIR, NULL);
    DeleteFileA(DIR "src.txt"); DeleteFileA(DIR "dst.txt");
    DWORD n;
    HANDLE f = CreateFileA(DIR "src.txt", GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, 0, NULL);
    if (f == INVALID_HANDLE_VALUE || !WriteFile(f, "hello", 5, &n, NULL)) return 1;
    CloseHandle(f);
    /* GetFileAttributesExA */
    WIN32_FILE_ATTRIBUTE_DATA fad;
    if (!GetFileAttributesExA(DIR "src.txt", GetFileExInfoStandard, &fad) || fad.nFileSizeLow != 5) return 2;
    /* CopyFileA: copy, then fail-if-exists */
    if (!CopyFileA(DIR "src.txt", DIR "dst.txt", FALSE)) return 3;
    if (CopyFileA(DIR "src.txt", DIR "dst.txt", TRUE) || GetLastError() != ERROR_FILE_EXISTS) return 4;
    /* SearchPathA: find dst.txt in DIR */
    char found[260]; char *part = NULL;
    DWORD len = SearchPathA(DIR, "dst.txt", NULL, sizeof found, found, &part);
    if (len == 0 || part == NULL || lstrcmpA(part, "dst.txt")) return 5;
    if (SearchPathA(DIR, "nope.txt", NULL, sizeof found, found, NULL) != 0 || GetLastError() != ERROR_FILE_NOT_FOUND) return 6;
    /* GetVolumeInformationA */
    char fsname[16] = {0}; DWORD serial = 0, flags = 0, maxc = 0;
    if (!GetVolumeInformationA("Z:\\", NULL, 0, &serial, &maxc, &flags, fsname, sizeof fsname)) return 7;
    if (lstrcmpA(fsname, "NTFS")) return 8;
    /* Atoms */
    ATOM a = GlobalAddAtomA("PeekGameTestAtom");
    if (a < 0xC000) return 9;
    if (GlobalAddAtomA("PeekGameTestAtom") != a) return 10; /* same name, same atom (refcount) */
    if (GlobalFindAtomA("PeekGameTestAtom") != a) return 11;
    char name[64] = {0};
    if (GlobalGetAtomNameA(a, name, sizeof name) == 0 || lstrcmpA(name, "PeekGameTestAtom")) return 12;
    if (GlobalDeleteAtom(a) != 0) return 13;          /* one ref left (added twice) */
    if (GlobalFindAtomA("PeekGameTestAtom") != a) return 14;
    GlobalDeleteAtom(a);                               /* last ref */
    if (GlobalFindAtomA("PeekGameTestAtom") != 0) return 15;
    ATOM i = GlobalAddAtomA((LPCSTR)(ULONG_PTR)0x4242);   /* integer atom */
    if (i != 0x4242) return 16;
    /* DOS date/time round trip */
    SYSTEMTIME st = {2021, 6, 0, 15, 10, 30, 20, 0};
    FILETIME ft, ft2; WORD dd, dt;
    SystemTimeToFileTime(&st, &ft);
    if (!FileTimeToDosDateTime(&ft, &dd, &dt)) return 17;
    if (!DosDateTimeToFileTime(dd, dt, &ft2)) return 18;
    SYSTEMTIME back; FileTimeToSystemTime(&ft2, &back);
    if (back.wYear != 2021 || back.wMonth != 6 || back.wDay != 15 || back.wHour != 10 || back.wMinute != 30 || back.wSecond != 20) return 19;
    /* GetProcessId of the current process */
    if (GetProcessId(GetCurrentProcess()) != GetCurrentProcessId()) return 20;
    /* CompareStringA */
    if (CompareStringA(0, 0, "apple", -1, "banana", -1) != CSTR_LESS_THAN) return 21;
    if (CompareStringA(0, NORM_IGNORECASE, "ABC", -1, "abc", -1) != CSTR_EQUAL) return 22;
    DeleteFileA(DIR "src.txt"); DeleteFileA(DIR "dst.txt"); RemoveDirectoryA(DIR);
    return 0;
}
void entry(void) { ExitProcess(check()); }
