#include <windows.h>
typedef struct {ULONG_PTR status,information;} IO;
__declspec(dllimport) LONG NtWriteFile(HANDLE,HANDLE,void *,void *,IO *,const void *,ULONG,const LONGLONG *,const ULONG *);
void entry(void) {
    IO io;char data[4];DWORD n;LONGLONG offset=-2;
    HANDLE h=CreateFileW(L"nt-write.bin",GENERIC_READ|GENERIC_WRITE,0,0,CREATE_ALWAYS,0,0);
    if (h==INVALID_HANDLE_VALUE) ExitProcess(1);
    SetLastError(123);
    if (NtWriteFile(h,0,0,0,&io,"abc",3,0,0)||io.status||io.information!=3||GetLastError()!=123) ExitProcess(2);
    if (NtWriteFile(h,0,0,0,&io,"d",1,&offset,0)||io.information!=1) ExitProcess(3);
    SetFilePointer(h,0,0,FILE_BEGIN);
    if (!ReadFile(h,data,4,&n,0)||n!=4||data[0]!='a'||data[3]!='d') ExitProcess(4);
    if ((ULONG)NtWriteFile(0,0,0,0,&io,"x",1,0,0)!=0xc0000008||io.information) ExitProcess(5);
    offset=0;
    if ((ULONG)NtWriteFile(h,0,0,0,&io,"x",1,&offset,0)!=0xc00000bb||io.information) ExitProcess(6);
    if ((ULONG)NtWriteFile(h,0,0,0,&io,0,1,0,0)!=0xc000000d) ExitProcess(7);
    if (NtWriteFile(GetStdHandle(STD_OUTPUT_HANDLE),0,0,0,&io,"NT_WRITE_PASS\n",14,0,0)||io.information!=14) ExitProcess(8);
    CloseHandle(h);DeleteFileW(L"nt-write.bin");ExitProcess(0);
}
