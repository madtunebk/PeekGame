#include <windows.h>
#include <objbase.h>
#define CHECK(x) do { if (!(x)) ExitProcess(__LINE__); } while (0)
static void token_error(HRESULT expected) { ULONG_PTR token = 0x1234; CHECK(CoGetContextToken(&token) == expected); CHECK(token == 0); }
static DWORD WINAPI worker(void *unused) {
 (void)unused; token_error(CO_E_NOTINITIALIZED);
 CHECK(CoInitializeEx(NULL, COINIT_MULTITHREADED) == S_OK); token_error(E_NOTIMPL);
 CoUninitialize(); token_error(CO_E_NOTINITIALIZED); return 0;
}
void entry(void) {
 CHECK(CoGetContextToken(NULL) == E_POINTER); token_error(CO_E_NOTINITIALIZED);
 CHECK(CoInitializeEx(NULL, COINIT_APARTMENTTHREADED) == S_OK);
 CHECK(CoGetContextToken(NULL) == E_POINTER); token_error(E_NOTIMPL);
 CHECK(CoInitializeEx(NULL, COINIT_APARTMENTTHREADED) == S_FALSE);
 CHECK(CoInitializeEx(NULL, COINIT_MULTITHREADED) == RPC_E_CHANGED_MODE);
 CoUninitialize(); token_error(E_NOTIMPL);
 HANDLE thread = CreateThread(NULL, 0, worker, NULL, 0, NULL); CHECK(thread);
 CHECK(WaitForSingleObject(thread, 2000) == WAIT_OBJECT_0); DWORD code = 99; CHECK(GetExitCodeThread(thread, &code) && code == 0); CHECK(CloseHandle(thread));
 token_error(E_NOTIMPL); CoUninitialize(); token_error(CO_E_NOTINITIALIZED);
 CHECK(OleInitialize(NULL) == S_OK); token_error(E_NOTIMPL); OleUninitialize(); token_error(CO_E_NOTINITIALIZED);
 ExitProcess(0);
}
