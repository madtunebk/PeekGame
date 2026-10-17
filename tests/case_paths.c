/* Case-insensitive Windows paths and device names. The test script creates
   /tmp/peekgame-case/Data/Textures/Wall.dds ("wall") plus Both.txt/both.txt. Exit 0 or step. */
#include <windows.h>

#define ROOT L"Z:\\tmp\\peekgame-case\\"
static int read_is(const WCHAR *path, const char *expected) {
    char b[32] = {0};
    DWORD n = 0;
    HANDLE f = CreateFileW(path, GENERIC_READ, FILE_SHARE_READ, NULL, OPEN_EXISTING, 0, NULL);
    if (f == INVALID_HANDLE_VALUE) return 0;
    ReadFile(f, b, sizeof b - 1, &n, NULL);
    CloseHandle(f);
    return lstrcmpA(b, expected) == 0;
}
static int check(void) {
    if (!read_is(ROOT L"data\\TEXTURES\\wall.DDS", "wall")) return 1;
    if (GetFileAttributesW(ROOT L"DATA\\textures") == INVALID_FILE_ATTRIBUTES) return 2;
    WIN32_FIND_DATAW fd;
    HANDLE find = FindFirstFileW(ROOT L"dAtA\\tExTuReS\\*.dds", &fd);
    if (find == INVALID_HANDLE_VALUE || lstrcmpW(fd.cFileName, L"Wall.dds")) return 3;
    FindClose(find);
    /* A new file in a mis-cased existing directory lands in that directory. */
    HANDLE f = CreateFileW(ROOT L"DATA\\TEXTURES\\New.txt", GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, 0, NULL);
    DWORD n;
    if (f == INVALID_HANDLE_VALUE || !WriteFile(f, "new", 3, &n, NULL)) return 4;
    CloseHandle(f);
    if (!read_is(ROOT L"Data\\Textures\\New.txt", "new")) return 5;
    /* An exact match wins over a case variant. */
    if (!read_is(ROOT L"both.txt", "lower") || !read_is(ROOT L"Both.txt", "upper")) return 6;
    /* Still missing: not found, with the right error. */
    if (GetFileAttributesW(ROOT L"data\\missing.txt") != INVALID_FILE_ATTRIBUTES || GetLastError() != ERROR_FILE_NOT_FOUND) return 7;
    /* Devices: NUL swallows writes and creates no file; COM1 does not exist. */
    f = CreateFileW(L"NUL", GENERIC_WRITE, 0, NULL, OPEN_EXISTING, 0, NULL);
    if (f == INVALID_HANDLE_VALUE || !WriteFile(f, "xyz", 3, &n, NULL) || n != 3) return 8;
    CloseHandle(f);
    f = CreateFileW(ROOT L"nul.txt", GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, 0, NULL);
    if (f == INVALID_HANDLE_VALUE) return 9;
    CloseHandle(f);
    if (GetFileAttributesW(ROOT L"nul.txt") != INVALID_FILE_ATTRIBUTES && GetFileAttributesW(L"NUL") == INVALID_FILE_ATTRIBUTES) return 10;
    if (CreateFileW(L"COM1", GENERIC_WRITE, 0, NULL, OPEN_EXISTING, 0, NULL) != INVALID_HANDLE_VALUE) return 11;
    return 0;
}
void entry(void) { ExitProcess(check()); }
