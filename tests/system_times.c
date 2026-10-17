#include <windows.h>
static ULONGLONG value(FILETIME t) { return ((ULONGLONG)t.dwHighDateTime << 32) | t.dwLowDateTime; }
void entry(void) {
    FILETIME a[3], b[3];
    if (!GetSystemTimes(&a[0], &a[1], &a[2])) ExitProcess(1);
    Sleep(40);
    if (!GetSystemTimes(&b[0], &b[1], &b[2])) ExitProcess(2);
    for (int i = 0; i < 3; ++i)
        if (!value(a[i]) || value(b[i]) < value(a[i])) ExitProcess(3+i);
    if (value(b[1]) < value(b[0])) ExitProcess(6);
    SetLastError(1234);
    if (!GetSystemTimes(NULL, NULL, NULL) || GetLastError()!=1234) ExitProcess(7);
    if (!GetSystemTimes(NULL, &b[1], NULL)) ExitProcess(8);
    DWORD written;
    if (!WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), b, sizeof b, &written, NULL) || written!=sizeof b)
        ExitProcess(9);
    ExitProcess(0);
}
