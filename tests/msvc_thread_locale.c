#include <windows.h>
__declspec(dllimport) int _configthreadlocale(int);
__declspec(dllimport) int *_errno(void);
__declspec(dllimport) char *setlocale(int,const char *);
static DWORD WINAPI worker(void *unused) {
    (void)unused;
    if (_configthreadlocale(0)!=2) return 1;
    if (_configthreadlocale(1)!=2||_configthreadlocale(0)!=1) return 2;
    if (!setlocale(0,"C")||setlocale(0,"not-a-locale")) return 3;
    return 0;
}
void entry(void) {
    if (_configthreadlocale(0)!=2) ExitProcess(1);
    *_errno()=77;
    if (_configthreadlocale(1)!=2||_configthreadlocale(0)!=1||*_errno()!=77) ExitProcess(2);
    if (_configthreadlocale(2)!=1||_configthreadlocale(0)!=2) ExitProcess(3);
    if (_configthreadlocale(-1)!=-1||*_errno()!=22||_configthreadlocale(0)!=2) ExitProcess(4);
    HANDLE t=CreateThread(0,0,worker,0,0,0);DWORD code;
    if (!t||WaitForSingleObject(t,5000)!=0||!GetExitCodeThread(t,&code)||code) ExitProcess(5);
    if (_configthreadlocale(0)!=2) ExitProcess(6);
    CloseHandle(t);
    ExitProcess(0);
}
