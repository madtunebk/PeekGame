#include <windows.h>
#define CHECK(x) do { if (!(x)) ExitProcess(__LINE__); } while (0)
static DWORD teb_locale(void) { DWORD id; __asm__ volatile("movl %%gs:0x108, %0" : "=r"(id)); return id; }
static DWORD WINAPI grandchild(void *unused) { (void)unused; CHECK(GetThreadLocale() == LOCALE_INVARIANT); CHECK(teb_locale() == LOCALE_INVARIANT); return 0; }
static void joined(HANDLE h) { DWORD result = 99; CHECK(h); CHECK(WaitForSingleObject(h, 2000) == WAIT_OBJECT_0); CHECK(GetExitCodeThread(h, &result) && result == 0); CHECK(CloseHandle(h)); }
static DWORD WINAPI child(void *unused) {
 (void)unused; CHECK(GetThreadLocale() == 0x411 && teb_locale() == 0x411);
 CHECK(SetThreadLocale(LOCALE_INVARIANT)); CHECK(GetThreadLocale() == LOCALE_INVARIANT && teb_locale() == LOCALE_INVARIANT);
 joined(CreateThread(NULL, 0, grandchild, NULL, 0, NULL));
 CHECK(GetThreadLocale() == LOCALE_INVARIANT); return 0;
}
void entry(void) {
 LCID original = GetUserDefaultLCID(); SetLastError(0x1234);
 CHECK(GetThreadLocale() == original && teb_locale() == original); CHECK(GetLastError() == 0x1234);
 CHECK(SetThreadLocale(0x411)); CHECK(GetLastError() == 0x1234); CHECK(GetThreadLocale() == 0x411 && teb_locale() == 0x411);
 HANDLE h = CreateThread(NULL, 0, child, NULL, CREATE_SUSPENDED, NULL); CHECK(h);
 CHECK(SetThreadLocale(LOCALE_USER_DEFAULT)); CHECK(GetThreadLocale() == original);
 CHECK(ResumeThread(h) == 1); joined(h); CHECK(GetThreadLocale() == original && teb_locale() == original);
 CHECK(!SetThreadLocale(0xdeadbeef)); CHECK(GetLastError() == ERROR_INVALID_PARAMETER); CHECK(GetThreadLocale() == original);
 CHECK(!SetThreadLocale(LOCALE_CUSTOM_UNSPECIFIED)); CHECK(GetLastError() == ERROR_NOT_SUPPORTED);
 CHECK(SetThreadLocale(LOCALE_SYSTEM_DEFAULT)); CHECK(GetThreadLocale() == original);
 CHECK(GetACP() == CP_UTF8); ExitProcess(0);
}
