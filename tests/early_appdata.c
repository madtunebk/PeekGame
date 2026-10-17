#include <windows.h>
#include <shlobj.h>
#define CHECK(x) do{if(!(x))ExitProcess(__LINE__);}while(0)
void entry(void){
 WCHAR env[4096],folder[260];DWORD n=GetEnvironmentVariableW(L"APPDATA",env,4096);CHECK(n&&n<4096);
 CHECK(GetFileAttributesW(env)&FILE_ATTRIBUTE_DIRECTORY);
 CHECK(SHGetFolderPathW(NULL,CSIDL_APPDATA,NULL,SHGFP_TYPE_CURRENT,folder)==S_OK);
 for(unsigned i=0;folder[i]||env[i];i++)CHECK(folder[i]==env[i]);
 char bytes[4096];int count=WideCharToMultiByte(CP_UTF8,0,env,-1,bytes,4096,NULL,NULL);CHECK(count>0);
 DWORD written;CHECK(WriteFile(GetStdHandle(STD_OUTPUT_HANDLE),bytes,count-1,&written,NULL)&&written==(DWORD)(count-1));
 ExitProcess(0);
}
