/* Threading audit batch B: CreateEvent security attributes and names, OpenEvent,
   waitable timer completion routines (APC) and thread reuse, critical section
   ownership fields. Exit 0 or the failing step. */
#include <windows.h>

static volatile LONG fired;
static volatile ULONG_PTR seen_arg;
static VOID CALLBACK timer_apc(LPVOID arg, DWORD low, DWORD high) {
    (void)low; (void)high;
    seen_arg = (ULONG_PTR)arg;
    InterlockedIncrement(&fired);
}
static CRITICAL_SECTION cs;
static DWORD WINAPI try_other(LPVOID unused) {
    (void)unused;
    if (TryEnterCriticalSection(&cs)) { LeaveCriticalSection(&cs); return 1; }
    return 0;
}

static int check(void) {
    /* T5 */
    SECURITY_ATTRIBUTES sa = {sizeof sa, NULL, TRUE};
    HANDLE e = CreateEventW(&sa, TRUE, FALSE, NULL);
    if (!e) return 1;
    HANDLE n1 = CreateEventW(NULL, TRUE, FALSE, L"Local\\PeekGameEvent");
    if (!n1 || GetLastError() != 0) return 2;
    HANDLE n2 = CreateEventW(NULL, FALSE, TRUE, L"PeekGameEvent");
    if (!n2 || GetLastError() != ERROR_ALREADY_EXISTS) return 3;
    if (WaitForSingleObject(n1, 0) != WAIT_TIMEOUT) return 4; /* state of the existing event kept */
    SetEvent(n2);
    if (WaitForSingleObject(n1, 0) != WAIT_OBJECT_0) return 5;
    HANDLE n3 = OpenEventW(EVENT_ALL_ACCESS, FALSE, L"PeekGameEvent");
    if (!n3 || WaitForSingleObject(n3, 0) != WAIT_OBJECT_0) return 6;
    if (OpenEventW(EVENT_ALL_ACCESS, FALSE, L"NoSuchPeekEvent") || GetLastError() != ERROR_FILE_NOT_FOUND) return 7;
    HANDLE m = CreateMutexW(NULL, FALSE, L"PeekGameMutex");
    if (!m || CreateEventW(NULL, TRUE, FALSE, L"PeekGameMutex") || GetLastError() != ERROR_INVALID_HANDLE) return 8;
    HANDLE ex = CreateEventExW(NULL, L"PeekGameEvent", CREATE_EVENT_MANUAL_RESET, EVENT_ALL_ACCESS);
    if (!ex || GetLastError() != ERROR_ALREADY_EXISTS) return 9;
    CloseHandle(n1); CloseHandle(n2); CloseHandle(n3); CloseHandle(ex);
    if (OpenEventW(EVENT_ALL_ACCESS, FALSE, L"PeekGameEvent") || GetLastError() != ERROR_FILE_NOT_FOUND) return 10; /* name dies with its handles */
    /* T6: completion routine runs only in an alertable wait */
    HANDLE t = CreateWaitableTimerW(NULL, FALSE, NULL);
    LARGE_INTEGER due; due.QuadPart = -100000; /* 10 ms */
    if (!SetWaitableTimer(t, &due, 0, timer_apc, (LPVOID)0x1234, FALSE)) return 20;
    if (WaitForSingleObject(t, 2000) != WAIT_OBJECT_0) return 21;
    Sleep(30);
    if (fired != 0) return 22;
    if (SleepEx(2000, TRUE) != WAIT_IO_COMPLETION || fired != 1 || seen_arg != 0x1234) return 23;
    due.QuadPart = -50000;
    if (!SetWaitableTimer(t, &due, 20, timer_apc, (LPVOID)0x99, FALSE)) return 24;
    for (int i = 0; i < 3; i++) if (SleepEx(2000, TRUE) != WAIT_IO_COMPLETION) return 25;
    if (fired < 4 || seen_arg != 0x99) return 26;
    CancelWaitableTimer(t);
    /* Re-arming many times must not leave sleeping threads behind (counted by the script). */
    due.QuadPart = -100000000; /* 10 s */
    for (int i = 0; i < 300; i++) SetWaitableTimer(t, &due, 0, NULL, NULL, FALSE);
    CloseHandle(t);
    Sleep(300);
    HANDLE marker = CreateFileW(L"Z:\\tmp\\peekgame-batch-b.ready", GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, 0, NULL);
    CloseHandle(marker);
    Sleep(1500); /* the script counts threads now */
    /* T8 */
    InitializeCriticalSection(&cs);
    if (cs.LockCount != -1 || cs.RecursionCount != 0 || cs.OwningThread) return 30;
    EnterCriticalSection(&cs);
    EnterCriticalSection(&cs);
    if (cs.RecursionCount != 2 || cs.OwningThread != (HANDLE)(ULONG_PTR)GetCurrentThreadId() || cs.LockCount != -2) return 31;
    HANDLE other = CreateThread(NULL, 0, try_other, NULL, 0, NULL);
    DWORD code = 9;
    WaitForSingleObject(other, 2000); GetExitCodeThread(other, &code);
    if (code != 0) return 32;
    LeaveCriticalSection(&cs);
    if (cs.RecursionCount != 1 || !cs.OwningThread) return 33;
    LeaveCriticalSection(&cs);
    if (cs.RecursionCount != 0 || cs.OwningThread || cs.LockCount != -1) return 34;
    DeleteCriticalSection(&cs);
    /* Process attribute lists are real; CreateProcess fails honestly. */
    SIZE_T size = 0;
    if (InitializeProcThreadAttributeList(NULL, 2, 0, &size) || GetLastError() != ERROR_INSUFFICIENT_BUFFER || size == 0) return 40;
    static BYTE storage[256];
    LPPROC_THREAD_ATTRIBUTE_LIST list = (LPPROC_THREAD_ATTRIBUTE_LIST)storage;
    if (size > sizeof storage || !InitializeProcThreadAttributeList(list, 2, 0, &size)) return 41;
    HANDLE inherit[1] = {e};
    if (!UpdateProcThreadAttribute(list, 0, PROC_THREAD_ATTRIBUTE_HANDLE_LIST, inherit, sizeof inherit, NULL, NULL)) return 42;
    DeleteProcThreadAttributeList(list);
    STARTUPINFOW si = {sizeof si};
    PROCESS_INFORMATION pi;
    WCHAR cmd[] = L"child.exe --flag";
    if (CreateProcessW(NULL, cmd, NULL, NULL, FALSE, 0, NULL, NULL, &si, &pi) || GetLastError() != ERROR_NOT_SUPPORTED) return 43;
    return 0;
}
void entry(void) { ExitProcess(check()); }
