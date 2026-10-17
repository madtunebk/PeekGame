#include <windows.h>
#include <direct.h>
#include <errno.h>
#define CHECK(x) do{if(!(x))ExitProcess(__LINE__);}while(0)
void entry(void){
 WCHAR before[4096],after[4096];
 CHECK(GetCurrentDirectoryW(4096,before)>0);
 CHECK(CreateDirectoryW(L"directory-\x00e9",NULL)||GetLastError()==ERROR_ALREADY_EXISTS);
 errno=123;
 CHECK(_wchdir(L"directory-\x00e9")==0 && errno==123);
 CHECK(CreateDirectoryW(L"child",NULL)||GetLastError()==ERROR_ALREADY_EXISTS);
 CHECK(GetCurrentDirectoryW(4096,after)>0);
 CHECK(_wchdir(before)==0);
 CHECK(_wchdir(after)==0);
 CHECK(_wchdir(L"..\\directory-\x00e9\\child")==0);
 CHECK(_wchdir(before)==0);
 CHECK(_wchdir(L"missing-directory")==-1 && errno==ENOENT);
 CHECK(_wchdir(NULL)==-1 && errno==EINVAL);
 WCHAR malformed[]={0xd800,0};
 CHECK(_wchdir(malformed)==-1 && errno==EILSEQ);
 CHECK(GetCurrentDirectoryW(4096,after)>0);
 for(int i=0;before[i]||after[i];i++)CHECK(before[i]==after[i]);
 ExitProcess(0);
}
