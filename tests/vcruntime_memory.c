#include <windows.h>
#include <stddef.h>
__declspec(dllimport) void *memcpy(void *,const void *,size_t);
__declspec(dllimport) void *memmove(void *,const void *,size_t);
__declspec(dllimport) void *memset(void *,int,size_t);
__declspec(dllimport) int memcmp(const void *,const void *,size_t);
void entry(void) {
    char a[16],b[16];
    if (memset(a,0x41,sizeof a)!=a||memcpy(b,a,sizeof a)!=b||memcmp(a,b,sizeof a)) ExitProcess(1);
    b[0]='B';
    if (memcmp(a,b,sizeof a)>=0||memcmp(b,a,sizeof a)<=0) ExitProcess(2);
    if (memmove(b+1,b,15)!=b+1||b[1]!='B'||b[2]!='A') ExitProcess(3);
    HMODULE h=LoadLibraryA("VCRUNTIME140.dll");
    if (!h||!GetProcAddress(h,"__C_specific_handler")) ExitProcess(4);
    ExitProcess(0);
}
