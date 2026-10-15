/* The kernel32 core an MSVC CRT uses at startup (heap, FLS, SList, SRW locks and
 * condition variables, InitOnce, WaitOnAddress, pointer encoding, time, environment,
 * locale and strings). Exit code 0 is a pass; any other code names the failed check. */
#include <windows.h>
static SLIST_HEADER list;
static SRWLOCK lock=SRWLOCK_INIT;
static CONDITION_VARIABLE ready=CONDITION_VARIABLE_INIT;
static volatile LONG stage,address_word,fls_released;
static DWORD fls;
static INIT_ONCE once=INIT_ONCE_STATIC_INIT;
static LONG once_calls;
static void WINAPI fls_callback(void *value){if(value==(void *)0x1234)InterlockedIncrement(&fls_released);}
static BOOL CALLBACK once_callback(PINIT_ONCE o,void *p,void **context){(void)o;(void)p;once_calls++;*context=(void *)0x40;return TRUE;}
static DWORD WINAPI worker(void *p){
 (void)p;
 FlsSetValue(fls,(void *)0x1234);
 AcquireSRWLockExclusive(&lock);stage=1;WakeAllConditionVariable(&ready);ReleaseSRWLockExclusive(&lock);
 LONG zero=0;if(!WaitOnAddress(&address_word,&zero,sizeof zero,5000))return 1;
 return address_word==7?0:2;
}
static int same(const WCHAR *a,const WCHAR *b){while(*a&&*a==*b){a++;b++;}return *a==*b;}
typedef struct {SLIST_ENTRY entry;int value;} __attribute__((aligned(16))) Item;
void entry(void){
 HANDLE heap=GetProcessHeap();
 char *p=HeapAlloc(heap,HEAP_ZERO_MEMORY,100);
 if(!p||p[99]||HeapSize(heap,0,p)!=100)ExitProcess(1);
 p=HeapReAlloc(heap,HEAP_ZERO_MEMORY,p,1000);
 if(!p||p[999]||HeapSize(heap,0,p)!=1000||!HeapFree(heap,0,p))ExitProcess(2);
 if(HeapFree(heap,0,(void *)&list)||GetLastError()!=ERROR_INVALID_PARAMETER)ExitProcess(3);
 static Item items[3];InitializeSListHead(&list);
 for(int i=0;i<3;i++){items[i].value=i;InterlockedPushEntrySList(&list,&items[i].entry);}
 if(QueryDepthSList(&list)!=3||((Item *)InterlockedPopEntrySList(&list))->value!=2)ExitProcess(4);
 if(((Item *)InterlockedFlushSList(&list))->value!=1||InterlockedPopEntrySList(&list))ExitProcess(5);
 void *encoded=EncodePointer((void *)entry);
 if(encoded==(void *)entry||DecodePointer(encoded)!=(void *)entry)ExitProcess(6);
 fls=FlsAlloc(fls_callback);if(fls==FLS_OUT_OF_INDEXES||!FlsSetValue(fls,(void *)5)||FlsGetValue(fls)!=(void *)5)ExitProcess(7);
 void *context=0;
 if(!InitOnceExecuteOnce(&once,once_callback,0,&context)||!InitOnceExecuteOnce(&once,once_callback,0,&context)||once_calls!=1||context!=(void *)0x40)ExitProcess(8);
 HANDLE thread=CreateThread(0,0,worker,0,0,0);if(!thread)ExitProcess(9);
 AcquireSRWLockExclusive(&lock);
 while(!stage)if(!SleepConditionVariableSRW(&ready,&lock,5000,0))ExitProcess(10);
 ReleaseSRWLockExclusive(&lock);
 Sleep(50);address_word=7;WakeByAddressAll((void *)&address_word);
 DWORD code=99;if(WaitForSingleObject(thread,5000)||!GetExitCodeThread(thread,&code)||code)ExitProcess(11);
 if(fls_released!=1||FlsGetValue(fls)!=(void *)5)ExitProcess(12);
 if(TryAcquireSRWLockShared(&lock)!=TRUE){ExitProcess(13);}ReleaseSRWLockShared(&lock);
 FILETIME ft;SYSTEMTIME st,back;GetSystemTimePreciseAsFileTime(&ft);
 if(!FileTimeToSystemTime(&ft,&st)||st.wYear<2024||!SystemTimeToFileTime(&st,(FILETIME *)&back)||((ULARGE_INTEGER *)&back)->QuadPart/10000!=((ULARGE_INTEGER *)&ft)->QuadPart/10000)ExitProcess(14);
 if(GetTickCount64()==0)ExitProcess(15);
 if(!SetEnvironmentVariableW(L"PEEK_CORE_TEST",L"value")||GetEnvironmentVariableW(L"peek_core_test",0,0)!=6)ExitProcess(16);
 WCHAR buffer[64];if(ExpandEnvironmentStringsW(L"<%PEEK_CORE_TEST%|%NOPE%>",buffer,64)!=15||!same(buffer,L"<value|%NOPE%>"))ExitProcess(17);
 WCHAR *env=GetEnvironmentStringsW();if(!env||!*env||!FreeEnvironmentStringsW(env))ExitProcess(18);
 WCHAR upper[8];if(LCMapStringW(LOCALE_USER_DEFAULT,LCMAP_UPPERCASE,L"abc",-1,upper,8)!=4||!same(upper,L"ABC"))ExitProcess(19);
 if(CompareStringW(LOCALE_USER_DEFAULT,NORM_IGNORECASE,L"Peek",-1,L"pEEK",-1)!=CSTR_EQUAL||CompareStringW(LOCALE_USER_DEFAULT,0,L"a",-1,L"b",-1)!=CSTR_LESS_THAN)ExitProcess(20);
 WORD types[3];if(!GetStringTypeW(CT_CTYPE1,L"A1 ",3,types)||!(types[0]&C1_UPPER)||!(types[1]&C1_DIGIT)||!(types[2]&C1_SPACE))ExitProcess(21);
 CPINFO cp;if(!GetCPInfo(CP_UTF8,&cp)||cp.MaxCharSize!=4||!IsValidCodePage(65001))ExitProcess(22);
 WCHAR name[LOCALE_NAME_MAX_LENGTH];if(GetUserDefaultLocaleName(name,LOCALE_NAME_MAX_LENGTH)<3)ExitProcess(23);
 ExitProcess(0);
}
