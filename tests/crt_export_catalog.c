#include <windows.h>
#include "crt_export_names.h"
void entry(void){
 HMODULE m=LoadLibraryA("msvcrt.dll");if(!m)ExitProcess(1);
 for(unsigned i=0;i<sizeof names/sizeof names[0];i++)if(!GetProcAddress(m,names[i]))ExitProcess(10+i);
 const char **zone=(const char**)GetProcAddress(m,"_tzname");
 const int *offset=(const int*)GetProcAddress(m,"_timezone");
 if(!zone||!zone[0]||!zone[1]||!offset||*offset < -86400||*offset > 86400)ExitProcess(8);
 HMODULE avrt=LoadLibraryA("avrt.dll");if(!avrt)ExitProcess(2);
 typedef HANDLE(WINAPI *Register)(const void*,DWORD*);
 typedef BOOL(WINAPI *Revert)(HANDLE);
 typedef BOOL(WINAPI *Priority)(HANDLE,int);
 Register reg=(Register)GetProcAddress(avrt,"AvSetMmThreadCharacteristicsW");
 Register rega=(Register)GetProcAddress(avrt,"AvSetMmThreadCharacteristicsA");
 Revert revert=(Revert)GetProcAddress(avrt,"AvRevertMmThreadCharacteristics");
 Priority priority=(Priority)GetProcAddress(avrt,"AvSetMmThreadPriority");
 if(!reg||!rega||!revert||!priority)ExitProcess(3);
 DWORD index=123;
 if(reg(L"Games",&index)||GetLastError()!=ERROR_NOT_SUPPORTED||index!=123)ExitProcess(4);
 if(rega("Games",&index)||GetLastError()!=ERROR_NOT_SUPPORTED||index!=123)ExitProcess(5);
 if(revert((HANDLE)1)||GetLastError()!=ERROR_INVALID_HANDLE)ExitProcess(6);
 if(priority((HANDLE)1,1)||GetLastError()!=ERROR_INVALID_HANDLE)ExitProcess(7);
 ExitProcess(0);
}
