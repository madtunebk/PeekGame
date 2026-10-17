#include <windows.h>
void entry(void) {
 HANDLE out=GetStdHandle(STD_OUTPUT_HANDLE), err=GetStdHandle(STD_ERROR_HANDLE);
 DWORD n=99, mode;
 CONSOLE_SCREEN_BUFFER_INFO info;
 if(GetConsoleScreenBufferInfo(out,NULL)||GetLastError()!=ERROR_INVALID_PARAMETER) ExitProcess(16);
 if(GetConsoleScreenBufferInfo((HANDLE)0x9876,&info)||GetLastError()!=ERROR_INVALID_HANDLE) ExitProcess(17);
 SetLastError(1234);
 if(!SetStdHandle(STD_OUTPUT_HANDLE,err)||GetStdHandle(STD_OUTPUT_HANDLE)!=err||GetLastError()!=1234) ExitProcess(1);
 if(!SetStdHandle(STD_OUTPUT_HANDLE,out)) ExitProcess(2);
 if(SetStdHandle(123,out)||GetLastError()!=ERROR_INVALID_PARAMETER) ExitProcess(3);
 if(!SetStdHandle(STD_OUTPUT_HANDLE,(HANDLE)0x9876)||GetStdHandle(STD_OUTPUT_HANDLE)!=(HANDLE)0x9876) ExitProcess(4);
 if(!SetStdHandle(STD_OUTPUT_HANDLE,out)) ExitProcess(5);
 if(TerminateProcess((HANDLE)0x9876,9)||GetLastError()!=ERROR_INVALID_HANDLE) ExitProcess(6);
 if(GetConsoleMode(out,&mode)) {
  if(GetConsoleScreenBufferInfo(out,&info)||GetLastError()!=ERROR_NOT_SUPPORTED) ExitProcess(18);
  DWORD original=mode;
  if(!SetConsoleMode(out,original|ENABLE_VIRTUAL_TERMINAL_PROCESSING)) ExitProcess(11);
  if(!GetConsoleMode(out,&mode)||mode!=(original|ENABLE_VIRTUAL_TERMINAL_PROCESSING)) ExitProcess(12);
  if(!SetConsoleMode(out,original)) ExitProcess(13);
  if(SetConsoleMode(out,0x80000000)||GetLastError()!=ERROR_INVALID_PARAMETER) ExitProcess(14);
  if(SetConsoleTextAttribute(out,0x4000)||GetLastError()!=ERROR_NOT_SUPPORTED) ExitProcess(20);
  if(!SetConsoleTextAttribute(out,0x1e)) ExitProcess(21);
  const WCHAR text[]={0x41,0xe9,0xd83d,0xde00};
  if(!WriteConsoleW(out,text,4,&n,NULL)||n!=4) ExitProcess(7);
  if(!SetConsoleTextAttribute(out,7)) ExitProcess(22);
 } else {
  if(GetConsoleScreenBufferInfo(out,&info)||GetLastError()!=ERROR_INVALID_HANDLE) ExitProcess(19);
  if(SetConsoleTextAttribute(out,7)||GetLastError()!=ERROR_INVALID_HANDLE) ExitProcess(23);
  if(SetConsoleMode(out,3)||GetLastError()!=ERROR_INVALID_HANDLE) ExitProcess(15);
  if(WriteConsoleW(out,L"X",1,&n,NULL)||n!=0||GetLastError()!=ERROR_INVALID_HANDLE) ExitProcess(8);
 }
 if(WriteConsoleW(out,L"X",1,&n,(void*)1)||GetLastError()!=ERROR_INVALID_PARAMETER) ExitProcess(9);
 TerminateProcess(GetCurrentProcess(),42);
 ExitProcess(10);
}
