#include <windows.h>
static DWORD tls_index;
__attribute__((section(".tls$AAA"))) unsigned long long initial_tls=0x12345678;
__attribute__((section(".tls$ZZZ"))) char tls_end;
static volatile LONG attached,detached;
static unsigned long long *block(void){void **vector;__asm__("movq %%gs:0x58,%0":"=r"(vector));return vector[tls_index];}
static void NTAPI notify(void *image,DWORD reason,void *reserved){
 (void)image;(void)reserved;
 if(reason==1||reason==2){if(block()[0]!=0x12345678||block()[1]!=0)ExitProcess(10+reason);block()[0]=reason==1?0x55:0x66;if(reason==2)InterlockedIncrement(&attached);}
 if(reason==3)InterlockedIncrement(&detached);
}
static PIMAGE_TLS_CALLBACK callbacks[]={notify,0};
__attribute__((section(".rdata$T"),used)) const IMAGE_TLS_DIRECTORY64 _tls_used={
 (ULONGLONG)&initial_tls,(ULONGLONG)&tls_end,(ULONGLONG)&tls_index,(ULONGLONG)&callbacks,8,0};
static CRITICAL_SECTION critical;
static HANDLE seen,release_worker;
static DWORD dynamic_slot;
static DWORD WINAPI worker(void *unused){
 (void)unused;
 if(block()[0]!=0x66||TlsGetValue(dynamic_slot)!=0)return 20;
 if(!TlsSetValue(dynamic_slot,(void*)44))return 21;
 if(TryEnterCriticalSection(&critical))return 22;
 if(!SetEvent(seen)||WaitForSingleObject(release_worker,5000)!=WAIT_OBJECT_0)return 23;
 EnterCriticalSection(&critical);LeaveCriticalSection(&critical);
 return TlsGetValue(dynamic_slot)==(void*)44?0:24;
}
void entry(void){
 if(block()[0]!=0x55)ExitProcess(1);
 dynamic_slot=TlsAlloc();if(dynamic_slot==TLS_OUT_OF_INDEXES||!TlsSetValue(dynamic_slot,(void*)77))ExitProcess(2);
 InitializeCriticalSection(&critical);EnterCriticalSection(&critical);EnterCriticalSection(&critical);
 seen=CreateEventW(0,FALSE,FALSE,0);release_worker=CreateEventA(0,TRUE,FALSE,0);
 DWORD id=0;HANDLE thread=CreateThread(0,0,worker,0,0,&id),duplicate=0;
 if(!thread||!id||!DuplicateHandle(GetCurrentProcess(),thread,GetCurrentProcess(),&duplicate,0,FALSE,DUPLICATE_SAME_ACCESS))ExitProcess(3);
 if(WaitForSingleObject(seen,5000)!=WAIT_OBJECT_0||WaitForSingleObject(seen,0)!=WAIT_TIMEOUT||WaitForSingleObject(thread,0)!=WAIT_TIMEOUT)ExitProcess(4);
 LeaveCriticalSection(&critical);LeaveCriticalSection(&critical);SetEvent(release_worker);
 if(WaitForSingleObject(duplicate,5000)!=WAIT_OBJECT_0)ExitProcess(5);
 DWORD code=999;if(!GetExitCodeThread(thread,&code)||code||attached!=1||detached!=1||TlsGetValue(dynamic_slot)!=(void*)77||block()[0]!=0x55)ExitProcess(6);
 if(!CloseHandle(thread)||!CloseHandle(duplicate)||!CloseHandle(seen)||!CloseHandle(release_worker)||!TlsFree(dynamic_slot))ExitProcess(7);
 DeleteCriticalSection(&critical);ExitProcess(0);
}
