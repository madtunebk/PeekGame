/* Audit regressions: desktop window = primary monitor, GetDeviceCaps(TECHNOLOGY),
   MoveFileEx semantics, LoadIcon names, MessageBox without a display.
   argv-free: PEEK_AUDIT_MODE=nodisplay runs only the MessageBox check. Exit 0 or the step. */
#include <windows.h>

static int check(void) {
    char mode[32] = {0};
    GetEnvironmentVariableA("PEEK_AUDIT_MODE", mode, sizeof mode);
    if (mode[0] == 'n') {
        if (MessageBoxA(NULL, "text", "caption", MB_YESNO) != 0 || GetLastError() != ERROR_NOT_SUPPORTED) return 40;
        return 0;
    }
    RECT client, window;
    if (!GetClientRect(GetDesktopWindow(), &client) || !GetWindowRect(GetDesktopWindow(), &window)) return 1;
    if (client.left || client.top || client.right != GetSystemMetrics(SM_CXSCREEN) || client.bottom != GetSystemMetrics(SM_CYSCREEN)) return 2;
    if (window.left || window.top || window.right != client.right || window.bottom != client.bottom) return 3;
    HDC dc = GetDC(NULL);
    if (GetDeviceCaps(dc, TECHNOLOGY) != DT_RASDISPLAY) return 4;
    ReleaseDC(NULL, dc);
    /* MoveFileEx */
    WCHAR a[] = L"Z:\\tmp\\peekgame-audit-a.txt", b[] = L"Z:\\tmp\\peekgame-audit-b.txt";
    WCHAR missing[] = L"Z:\\tmp\\peekgame-audit-missing-dir\\b.txt";
    WCHAR shm[] = L"Z:\\dev\\shm\\peekgame-audit-c.txt";
    DWORD n;
    HANDLE f = CreateFileW(a, GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, 0, NULL);
    if (f == INVALID_HANDLE_VALUE || !WriteFile(f, "data", 4, &n, NULL)) return 5;
    CloseHandle(f);
    DeleteFileW(b);
    if (MoveFileExW(a, missing, 0) || GetLastError() != ERROR_PATH_NOT_FOUND) return 6;
    if (GetFileAttributesW(L"Z:\\tmp\\peekgame-audit-missing-dir") != INVALID_FILE_ATTRIBUTES) return 7;
    if (MoveFileExW(a, b, MOVEFILE_DELAY_UNTIL_REBOOT) || GetLastError() != ERROR_NOT_SUPPORTED) return 8;
    if (!MoveFileExW(a, b, 0)) return 9;
    f = CreateFileW(a, GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, 0, NULL);
    CloseHandle(f);
    if (MoveFileExW(a, b, 0) || GetLastError() != ERROR_ALREADY_EXISTS && GetLastError() != ERROR_FILE_EXISTS) return 10;
    if (!MoveFileExW(a, b, MOVEFILE_REPLACE_EXISTING)) return 11;
    /* Across filesystems (/tmp -> /dev/shm when they differ): only with COPY_ALLOWED. */
    DeleteFileW(shm);
    char cross[8] = {0};
    GetEnvironmentVariableA("PEEK_AUDIT_CROSS", cross, sizeof cross);
    if (cross[0] == '1') {
        if (MoveFileExW(b, shm, 0) || GetLastError() != ERROR_NOT_SAME_DEVICE) return 12;
        if (!MoveFileExW(b, shm, MOVEFILE_COPY_ALLOWED)) return 13;
        if (GetFileAttributesW(b) != INVALID_FILE_ATTRIBUTES || GetFileAttributesW(shm) == INVALID_FILE_ATTRIBUTES) return 14;
        DeleteFileW(shm);
    } else {
        DeleteFileW(b);
    }
    /* LoadIcon */
    if (!LoadIconW(NULL, MAKEINTRESOURCEW(32512))) return 20;
    if (LoadIconW(NULL, MAKEINTRESOURCEW(1)) || GetLastError() != ERROR_RESOURCE_NAME_NOT_FOUND) return 21;
    return 0;
}

void entry(void) { ExitProcess(check()); }
