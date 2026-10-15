#define _WIN32_WINNT 0x0601
#include <windows.h>
#define CHECK(x) do {if(!(x)) ExitProcess(__LINE__);} while(0)
static HANDLE port, ready;
static volatile DWORD waiter_error;
static DWORD WINAPI waiter(void *unused) {
    (void)unused;DWORD bytes;ULONG_PTR key;OVERLAPPED *ov=(void*)1;
    SetEvent(ready);
    BOOL ok=GetQueuedCompletionStatus(port,&bytes,&key,&ov,INFINITE);
    waiter_error=GetLastError();
    return !ok && !ov ? 29:99;
}
static DWORD WINAPI early_exit(void *unused) {(void)unused;ExitThread(73);return 99;}
static VOID CALLBACK apc(ULONG_PTR value) {waiter_error=(DWORD)value;}
void entry(void) {
    HMODULE ktm=LoadLibraryW(L"ktmw32.dll");CHECK(ktm);
    HANDLE (WINAPI *transaction)(void*,void*,DWORD,DWORD,DWORD,DWORD,const WCHAR*)=(void*)GetProcAddress(ktm,"CreateTransaction");CHECK(transaction);
    CHECK(transaction(0,0,0,0,0,0,0)==INVALID_HANDLE_VALUE && GetLastError()==ERROR_NOT_SUPPORTED);
    CHECK(FreeLibrary(ktm));
    DWORD bytes=77,removed=0,exit_code;ULONG_PTR key=88;OVERLAPPED *got=(void*)1;
    OVERLAPPED ov={0};OVERLAPPED_ENTRY entries[3];
    CHECK(!CreateIoCompletionPort(INVALID_HANDLE_VALUE,(HANDLE)123,0,0));
    port=CreateIoCompletionPort(INVALID_HANDLE_VALUE,0,0,1);CHECK(port);
    CHECK(!GetQueuedCompletionStatus(port,&bytes,&key,&got,0));CHECK(GetLastError()==WAIT_TIMEOUT && !got);
    CHECK(PostQueuedCompletionStatus(port,42,0x123456789abcdef0ULL,&ov));
    CHECK(GetQueuedCompletionStatus(port,&bytes,&key,&got,0));CHECK(bytes==42 && key==0x123456789abcdef0ULL && got==&ov);
    CHECK(PostQueuedCompletionStatus(port,1,11,0));CHECK(PostQueuedCompletionStatus(port,2,22,(void*)0x123));
    CHECK(GetQueuedCompletionStatusEx(port,entries,3,&removed,0,FALSE));CHECK(removed==2 && entries[0].lpCompletionKey==11 && entries[1].dwNumberOfBytesTransferred==2 && entries[1].lpOverlapped==(void*)0x123);
    CHECK(QueueUserAPC(apc,GetCurrentThread(),61));
    CHECK(!GetQueuedCompletionStatusEx(port,entries,3,&removed,100,TRUE));CHECK(GetLastError()==WAIT_IO_COMPLETION && waiter_error==61);
    HANDLE alias;CHECK(DuplicateHandle(GetCurrentProcess(),port,GetCurrentProcess(),&alias,0,0,DUPLICATE_SAME_ACCESS));
    CHECK(CloseHandle(port));port=alias;
    CHECK(PostQueuedCompletionStatus(port,3,33,&ov));CHECK(GetQueuedCompletionStatus(port,&bytes,&key,&got,0));CHECK(key==33);
    // File association is shared by DuplicateHandle; sync completion posts to IOCP.
    HANDLE file=CreateFileW(L"iocp-data.bin",GENERIC_READ|GENERIC_WRITE,0,0,CREATE_ALWAYS,FILE_FLAG_OVERLAPPED,0);CHECK(file!=INVALID_HANDLE_VALUE);
    CHECK(CreateIoCompletionPort(file,port,81,0)==port);
    HANDLE file2;CHECK(DuplicateHandle(GetCurrentProcess(),file,GetCurrentProcess(),&file2,0,0,DUPLICATE_SAME_ACCESS));CHECK(CloseHandle(file));
    CHECK(WriteFile(file2,"abc",3,&bytes,&ov));CHECK(GetQueuedCompletionStatus(port,&bytes,&key,&got,0));CHECK(bytes==3 && key==81 && got==&ov);
    CHECK(SetFileCompletionNotificationModes(file2,FILE_SKIP_COMPLETION_PORT_ON_SUCCESS));
    CHECK(WriteFile(file2,"abc",3,&bytes,&ov));CHECK(!GetQueuedCompletionStatus(port,&bytes,&key,&got,0));CHECK(GetLastError()==WAIT_TIMEOUT);
    OVERLAPPED eof={0};eof.Offset=1000;
    CHECK(!ReadFile(file2,(char*)&bytes,1,&bytes,&eof)&&GetLastError()==ERROR_HANDLE_EOF);
    CHECK(!GetQueuedCompletionStatus(port,&bytes,&key,&got,0));CHECK(GetLastError()==ERROR_HANDLE_EOF && got==&eof && key==81);
    CHECK(CloseHandle(file2));CHECK(DeleteFileW(L"iocp-data.bin"));
    // ExitThread must leave its handle waitable and preserve its explicit exit code.
    HANDLE thread=CreateThread(0,0,early_exit,0,0,0);CHECK(thread);CHECK(WaitForSingleObject(thread,2000)==0);CHECK(GetExitCodeThread(thread,&exit_code) && exit_code==73);CHECK(CloseHandle(thread));
    // Closing the last port handle wakes a consumer already waiting on it.
    ready=CreateEventW(0,TRUE,FALSE,0);CHECK(ready);
    thread=CreateThread(0,0,waiter,0,0,0);CHECK(thread);CHECK(WaitForSingleObject(ready,2000)==0);Sleep(30);CHECK(CloseHandle(port));
    CHECK(WaitForSingleObject(thread,2000)==0);CHECK(GetExitCodeThread(thread,&exit_code) && exit_code==29 && waiter_error==ERROR_ABANDONED_WAIT_0);CHECK(CloseHandle(thread));CHECK(CloseHandle(ready));
    HMODULE dll=LoadLibraryW(L"exit_worker.dll");CHECK(dll);
    DWORD (WINAPI *start)(void*)=(void*)GetProcAddress(dll,"worker");CHECK(start);
    struct {HMODULE module;volatile DWORD detached;} args={dll,0};
    thread=CreateThread(0,0,start,&args,0,0);CHECK(thread);CHECK(WaitForSingleObject(thread,2000)==0);CHECK(GetExitCodeThread(thread,&exit_code) && exit_code==91 && args.detached==1);CHECK(!GetModuleHandleW(L"exit_worker.dll"));CHECK(CloseHandle(thread));
    ExitProcess(0);
}
