#include <windows.h>
#include <shobjidl.h>
#include <shellapi.h>
#include <ole2.h>
#define CHECK(x) do{if(!(x))ExitProcess(__LINE__);}while(0)
static LRESULT CALLBACK proc(HWND w,UINT m,WPARAM a,LPARAM b){return DefWindowProcW(w,m,a,b);}
void entry(void){
 CHECK(SetCurrentProcessExplicitAppUserModelID(NULL)==E_INVALIDARG);
 CHECK(SetCurrentProcessExplicitAppUserModelID(L"")==E_INVALIDARG);
 CHECK(SetCurrentProcessExplicitAppUserModelID(L"invalid id")==E_INVALIDARG);
 CHECK(SetCurrentProcessExplicitAppUserModelID(L"PeekGame.Test.Unicodeé")==S_OK);
 WNDCLASSW c={0};c.lpfnWndProc=proc;c.hInstance=GetModuleHandleW(NULL);c.lpszClassName=L"IdentityTest";
 CHECK(RegisterClassW(&c));
 HWND w=CreateWindowExW(0,c.lpszClassName,L"PeekGame identity test",WS_OVERLAPPEDWINDOW,0,0,320,240,0,0,c.hInstance,0);
 CHECK(w);
 CHECK(RegisterDragDrop((HWND)0x9876,(IDropTarget*)1)==DRAGDROP_E_INVALIDHWND);
 CHECK(RegisterDragDrop(w,NULL)==E_INVALIDARG);
 CHECK(RegisterDragDrop(w,(IDropTarget*)1)==E_OUTOFMEMORY);
 CHECK(RevokeDragDrop(w)==DRAGDROP_E_NOTREGISTERED);
 GUID property_iid={0};void *store=(void*)1;
 CHECK(SHGetPropertyStoreForWindow(w,&property_iid,&store)==E_NOTIMPL && store==NULL);
 CHECK(SHGetPropertyStoreForWindow(w,&property_iid,NULL)==E_POINTER);
 DragAcceptFiles(w,TRUE);
 CHECK(GetWindowLongW(w,GWL_EXSTYLE)&WS_EX_ACCEPTFILES);
 DragAcceptFiles(w,FALSE);
 CHECK(!(GetWindowLongW(w,GWL_EXSTYLE)&WS_EX_ACCEPTFILES));
 DragAcceptFiles((HWND)0x9876,TRUE);CHECK(GetLastError()==ERROR_INVALID_WINDOW_HANDLE);
 POINT pt={-1920,1440};SetLastError(1234);
 CHECK(LogicalToPhysicalPointForPerMonitorDPI(w,&pt)&&pt.x==-1920&&pt.y==1440&&GetLastError()==1234);
 CHECK(PhysicalToLogicalPointForPerMonitorDPI(w,&pt)&&pt.x==-1920&&pt.y==1440);
 CHECK(!LogicalToPhysicalPointForPerMonitorDPI(w,NULL)&&GetLastError()==ERROR_INVALID_PARAMETER);
 CHECK(!PhysicalToLogicalPointForPerMonitorDPI((HWND)0x9876,&pt)&&GetLastError()==ERROR_INVALID_WINDOW_HANDLE);
 ShowWindow(w,SW_SHOW);Sleep(4000);DestroyWindow(w);ExitProcess(0);
}
