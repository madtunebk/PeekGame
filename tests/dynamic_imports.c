#include <windows.h>
void entry(void) {
    HMODULE kernel = LoadLibraryA("kernel32.dll");
    if (!kernel) ExitProcess(1);
    typedef DWORD (WINAPI *ErrorFn)(void);
    ErrorFn get_error = (ErrorFn)GetProcAddress(kernel,"GetLastError");
    if (!get_error) ExitProcess(2);
    if (GetProcAddress(kernel,"PeekGameMissingExport") || get_error()!=ERROR_PROC_NOT_FOUND) ExitProcess(3);
    if (LoadLibraryW(L"PeekGameMissingModule.dll") || get_error()!=ERROR_MOD_NOT_FOUND) ExitProcess(4);
    ExitProcess(0);
}
