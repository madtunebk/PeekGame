#include <windows.h>
#define CHECK(x) do{if(!(x))ExitProcess(__LINE__);}while(0)
void entry(void){
 HRGN a=CreateRectRgn(0,0,100,100),b=CreateRectRgn(50,20,150,70),out=CreateRectRgn(0,0,0,0);CHECK(a&&b&&out);
 RECT r;CHECK(GetRgnBox(out,&r)==NULLREGION);
 CHECK(CombineRgn(out,a,b,RGN_AND)==SIMPLEREGION);
 CHECK(GetRgnBox(out,&r)==SIMPLEREGION&&r.left==50&&r.top==20&&r.right==100&&r.bottom==70);
 CHECK(CombineRgn(a,a,b,RGN_COPY)==SIMPLEREGION);
 CHECK(CombineRgn(out,a,b,RGN_OR)==ERROR&&GetLastError()==ERROR_NOT_SUPPORTED);
 CHECK(DeleteObject(b));CHECK(!DeleteObject(b)&&GetLastError()==ERROR_INVALID_HANDLE);
 CHECK(CombineRgn(out,a,b,RGN_COPY)==SIMPLEREGION);
 CHECK(CombineRgn(out,a,b,RGN_AND)==ERROR&&GetLastError()==ERROR_INVALID_HANDLE);
 CHECK(DeleteObject(a)&&DeleteObject(out));
 ExitProcess(0);
}
