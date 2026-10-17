/* Threading regressions: abandoned mutexes (WAIT_ABANDONED), critical section re-init at
   the same address, FlushProcessWriteBuffers, INIT_ONCE on reused memory. Exit 0 or step. */
#include <windows.h>

static HANDLE mutex;
static DWORD WINAPI owner_dies(LPVOID unused) {
    (void)unused;
    return WaitForSingleObject(mutex, INFINITE) == WAIT_OBJECT_0 ? 0 : 1; /* exits still owning it */
}
static DWORD WINAPI flusher(LPVOID unused) {
    (void)unused;
    for (int i = 0; i < 200; i++) FlushProcessWriteBuffers();
    return 0;
}
static LONG runs;
static BOOL CALLBACK once_callback(PINIT_ONCE once, PVOID parameter, PVOID *context) {
    (void)once; (void)parameter;
    InterlockedIncrement(&runs);
    *context = (PVOID)(LONG_PTR)(runs * 4);
    return TRUE;
}

static int check(void) {
    /* T1: abandoned mutex, single and multiple waits */
    mutex = CreateMutexW(NULL, FALSE, NULL);
    HANDLE t = CreateThread(NULL, 0, owner_dies, NULL, 0, NULL);
    if (!t || WaitForSingleObject(t, 5000) != WAIT_OBJECT_0) return 1;
    DWORD code = 9;
    if (!GetExitCodeThread(t, &code) || code != 0) return 2;
    CloseHandle(t);
    if (WaitForSingleObject(mutex, 2000) != WAIT_ABANDONED) return 3;
    if (!ReleaseMutex(mutex)) return 4;
    if (WaitForSingleObject(mutex, 0) != WAIT_OBJECT_0) return 5; /* no longer abandoned */
    ReleaseMutex(mutex);
    t = CreateThread(NULL, 0, owner_dies, NULL, 0, NULL);
    WaitForSingleObject(t, 5000);
    CloseHandle(t);
    HANDLE pair[2] = {CreateEventW(NULL, TRUE, FALSE, NULL), mutex};
    if (WaitForMultipleObjects(2, pair, FALSE, 2000) != WAIT_ABANDONED_0 + 1) return 6;
    ReleaseMutex(mutex);
    /* T2: critical section initialized again at the same address without Delete */
    static CRITICAL_SECTION cs;
    InitializeCriticalSection(&cs);
    EnterCriticalSection(&cs);
    LeaveCriticalSection(&cs);
    InitializeCriticalSection(&cs);
    if (!TryEnterCriticalSection(&cs)) return 7;
    LeaveCriticalSection(&cs);
    DeleteCriticalSection(&cs);
    /* T3: FlushProcessWriteBuffers from several threads */
    HANDLE threads[4];
    for (int i = 0; i < 4; i++) threads[i] = CreateThread(NULL, 0, flusher, NULL, 0, NULL);
    if (WaitForMultipleObjects(4, threads, TRUE, 10000) != WAIT_OBJECT_0) return 8;
    /* T4: INIT_ONCE completed, then its memory zeroed (reuse): the callback runs again */
    static INIT_ONCE once = INIT_ONCE_STATIC_INIT;
    PVOID context = NULL;
    if (!InitOnceExecuteOnce(&once, once_callback, NULL, &context) || runs != 1 || context != (PVOID)4) return 9;
    if (!InitOnceExecuteOnce(&once, once_callback, NULL, &context) || runs != 1) return 10; /* still done */
    once.Ptr = NULL;
    if (!InitOnceExecuteOnce(&once, once_callback, NULL, &context) || runs != 2 || context != (PVOID)8) return 11;
    return 0;
}
void entry(void) { ExitProcess(check()); }
