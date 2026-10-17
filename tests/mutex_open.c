#include <windows.h>
#define CHECK(x) do{if(!(x))ExitProcess(__LINE__);}while(0)
void entry(void){
 CHECK(!OpenMutexW(SYNCHRONIZE,FALSE,L"missing")&&GetLastError()==ERROR_FILE_NOT_FOUND);
 HANDLE a=CreateMutexW(NULL,TRUE,L"test-mutex");CHECK(a);
 HANDLE b=OpenMutexW(MUTEX_ALL_ACCESS,FALSE,L"test-mutex");CHECK(b&&b!=a);
 CHECK(CloseHandle(a));CHECK(!CloseHandle(a)&&GetLastError()==ERROR_INVALID_HANDLE);
 CHECK(ReleaseMutex(b));CHECK(WaitForSingleObject(b,100)==WAIT_OBJECT_0);CHECK(ReleaseMutex(b));
 HANDLE c=CreateMutexW(NULL,FALSE,L"test-mutex");CHECK(c&&c!=b&&GetLastError()==ERROR_ALREADY_EXISTS);
 CHECK(CloseHandle(b));CHECK(WaitForSingleObject(c,100)==WAIT_OBJECT_0&&ReleaseMutex(c));CHECK(CloseHandle(c));
 CHECK(!OpenMutexW(SYNCHRONIZE,FALSE,L"test-mutex")&&GetLastError()==ERROR_FILE_NOT_FOUND);
 CHECK(!SetPriorityClass((HANDLE)0x9876,NORMAL_PRIORITY_CLASS)&&GetLastError()==ERROR_INVALID_HANDLE);
 CHECK(!SetPriorityClass(GetCurrentProcess(),0x9876)&&GetLastError()==ERROR_INVALID_PARAMETER);
 CHECK(SetPriorityClass(GetCurrentProcess(),IDLE_PRIORITY_CLASS));CHECK(GetPriorityClass(GetCurrentProcess())==IDLE_PRIORITY_CLASS);
 ExitProcess(0);
}
