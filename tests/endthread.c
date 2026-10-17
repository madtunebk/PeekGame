#include <windows.h>
#include <process.h>
#define CHECK(x) do { if (!(x)) ExitProcess(__LINE__); } while (0)
static DWORD slot;
static LONG callbacks;
static void WINAPI cleanup(void *value) {
    CHECK(value == (void *)0x1234);
    InterlockedIncrement(&callbacks);
}
static unsigned __stdcall worker(void *unused) {
    (void)unused;
    CHECK(FlsSetValue(slot, (void *)0x1234));
    _endthreadex(73);
    return 99;
}
void entry(void) {
    slot = FlsAlloc(cleanup);
    CHECK(slot != FLS_OUT_OF_INDEXES);
    for (int i = 0; i < 4; ++i) {
        HANDLE thread = (HANDLE)_beginthreadex(NULL, 0, worker, NULL, 0, NULL);
        CHECK(thread);
        CHECK(WaitForSingleObject(thread, 2000) == WAIT_OBJECT_0);
        DWORD code = 0;
        CHECK(GetExitCodeThread(thread, &code) && code == 73);
        CHECK(callbacks == i + 1);
        CHECK(CloseHandle(thread));
    }
    CHECK(FlsFree(slot));
    ExitProcess(0);
}
