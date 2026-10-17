#include <windows.h>
__declspec(dllimport) int _set_fmode(int);
__declspec(dllimport) int _get_fmode(int *);
__declspec(dllimport) int *__p__fmode(void);
__declspec(dllimport) int *_errno(void);
__declspec(dllimport) int *__p__commode(void);
void entry(void) {
    int *commit=__p__commode();
    if (!commit||*commit!=0) ExitProcess(7);
    *commit=1;
    if (__p__commode()!=commit||*__p__commode()!=1) ExitProcess(8);
    *commit=0;
    int mode=-1;
    if (_get_fmode(&mode)||mode!=0x4000) ExitProcess(1);
    int *p=__p__fmode();
    if (!p||*p!=mode) ExitProcess(2);
    int modes[]={0x4000,0x8000,0x10000};
    for (int i=0;i<3;i++) {
        *_errno()=77;
        if (_set_fmode(modes[i])||_get_fmode(&mode)||mode!=modes[i]||*p!=mode||*_errno()!=77) ExitProcess(3);
    }
    if (_set_fmode(123)!=22||*_errno()!=22||*p!=0x10000) ExitProcess(4);
    if (_get_fmode(0)!=22||*_errno()!=22) ExitProcess(5);
    *p=0x8000;
    if (_get_fmode(&mode)||mode!=0x8000) ExitProcess(6);
    ExitProcess(0);
}
