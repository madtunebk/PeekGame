#define COBJMACROS
#include <windows.h>
#include <ole2.h>
#include <shellapi.h>
static void check(int ok,UINT code){if(!ok)ExitProcess(code);}
static LONG refs=1;static DWORD owner;static int enters,overs,leaves,drops;static volatile LONG done;
static IDataObject *retained;static int destroy_on_drop;static HWND window;
static HRESULT STDMETHODCALLTYPE qi(IDropTarget*t,REFIID i,void**o){(void)t;(void)i;*o=0;return E_NOINTERFACE;}
static ULONG STDMETHODCALLTYPE add(IDropTarget*t){(void)t;return ++refs;}
static ULONG STDMETHODCALLTYPE release(IDropTarget*t){(void)t;return --refs;}
static void inspect(IDataObject*d){
 GUID iid={0x10e,0,0,{0xc0,0,0,0,0,0,0,0x46}};void *same=0;check(IDataObject_QueryInterface(d,&iid,&same)==S_OK&&same==d,__LINE__);check(IDataObject_Release((IDataObject*)same)>0,__LINE__);
 FORMATETC format={CF_HDROP,0,DVASPECT_CONTENT,-1,TYMED_HGLOBAL};STGMEDIUM medium;
 check(IDataObject_QueryGetData(d,&format)==S_OK,__LINE__);
 check(IDataObject_GetData(d,&format,&medium)==S_OK,__LINE__);check(medium.tymed==TYMED_HGLOBAL,__LINE__);
 check(DragQueryFileW(medium.hGlobal,0xffffffff,0,0)==2,__LINE__);
 WCHAR name[128];check(DragQueryFileW(medium.hGlobal,0,name,128)==13,__LINE__);
 const WCHAR wanted[]=L"Z:\\drag-é.txt";for(unsigned i=0;i<sizeof(wanted)/2;i++)check(name[i]==wanted[i],__LINE__);
 check(DragQueryFileW(medium.hGlobal,1,0,0)==17,__LINE__);
 WCHAR tiny[3];check(DragQueryFileW(medium.hGlobal,1,tiny,3)==2&&tiny[2]==0,__LINE__);
 void *p=GlobalLock(medium.hGlobal);check(p!=0,__LINE__);check(*(DWORD*)p==20&&*(DWORD*)((char*)p+16)==1,__LINE__);GlobalUnlock(medium.hGlobal);
 ReleaseStgMedium(&medium);check(medium.tymed==0,__LINE__);
 format.cfFormat=CF_TEXT;check(IDataObject_QueryGetData(d,&format)==DV_E_FORMATETC,__LINE__);
}
static HRESULT STDMETHODCALLTYPE enter(IDropTarget*t,IDataObject*d,DWORD keys,POINTL pt,DWORD*effect){
 (void)t;(void)keys;check(GetCurrentThreadId()==owner,__LINE__);check(pt.x==115&&pt.y==126,__LINE__);inspect(d);enters++;*effect=enters==1?DROPEFFECT_COPY:DROPEFFECT_NONE;return S_OK;
}
static HRESULT STDMETHODCALLTYPE over(IDropTarget*t,DWORD keys,POINTL pt,DWORD*effect){
 (void)t;(void)keys;check(GetCurrentThreadId()==owner,__LINE__);check(pt.x==119&&pt.y==130,__LINE__);overs++;*effect=DROPEFFECT_COPY;return S_OK;
}
static HRESULT STDMETHODCALLTYPE leave(IDropTarget*t){(void)t;check(GetCurrentThreadId()==owner,__LINE__);leaves++;return S_OK;}
static HRESULT STDMETHODCALLTYPE drop(IDropTarget*t,IDataObject*d,DWORD keys,POINTL pt,DWORD*effect){
 (void)t;(void)keys;check(GetCurrentThreadId()==owner,__LINE__);check(pt.x==119&&pt.y==130,__LINE__);inspect(d);IDataObject_AddRef(d);retained=d;drops++;*effect=DROPEFFECT_COPY;if(destroy_on_drop){check(RevokeDragDrop(window)==S_OK,__LINE__);check(DestroyWindow(window),__LINE__);}return S_OK;
}
static IDropTargetVtbl vt={qi,add,release,enter,over,leave,drop};
static IDropTarget target={&vt};
static DWORD WINAPI pump_worker(void*p){(void)p;MSG m;while(!done){PeekMessageW(&m,0,0,0,PM_REMOVE);Sleep(1);}return 0;}
static LRESULT CALLBACK proc(HWND w,UINT m,WPARAM a,LPARAM b){if(m==WM_CLOSE){check(enters==2&&overs>=1&&leaves==1&&drops==1,__LINE__);check(RevokeDragDrop(w)==S_OK&&refs==1,__LINE__);check(RevokeDragDrop(w)==DRAGDROP_E_NOTREGISTERED,__LINE__);check(RegisterDragDrop(w,&target)==S_OK&&refs==2,__LINE__);DestroyWindow(w);return 0;}if(m==WM_DESTROY){PostQuitMessage(0);return 0;}return DefWindowProcW(w,m,a,b);}
void entry(void){
 char option[2];destroy_on_drop=GetEnvironmentVariableA("PEEKGAME_DROP_DESTROY",option,2)!=0;
 owner=GetCurrentThreadId();WNDCLASSW wc={0};wc.lpfnWndProc=proc;wc.lpszClassName=L"PeekDrop";check(RegisterClassW(&wc),__LINE__);
 HWND w=CreateWindowExW(0,wc.lpszClassName,L"PeekGame Xdnd OLE test",WS_OVERLAPPEDWINDOW|WS_VISIBLE,70,80,300,200,0,0,0,0);check(w!=0,__LINE__);window=w;
 check(RegisterDragDrop(w,&target)==E_OUTOFMEMORY,__LINE__);check(OleInitialize(0)==S_OK,__LINE__);check(RegisterDragDrop(w,&target)==S_OK&&refs==2,__LINE__);check(RegisterDragDrop(w,&target)==DRAGDROP_E_ALREADYREGISTERED,__LINE__);
 HANDLE thread=CreateThread(0,0,pump_worker,0,0,0);check(thread!=0,__LINE__);MSG m;
 while(GetMessageW(&m,0,0,0)>0){TranslateMessage(&m);DispatchMessageW(&m);}
 done=1;check(WaitForSingleObject(thread,5000)==WAIT_OBJECT_0,__LINE__);CloseHandle(thread);
 if(destroy_on_drop)check(enters==1&&overs>=1&&leaves==0&&drops==1,__LINE__);
 check(refs==1,__LINE__);check(retained!=0,__LINE__);inspect(retained);check(IDataObject_Release(retained)==0,__LINE__);OleUninitialize();ExitProcess(0);
}
