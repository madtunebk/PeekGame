#include <windows.h>
#define CHECK(x) do{if(!(x))ExitProcess(__LINE__);}while(0)
static int calls,order[8],suppress;
static HHOOK older,newer;
static LRESULT CALLBACK first(int code,WPARAM wp,LPARAM lp){
 CHECK(code==HC_ACTION||code==HC_NOREMOVE);CHECK(wp==WM_MOUSEMOVE);
 MOUSEHOOKSTRUCT *info=(void*)lp;CHECK(info->pt.x==12&&info->pt.y==34&&info->dwExtraInfo==0);
 order[calls++]=1;return suppress?1:CallNextHookEx(older,code,wp,lp);
}
static LRESULT CALLBACK second(int code,WPARAM wp,LPARAM lp){order[calls++]=2;return CallNextHookEx(newer,code,wp,lp);}
void entry(void){
 DWORD me=GetCurrentThreadId();MSG m;
 CHECK(!SetWindowsHookExA(WH_MOUSE,NULL,NULL,me));
 CHECK(!SetWindowsHookExA(WH_KEYBOARD,first,NULL,me)&&GetLastError()==ERROR_NOT_SUPPORTED);
 CHECK(!SetWindowsHookExA(WH_MOUSE,first,NULL,0)&&GetLastError()==ERROR_NOT_SUPPORTED);
 older=SetWindowsHookExA(WH_MOUSE,first,NULL,me);CHECK(older);
 newer=SetWindowsHookExW(WH_MOUSE,second,NULL,me);CHECK(newer);
 CHECK(PostThreadMessageW(me,WM_MOUSEMOVE,0,MAKELPARAM(12,34)));
 CHECK(PeekMessageW(&m,(HWND)-1,WM_MOUSEMOVE,WM_MOUSEMOVE,PM_NOREMOVE));
 CHECK(calls==2&&order[0]==2&&order[1]==1);calls=0;
 CHECK(PeekMessageW(&m,(HWND)-1,WM_MOUSEMOVE,WM_MOUSEMOVE,PM_REMOVE));
 CHECK(calls==2&&order[0]==2&&order[1]==1);calls=0;suppress=1;
 CHECK(PostThreadMessageW(me,WM_MOUSEMOVE,0,MAKELPARAM(12,34)));
 CHECK(!PeekMessageW(&m,(HWND)-1,WM_MOUSEMOVE,WM_MOUSEMOVE,PM_REMOVE));CHECK(calls==2);
 CHECK(UnhookWindowsHookEx(newer));CHECK(UnhookWindowsHookEx(older));
 CHECK(!UnhookWindowsHookEx(older)&&GetLastError()==ERROR_INVALID_HOOK_HANDLE);
 calls=0;CHECK(PostThreadMessageW(me,WM_MOUSEMOVE,0,MAKELPARAM(12,34)));
 CHECK(PeekMessageW(&m,(HWND)-1,WM_MOUSEMOVE,WM_MOUSEMOVE,PM_REMOVE)&&calls==0);
 CHECK(CallNextHookEx(NULL,0,0,0)==0);
 ExitProcess(0);
}
