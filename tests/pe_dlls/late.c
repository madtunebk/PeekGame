/* late.dll: loaded with LoadLibrary and unloaded with FreeLibrary. It imports from
 * the EXE, and its DLL_PROCESS_DETACH reports back through that import. */
#include <windows.h>
__declspec(dllimport) void host_note_detach(void);
__declspec(dllexport) int late_value(void){return 99;}
BOOL WINAPI DllMain(HINSTANCE h,DWORD reason,LPVOID r){(void)h;(void)r;if(reason==DLL_PROCESS_DETACH)host_note_detach();return TRUE;}
