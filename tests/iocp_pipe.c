#define _WIN32_WINNT 0x0601
#include <windows.h>
#define CHECK(x) do{if(!(x))ExitProcess(__LINE__);}while(0)
void entry(void){
    HANDLE pipe=CreateFileW(L"\\\\?\\pipe\\peek-fixture",GENERIC_READ|GENERIC_WRITE,0,0,OPEN_EXISTING,FILE_FLAG_OVERLAPPED,0);CHECK(pipe!=INVALID_HANDLE_VALUE);
    HANDLE port=CreateIoCompletionPort(pipe,0,0x1234,1);CHECK(port);
    OVERLAPPED zero={0};DWORD zero_bytes=99;OVERLAPPED *zero_got;ULONG_PTR zero_key;
    CHECK(ReadFile(pipe,0,0,&zero_bytes,&zero));CHECK(zero_bytes==0);
    CHECK(GetQueuedCompletionStatus(port,&zero_bytes,&zero_key,&zero_got,2000));CHECK(zero_got==&zero&&zero_key==0x1234);
    OVERLAPPED read={0},write={0},cancelled={0};char buffer[8];DWORD bytes;ULONG_PTR key;OVERLAPPED *got;
    read.hEvent=CreateEventW(0,TRUE,FALSE,0);CHECK(read.hEvent);
    CHECK(!ReadFile(pipe,buffer,5,0,&read)&&GetLastError()==ERROR_IO_PENDING);
    CHECK(!GetOverlappedResult(pipe,&read,&bytes,FALSE)&&GetLastError()==ERROR_IO_INCOMPLETE);
    CHECK(!WriteFile(pipe,"hello",5,0,&write)&&GetLastError()==ERROR_IO_PENDING);
    int reads=0,writes=0;
    for(int i=0;i<2;i++){
        CHECK(GetQueuedCompletionStatus(port,&bytes,&key,&got,2000));CHECK(key==0x1234&&bytes==5);
        if(got==&read)reads++;else if(got==&write)writes++;else CHECK(0);
    }
    CHECK(reads==1&&writes==1&&buffer[0]=='h'&&buffer[4]=='o');
    CHECK(GetOverlappedResult(pipe,&read,&bytes,TRUE)&&bytes==5);CHECK(WaitForSingleObject(read.hEvent,0)==0);
    CHECK(!ReadFile(pipe,buffer,5,0,&cancelled)&&GetLastError()==ERROR_IO_PENDING);
    CHECK(CancelIoEx(pipe,&cancelled));
    CHECK(!GetQueuedCompletionStatus(port,&bytes,&key,&got,2000));CHECK(GetLastError()==ERROR_OPERATION_ABORTED&&got==&cancelled&&key==0x1234);
    CHECK(!GetOverlappedResult(pipe,&cancelled,&bytes,FALSE)&&GetLastError()==ERROR_OPERATION_ABORTED);
    CHECK(!CancelIoEx(pipe,&cancelled)&&GetLastError()==ERROR_NOT_FOUND);
    // The low event bit suppresses IOCP, including cancellation packets.
    OVERLAPPED quiet={0};quiet.hEvent=(HANDLE)((ULONG_PTR)read.hEvent|1);
    CHECK(ResetEvent(read.hEvent));
    CHECK(!ReadFile(pipe,buffer,1,0,&quiet)&&GetLastError()==ERROR_IO_PENDING);CHECK(CancelIoEx(pipe,&quiet));
    CHECK(WaitForSingleObject(read.hEvent,1000)==0);
    CHECK(!GetQueuedCompletionStatus(port,&bytes,&key,&got,0)&&GetLastError()==WAIT_TIMEOUT);
    OVERLAPPED closing={0};CHECK(!ReadFile(pipe,buffer,1,0,&closing)&&GetLastError()==ERROR_IO_PENDING);
    CHECK(CloseHandle(pipe));pipe=0;
    CHECK(!GetQueuedCompletionStatus(port,&bytes,&key,&got,2000)&&GetLastError()==ERROR_OPERATION_ABORTED&&got==&closing);
    CHECK(CloseHandle(read.hEvent));CHECK(CloseHandle(port));ExitProcess(0);
}
