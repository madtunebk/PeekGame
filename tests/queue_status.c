/* GetQueueStatus: queued kinds (high word) and kinds added since the last call (low word). */
#include <windows.h>
#define CHECK(x) do{if(!(x))ExitProcess(__LINE__);}while(0)
void entry(void){
    MSG m;PeekMessageW(&m,NULL,0,0,PM_NOREMOVE); /* create the queue */
    GetQueueStatus(QS_ALLINPUT);
    CHECK(GetQueueStatus(QS_ALLINPUT)==0);
    CHECK(PostThreadMessageW(GetCurrentThreadId(),WM_USER+1,0,0));
    DWORD s=GetQueueStatus(QS_ALLINPUT|QS_ALLPOSTMESSAGE);
    CHECK(HIWORD(s)&QS_POSTMESSAGE&&HIWORD(s)&QS_ALLPOSTMESSAGE&&LOWORD(s)&QS_POSTMESSAGE);
    s=GetQueueStatus(QS_ALLINPUT);CHECK(HIWORD(s)&QS_POSTMESSAGE&&LOWORD(s)==0);
    CHECK(GetQueueStatus(QS_KEY)==0);
    CHECK(PeekMessageW(&m,NULL,0,0,PM_REMOVE)&&m.message==WM_USER+1);
    CHECK(HIWORD(GetQueueStatus(QS_ALLINPUT))==0);
    CHECK(GetQueueStatus(0x80000000)==0&&GetLastError()==ERROR_INVALID_FLAGS);
    ExitProcess(0);
}
