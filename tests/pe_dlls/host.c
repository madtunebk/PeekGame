/* host.exe: exercises the PE DLL loader. Exit code 0 is a pass; any other code
 * names the check that failed. */
#include <windows.h>
__declspec(dllimport) int add_one(int);
__declspec(dllimport) unsigned long long swap_tls(unsigned long long);
__declspec(dllimport) extern volatile LONG process_attach,thread_attach,thread_detach,tls_notes;
static volatile LONG detached;
__declspec(dllexport) void host_note_detach(void){detached=1;}
static DWORD WINAPI worker(void *p){(void)p;return swap_tls(0x77)==0x1234?0:1;}
typedef int (*IntFn)(void);
void entry(void){
 if(add_one(1)!=2||process_attach!=1)ExitProcess(1);
 HMODULE plugin=GetModuleHandleA("PLUGIN.DLL"),from=0;
 if(!plugin||((UINT_PTR)plugin&0xffff))ExitProcess(2);
 if(!GetModuleHandleExA(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS|GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,(LPCSTR)(void *)add_one,&from)||from!=plugin)ExitProcess(3);
 if(GetModuleHandleA("dep")==NULL||GetModuleHandleA("dep")==plugin)ExitProcess(4);
 if(GetProcAddress(plugin,"Sleepy")!=GetProcAddress(GetModuleHandleA("kernel32.dll"),"Sleep"))ExitProcess(5);
 IntFn dep=(IntFn)(void *)GetProcAddress(plugin,"DepValue"),hidden=(IntFn)(void *)GetProcAddress(plugin,MAKEINTRESOURCEA(7));
 if(!dep||dep()!=41||!hidden||hidden()!=70)ExitProcess(6);
 SetLastError(0);if(GetProcAddress(plugin,"Nothing")||GetLastError()!=ERROR_PROC_NOT_FOUND)ExitProcess(7);
 if(swap_tls(0x55)!=0x1234||swap_tls(0x55)!=0x55)ExitProcess(8);
 DWORD id;HANDLE thread=CreateThread(0,0,worker,0,0,&id),code=0;
 if(!thread||WaitForSingleObject(thread,5000)!=WAIT_OBJECT_0||!GetExitCodeThread(thread,(DWORD *)&code)||code)ExitProcess(9);
 if(thread_attach!=1||thread_detach!=1||swap_tls(0x55)!=0x55||tls_notes!=3)ExitProcess(10);
 if(!LoadLibraryExW(L"api-ms-win-core-synch-l1-2-0",0,LOAD_LIBRARY_SEARCH_SYSTEM32))ExitProcess(11);
 SetLastError(0);if(LoadLibraryExW(L"late.dll",0,LOAD_LIBRARY_SEARCH_SYSTEM32)||GetLastError()!=ERROR_MOD_NOT_FOUND)ExitProcess(12);
 HMODULE late=LoadLibraryA("late");IntFn value=late?(IntFn)(void *)GetProcAddress(late,"late_value"):0;
 if(!value||value()!=99||!FreeLibrary(late)||!detached||GetModuleHandleA("late.dll"))ExitProcess(13);
 SetLastError(0);if(LoadLibraryA("missing.dll")||GetLastError()!=ERROR_MOD_NOT_FOUND)ExitProcess(14);
 ExitProcess(0);
}
