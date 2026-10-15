/* dep.dll: a dependency of plugin.dll; its DllMain must run first. */
#include <windows.h>
__declspec(dllexport) volatile LONG dep_attached;
__declspec(dllexport) int dep_value(void){return 41;}
BOOL WINAPI DllMain(HINSTANCE h,DWORD reason,LPVOID r){(void)h;(void)r;if(reason==DLL_PROCESS_ATTACH)dep_attached=1;return TRUE;}
