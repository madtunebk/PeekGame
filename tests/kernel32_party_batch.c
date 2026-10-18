/* kernel32 imports of PlayFab Party and msvcp140: thread pool work/timer/wait,
   FreeLibraryWhenCallbackReturns, CreateFile2, CopyFile2, symbolic/hard links,
   TerminateThread on the caller, WindowsConcatString. */
#define _WIN32_WINNT 0x0A00
#include <windows.h>
#define CHECK(x) do{if(!(x))ExitProcess(__LINE__);}while(0)
static volatile LONG work_runs,timer_runs,wait_result=-1;static HANDLE done;
static void CALLBACK on_work(PTP_CALLBACK_INSTANCE i,PVOID c,PTP_WORK w){(void)i;(void)w;InterlockedIncrement(&work_runs);CHECK(c==(PVOID)0x1234);if(work_runs==3)SetEvent(done);}
static void CALLBACK on_timer(PTP_CALLBACK_INSTANCE i,PVOID c,PTP_TIMER t){(void)i;(void)c;(void)t;if(InterlockedIncrement(&timer_runs)==3)SetEvent(done);}
static void CALLBACK on_wait(PTP_CALLBACK_INSTANCE i,PVOID c,PTP_WAIT w,TP_WAIT_RESULT r){(void)i;(void)c;(void)w;wait_result=r;SetEvent(done);}
static void CALLBACK on_free(PTP_CALLBACK_INSTANCE i,PVOID c,PTP_WORK w){(void)w;FreeLibraryWhenCallbackReturns(i,(HMODULE)c);SetEvent(done);}
static DWORD WINAPI quits(LPVOID p){(void)p;TerminateThread(GetCurrentThread(),77);return 1;}
typedef HRESULT(WINAPI*create_t)(const WCHAR*,UINT32,void**);typedef HRESULT(WINAPI*concat_t)(void*,void*,void**);
typedef const WCHAR*(WINAPI*raw_t)(void*,UINT32*);typedef HRESULT(WINAPI*del_t)(void*);
void entry(void){
    done=CreateEventW(NULL,FALSE,FALSE,NULL);
    PTP_WORK work=CreateThreadpoolWork(on_work,(PVOID)0x1234,NULL);CHECK(work);
    SubmitThreadpoolWork(work);SubmitThreadpoolWork(work);SubmitThreadpoolWork(work);
    CHECK(WaitForSingleObject(done,5000)==WAIT_OBJECT_0);WaitForThreadpoolWorkCallbacks(work,FALSE);CHECK(work_runs==3);CloseThreadpoolWork(work);
    TP_CALLBACK_ENVIRON env;InitializeThreadpoolEnvironment(&env);CHECK(CreateThreadpoolWork(on_work,NULL,&env));
    PTP_CLEANUP_GROUP group=(PTP_CLEANUP_GROUP)1;SetThreadpoolCallbackCleanupGroup(&env,group,NULL);
    CHECK(!CreateThreadpoolWork(on_work,NULL,&env)&&GetLastError()==ERROR_NOT_SUPPORTED);

    PTP_TIMER timer=CreateThreadpoolTimer(on_timer,NULL,NULL);CHECK(timer&&!IsThreadpoolTimerSet(timer));
    ULARGE_INTEGER due;due.QuadPart=(ULONGLONG)-(10*10000LL);FILETIME ft={due.LowPart,due.HighPart};
    SetThreadpoolTimer(timer,&ft,20,0);CHECK(IsThreadpoolTimerSet(timer));
    CHECK(WaitForSingleObject(done,5000)==WAIT_OBJECT_0&&timer_runs>=3);
    WaitForThreadpoolTimerCallbacks(timer,TRUE);CHECK(!IsThreadpoolTimerSet(timer));
    LONG after=timer_runs;Sleep(100);CHECK(timer_runs==after);CloseThreadpoolTimer(timer);

    HANDLE signal=CreateEventW(NULL,TRUE,FALSE,NULL);
    PTP_WAIT wait=CreateThreadpoolWait(on_wait,NULL,NULL);CHECK(wait);
    SetThreadpoolWait(wait,signal,NULL);Sleep(50);CHECK(wait_result==-1);SetEvent(signal);
    CHECK(WaitForSingleObject(done,5000)==WAIT_OBJECT_0&&wait_result==WAIT_OBJECT_0);
    ResetEvent(signal);due.QuadPart=(ULONGLONG)-(30*10000LL);ft.dwLowDateTime=due.LowPart;ft.dwHighDateTime=due.HighPart;
    SetThreadpoolWait(wait,signal,&ft);CHECK(WaitForSingleObject(done,5000)==WAIT_OBJECT_0&&wait_result==WAIT_TIMEOUT);
    WaitForThreadpoolWaitCallbacks(wait,TRUE);CloseThreadpoolWait(wait);

    HMODULE self=LoadLibraryW(L"kernel32.dll");CHECK(self);
    PTP_WORK freeing=CreateThreadpoolWork(on_free,self,NULL);SubmitThreadpoolWork(freeing);
    CHECK(WaitForSingleObject(done,5000)==WAIT_OBJECT_0);WaitForThreadpoolWorkCallbacks(freeing,FALSE);CloseThreadpoolWork(freeing);

    CREATEFILE2_EXTENDED_PARAMETERS p={sizeof(p),FILE_ATTRIBUTE_NORMAL,0,0,NULL,NULL};
    HANDLE f=CreateFile2(L"created2.txt",GENERIC_WRITE,0,CREATE_ALWAYS,&p);CHECK(f!=INVALID_HANDLE_VALUE);
    DWORD wrote;CHECK(WriteFile(f,"party",5,&wrote,NULL)&&wrote==5&&CloseHandle(f));
    CHECK(CreateFile2(L"missing-dir\\x.txt",GENERIC_READ,0,OPEN_EXISTING,NULL)==INVALID_HANDLE_VALUE);
    COPYFILE2_EXTENDED_PARAMETERS c={sizeof(c),COPY_FILE_FAIL_IF_EXISTS,NULL,NULL,NULL};
    CHECK(CopyFile2(L"created2.txt",L"copied2.txt",&c)==S_OK);
    CHECK(CopyFile2(L"created2.txt",L"copied2.txt",&c)==HRESULT_FROM_WIN32(ERROR_FILE_EXISTS));
    c.dwCopyFlags=COPY_FILE_RESTARTABLE;CHECK(CopyFile2(L"created2.txt",L"copied3.txt",&c)==HRESULT_FROM_WIN32(ERROR_NOT_SUPPORTED));
    CHECK(CreateSymbolicLinkW(L"link.txt",L"created2.txt",SYMBOLIC_LINK_FLAG_ALLOW_UNPRIVILEGED_CREATE));
    CHECK(CreateHardLinkW(L"hard.txt",L"created2.txt",NULL));
    char got[8]={0};DWORD read;
    f=CreateFileW(L"link.txt",GENERIC_READ,0,NULL,OPEN_EXISTING,0,NULL);CHECK(f!=INVALID_HANDLE_VALUE&&ReadFile(f,got,5,&read,NULL)&&read==5&&got[0]=='p');CloseHandle(f);
    f=CreateFileW(L"hard.txt",GENERIC_READ,0,NULL,OPEN_EXISTING,0,NULL);CHECK(f!=INVALID_HANDLE_VALUE&&ReadFile(f,got,5,&read,NULL)&&read==5&&got[4]=='y');CloseHandle(f);
    CHECK(!CreateHardLinkW(L"hard.txt",L"created2.txt",NULL)&&GetLastError()==ERROR_ALREADY_EXISTS);

    HANDLE t=CreateThread(NULL,0,quits,NULL,0,NULL);CHECK(t&&WaitForSingleObject(t,5000)==WAIT_OBJECT_0);
    DWORD code=0;CHECK(GetExitCodeThread(t,&code)&&code==77);
    CHECK(!TerminateThread(t,1)||GetLastError()==ERROR_NOT_SUPPORTED);

    HMODULE winrt=LoadLibraryW(L"api-ms-win-core-winrt-string-l1-1-0.dll");CHECK(winrt);
    create_t create=(create_t)GetProcAddress(winrt,"WindowsCreateString");concat_t concat=(concat_t)GetProcAddress(winrt,"WindowsConcatString");
    raw_t raw=(raw_t)GetProcAddress(winrt,"WindowsGetStringRawBuffer");del_t del=(del_t)GetProcAddress(winrt,"WindowsDeleteString");
    CHECK(create&&concat&&raw&&del);
    void *a,*b,*joined;CHECK(create(L"Play",4,&a)==S_OK&&create(L"Fab",3,&b)==S_OK&&concat(a,b,&joined)==S_OK);
    UINT32 len;const WCHAR*text=raw(joined,&len);CHECK(len==7&&text[0]==L'P'&&text[4]==L'F'&&text[7]==0);
    void *same;CHECK(concat(a,NULL,&same)==S_OK&&raw(same,&len)&&len==4);
    del(a);del(b);del(joined);del(same);
    ExitProcess(0);
}
