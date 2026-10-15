#include <windows.h>
struct Args {HMODULE module;volatile DWORD detached;};
static struct Args *args;
__declspec(dllexport) DWORD WINAPI worker(void *data) {
    args=data;
    FreeLibraryAndExitThread(args->module,91);
    return 99;
}
BOOL WINAPI DllMain(HINSTANCE module,DWORD reason,void *reserved) {
    (void)module;(void)reserved;
    if(reason==DLL_PROCESS_DETACH && args) args->detached=1;
    return TRUE;
}
