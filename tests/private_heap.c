#include <windows.h>
#define CHECK(x,n) if(!(x)) ExitProcess(n)
void entry(void) {
 HANDLE a=HeapCreate(0,4096,0),b=HeapCreate(HEAP_NO_SERIALIZE,0,0);
 CHECK(a&&b&&a!=b&&a!=GetProcessHeap(),1);
 unsigned char *p=HeapAlloc(a,HEAP_ZERO_MEMORY,32); CHECK(p,2);
 for(int i=0;i<32;i++) CHECK(!p[i],3);
 CHECK(HeapSize(a,0,p)==32,4);
 CHECK(!HeapFree(b,0,p),5);
 CHECK(HeapSize(a,0,p)==32,6);
 p[0]=42; p=HeapReAlloc(a,HEAP_ZERO_MEMORY,p,8192); CHECK(p&&p[0]==42,7);
 for(int i=32;i<8192;i++) CHECK(!p[i],8);
 CHECK(HeapDestroy(a),9);
 CHECK(!HeapAlloc(a,0,8)&&GetLastError()==ERROR_INVALID_HANDLE,10);
 CHECK(!HeapDestroy(a),11); CHECK(!HeapDestroy(GetProcessHeap()),12);
 CHECK(HeapDestroy(b),13);
 CHECK(!HeapCreate(0,0,4096)&&GetLastError()==ERROR_NOT_SUPPORTED,14);
 CHECK(!HeapCreate(HEAP_CREATE_ENABLE_EXECUTE,0,0),15);
 ULONG mode=99; SIZE_T got=0;
 CHECK(HeapQueryInformation(GetProcessHeap(),HeapCompatibilityInformation,&mode,sizeof mode,&got)&&mode==0&&got==4,16);
 mode=2; CHECK(!HeapSetInformation(GetProcessHeap(),HeapCompatibilityInformation,&mode,sizeof mode)&&GetLastError()==ERROR_NOT_SUPPORTED,17);
 void *q=HeapAlloc(GetProcessHeap(),0,128); CHECK(q&&HeapFree(GetProcessHeap(),0,q),18);
 ExitProcess(0);
}
