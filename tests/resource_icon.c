#include <windows.h>
#include "icon_png.h"
#define CHECK(x) do{if(!(x))ExitProcess(__LINE__);}while(0)
static LRESULT CALLBACK proc(HWND w,UINT m,WPARAM a,LPARAM b){return DefWindowProcW(w,m,a,b);}
void entry(void){
 BYTE dib[64]={0};BITMAPINFOHEADER *h=(void*)dib;h->biSize=40;h->biWidth=2;h->biHeight=4;h->biPlanes=1;h->biBitCount=32;
 DWORD *pixels=(void*)(dib+40);for(int i=0;i<4;i++)pixels[i]=0xff00ff00;
 HICON first=CreateIconFromResourceEx(dib,64,TRUE,0x30000,4,4,0);CHECK(first);
 CHECK(!CreateIconFromResourceEx(icon_png,12,TRUE,0x30000,0,0,0));
 HICON icon=CreateIconFromResourceEx(icon_png,sizeof(icon_png),TRUE,0x30000,0,0,0);CHECK(icon);
 WNDCLASSW c={0};c.lpfnWndProc=proc;c.hInstance=GetModuleHandleW(NULL);c.lpszClassName=L"ResourceIconTest";CHECK(RegisterClassW(&c));
 HWND w=CreateWindowExW(0,c.lpszClassName,L"PeekGame resource icon test",WS_OVERLAPPEDWINDOW,0,0,320,240,0,0,c.hInstance,0);CHECK(w);
 CHECK(SendMessageW(w,WM_SETICON,ICON_BIG,(LPARAM)first)==0);
 CHECK(SendMessageW(w,WM_SETICON,ICON_BIG,(LPARAM)icon)==(LRESULT)first);
 CHECK(SendMessageW(w,WM_GETICON,ICON_BIG,0)==(LRESULT)icon);
 CHECK(DestroyIcon(first));CHECK(!DestroyIcon(first)&&GetLastError()==ERROR_INVALID_ICON_HANDLE);
 ShowWindow(w,SW_SHOW);Sleep(2500);DestroyWindow(w);CHECK(DestroyIcon(icon));ExitProcess(0);
}
