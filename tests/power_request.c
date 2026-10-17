#include <windows.h>
#define CHECK(x) do {if(!(x))ExitProcess(__LINE__);}while(0)
void entry(void){
 REASON_CONTEXT c={0};c.Version=POWER_REQUEST_CONTEXT_VERSION;c.Flags=POWER_REQUEST_CONTEXT_SIMPLE_STRING;c.Reason.SimpleReasonString=L"Playing a game";
 CHECK(PowerCreateRequest(NULL)==INVALID_HANDLE_VALUE && GetLastError()==ERROR_INVALID_PARAMETER);
 HANDLE h=PowerCreateRequest(&c);CHECK(h!=INVALID_HANDLE_VALUE);
 CHECK(!PowerSetRequest(h,PowerRequestSystemRequired)&&GetLastError()==ERROR_NOT_SUPPORTED);
 CHECK(!PowerSetRequest(h,(POWER_REQUEST_TYPE)99)&&GetLastError()==ERROR_INVALID_PARAMETER);
 CHECK(!PowerClearRequest(h,PowerRequestDisplayRequired)&&GetLastError()==ERROR_NOT_SUPPORTED);
 CHECK(CloseHandle(h));CHECK(!CloseHandle(h)&&GetLastError()==ERROR_INVALID_HANDLE);
 CHECK(!PowerSetRequest(h,PowerRequestDisplayRequired)&&GetLastError()==ERROR_INVALID_HANDLE);
 c.Version=1;CHECK(PowerCreateRequest(&c)==INVALID_HANDLE_VALUE&&GetLastError()==ERROR_INVALID_PARAMETER);
 ExitProcess(0);
}
