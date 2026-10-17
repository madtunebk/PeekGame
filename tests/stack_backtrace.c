#include <windows.h>
#define CHECK(x) do { if (!(x)) ExitProcess(__LINE__); } while (0)
static volatile unsigned work;
__attribute__((noinline)) static unsigned leaf(void *grandparent) {
    void *full[32], *skipped[32]; ULONG hash=0, other=0;
    USHORT count = RtlCaptureStackBackTrace(0, 32, full, &hash); CHECK(count >= 3 && count <= 32);
    CHECK(full[1] == __builtin_return_address(0)); CHECK(full[2] == grandparent);
    ULONG sum=0; for (unsigned i=0;i<count;++i) { CHECK(full[i]); sum += (ULONG)(ULONG_PTR)full[i]; } CHECK(hash == sum);
    USHORT rest = RtlCaptureStackBackTrace(1, 32, skipped, &other); CHECK(rest == count-1);
    for (unsigned i=0;i<rest;++i) CHECK(skipped[i] == full[i+1]);
    CHECK(RtlCaptureStackBackTrace(0, 1, skipped, NULL) == 1);
    other=99; CHECK(RtlCaptureStackBackTrace(0, 0, skipped, &other) == 0 && other == 0);
    other=99; CHECK(RtlCaptureStackBackTrace(0xffffffff, 32, skipped, &other) == 0 && other == 0);
    work += count; return count;
}
__attribute__((noinline)) static unsigned middle(void) { volatile unsigned padding[19];padding[0]=7; unsigned n=leaf(__builtin_return_address(0));work+=padding[0];return n; }
__attribute__((noinline)) static unsigned outer(void) { volatile unsigned padding[27];padding[0]=11;unsigned n=middle();work+=padding[0];return n; }
static DWORD WINAPI worker(void *unused) { (void)unused;CHECK(outer()>=3);return 0; }
void entry(void) {
 CHECK(outer()>=3);
 HANDLE h=CreateThread(NULL,0,worker,NULL,0,NULL);CHECK(h);CHECK(WaitForSingleObject(h,2000)==WAIT_OBJECT_0);
 DWORD code=99;CHECK(GetExitCodeThread(h,&code)&&code==0);CHECK(CloseHandle(h));ExitProcess(0);
}
