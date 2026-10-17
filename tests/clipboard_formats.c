#include <windows.h>
#define CHECK(x) do{if(!(x))ExitProcess(__LINE__);}while(0)
void entry(void){
 SetLastError(1234);
 UINT a=RegisterClipboardFormatA("PeekGame.TestFormat");CHECK(a>=0xc000&&a<=0xffff&&GetLastError()==1234);
 CHECK(RegisterClipboardFormatW(L"peekgame.testformat")==a);
 CHECK(RegisterWindowMessageA("PEEKGAME.TestFormat")==a);
 CHECK(RegisterWindowMessageW(L"peekgame.TestFormat")==a);
 UINT b=RegisterClipboardFormatA("PeekGame.OtherFormat");CHECK(b>=0xc000&&b<=0xffff&&b!=a);
 CHECK(RegisterClipboardFormatA("format-\xc3\xa9")==RegisterClipboardFormatW(L"format-\x00e9"));
 CHECK(!RegisterClipboardFormatA(NULL)&&GetLastError()==ERROR_INVALID_PARAMETER);
 CHECK(!RegisterClipboardFormatW(NULL)&&GetLastError()==ERROR_INVALID_PARAMETER);
 CHECK(!RegisterClipboardFormatA("")&&GetLastError()==ERROR_INVALID_PARAMETER);
 WCHAR large[257];for(int i=0;i<256;i++)large[i]='a';large[256]=0;
 CHECK(!RegisterClipboardFormatW(large)&&GetLastError()==ERROR_INVALID_PARAMETER);
 CHECK(RegisterClipboardFormatA("PeekGame.TestFormat")==a);
 ExitProcess(0);
}
