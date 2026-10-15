/* plugin.dll: imported statically by host.exe. Same preferred base as dep.dll, so one
 * of them is relocated. Has static TLS with a callback, exports by name, by ordinal
 * only, a data export and forwarders (see plugin.def). */
#include <windows.h>
__declspec(dllimport) extern volatile LONG dep_attached;
static DWORD tls_index;
__attribute__((section(".tls$AAA"))) unsigned long long tls_cell=0x1234;
__attribute__((section(".tls$ZZZ"))) char tls_end;
volatile LONG process_attach,thread_attach,thread_detach,tls_notes;
static unsigned long long *block(void){void **v;__asm__("movq %%gs:0x58,%0":"=r"(v));return v[tls_index];}
static void NTAPI note(void *image,DWORD reason,void *reserved){(void)image;(void)reserved;if(reason<=3)InterlockedIncrement(&tls_notes);}
static PIMAGE_TLS_CALLBACK callbacks[]={note,0};
__attribute__((section(".rdata$T"),used)) const IMAGE_TLS_DIRECTORY64 _tls_used={
 (ULONGLONG)&tls_cell,(ULONGLONG)&tls_end,(ULONGLONG)&tls_index,(ULONGLONG)&callbacks,0,0};
int add_one(int x){return x+1;}
int hidden(void){return 70;}
/* This thread's copy of the TLS cell, then sets it to `next`. */
unsigned long long swap_tls(unsigned long long next){unsigned long long old=block()[0];block()[0]=next;return old;}
BOOL WINAPI DllMain(HINSTANCE h,DWORD reason,LPVOID r){
 (void)h;(void)r;
 if(reason==DLL_PROCESS_ATTACH){if(!dep_attached)return FALSE;InterlockedIncrement(&process_attach);}
 if(reason==DLL_THREAD_ATTACH)InterlockedIncrement(&thread_attach);
 if(reason==DLL_THREAD_DETACH)InterlockedIncrement(&thread_detach);
 return TRUE;
}
