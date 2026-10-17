#include <windows.h>
#define CHECK(x) do { if (!(x)) ExitProcess(__LINE__); } while (0)
static ULONGLONG value(FILETIME f) { return ((ULONGLONG)f.dwHighDateTime << 32) | f.dwLowDateTime; }
struct Times { DWORD guard; FILETIME creation, exit, kernel, user; };
static HANDLE ready, release;
static volatile ULONG spin;
static void burn(void) { ULONGLONG end = GetTickCount64() + 80; while (GetTickCount64() < end) ++spin; }
static DWORD WINAPI worker(void *unused) { (void)unused; burn(); CHECK(SetEvent(ready)); CHECK(WaitForSingleObject(release, 2000) == WAIT_OBJECT_0); return 17; }
static void read_times(HANDLE h, struct Times *t) { t->guard = 0x12345678; CHECK(GetThreadTimes(h, &t->creation, &t->exit, &t->kernel, &t->user)); CHECK(t->guard == 0x12345678); }
void entry(void) {
    struct Times a, b; FILETIME now; GetSystemTimeAsFileTime(&now);
    read_times(GetCurrentThread(), &a); CHECK(value(a.creation) > 116444736000000000ULL); CHECK(value(a.creation) <= value(now)); CHECK(value(a.exit) == 0);
    burn(); read_times(GetCurrentThread(), &b); CHECK(value(a.creation) == value(b.creation)); CHECK(value(b.kernel) + value(b.user) > value(a.kernel) + value(a.user));
    CHECK(!GetThreadTimes((HANDLE)0x1234, &a.creation, &a.exit, &a.kernel, &a.user)); CHECK(GetLastError() == ERROR_INVALID_HANDLE);
    CHECK(!GetThreadTimes(GetCurrentThread(), NULL, &a.exit, &a.kernel, &a.user)); CHECK(GetLastError() == ERROR_NOACCESS);
    ready = CreateEventW(NULL, TRUE, FALSE, NULL); release = CreateEventW(NULL, TRUE, FALSE, NULL); CHECK(ready && release);
    HANDLE thread = CreateThread(NULL, 0, worker, NULL, 0, NULL), duplicate = NULL; CHECK(thread);
    CHECK(DuplicateHandle(GetCurrentProcess(), thread, GetCurrentProcess(), &duplicate, 0, FALSE, DUPLICATE_SAME_ACCESS));
    CHECK(WaitForSingleObject(ready, 2000) == WAIT_OBJECT_0); read_times(thread, &a); CHECK(value(a.exit) == 0); CHECK(value(a.user) + value(a.kernel) > 0);
    CHECK(SetEvent(release)); CHECK(WaitForSingleObject(thread, 2000) == WAIT_OBJECT_0); read_times(thread, &b);
    CHECK(value(a.creation) == value(b.creation)); CHECK(value(b.exit) >= value(b.creation)); CHECK(value(b.kernel) + value(b.user) >= value(a.kernel) + value(a.user));
    CHECK(CloseHandle(thread)); read_times(duplicate, &a); CHECK(value(a.exit) == value(b.exit)); CHECK(value(a.kernel) == value(b.kernel) && value(a.user) == value(b.user));
    CHECK(CloseHandle(duplicate)); CHECK(!GetThreadTimes(duplicate, &a.creation, &a.exit, &a.kernel, &a.user)); CHECK(GetLastError() == ERROR_INVALID_HANDLE);
    CHECK(CloseHandle(ready)); CHECK(CloseHandle(release)); ExitProcess(0);
}
