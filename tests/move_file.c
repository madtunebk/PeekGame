#include <windows.h>
static void check(int ok,UINT code){if(!ok)ExitProcess(code);}
static void write_file(const WCHAR *path,const char *data){HANDLE f=CreateFileW(path,GENERIC_WRITE,0,0,CREATE_ALWAYS,0,0);DWORD n;check(f!=INVALID_HANDLE_VALUE,__LINE__);check(WriteFile(f,data,1,&n,0)&&n==1,__LINE__);CloseHandle(f);}
static char read_file(const WCHAR *path){char b=0;DWORD n;HANDLE f=CreateFileW(path,GENERIC_READ,FILE_SHARE_READ,0,OPEN_EXISTING,0,0);check(f!=INVALID_HANDLE_VALUE,__LINE__);check(ReadFile(f,&b,1,&n,0)&&n==1,__LINE__);CloseHandle(f);return b;}
void entry(void){
 write_file(L"source-é", "A");check(MoveFileW(L"source-é",L"renamed-漢"),__LINE__);check(read_file(L"renamed-漢")=='A',__LINE__);
 check(GetFileAttributesW(L"source-é")==INVALID_FILE_ATTRIBUTES,__LINE__);
 write_file(L"source-é", "B");check(!MoveFileW(L"source-é",L"renamed-漢")&&GetLastError()==ERROR_ALREADY_EXISTS,__LINE__);
 check(read_file(L"source-é")=='B'&&read_file(L"renamed-漢")=='A',__LINE__);
 check(!MoveFileW(L"source-é",L"dangling-link")&&GetLastError()==ERROR_ALREADY_EXISTS,__LINE__);
 check(!MoveFileW(L"absent",L"new-name")&&GetLastError()==ERROR_FILE_NOT_FOUND,__LINE__);
 check(!MoveFileW(L"source-é",L"absent-parent\\child")&&GetLastError()==ERROR_PATH_NOT_FOUND,__LINE__);
 check(!MoveFileW(0,L"new-name")&&GetLastError()==ERROR_INVALID_PARAMETER,__LINE__);
 WCHAR bad[]={0xd800,0};check(!MoveFileW(bad,L"new-name")&&GetLastError()==ERROR_NO_UNICODE_TRANSLATION,__LINE__);
 check(CreateDirectoryW(L"dir-é",0),__LINE__);check(MoveFileW(L"dir-é",L"dir-漢"),__LINE__);
 WCHAR cwd[1024],src[1100];DWORD len=GetCurrentDirectoryW(1024,cwd);check(len>0&&len<1024,__LINE__);
 unsigned i;for(i=0;i<len;i++)src[i]=cwd[i];src[i++]=L'\\';const WCHAR tail[]=L"source-é";for(unsigned j=0;j<sizeof(tail)/sizeof(*tail);j++)src[i++]=tail[j];
 check(MoveFileW(src,L"absolute-moved"),__LINE__);check(read_file(L"absolute-moved")=='B',__LINE__);
 write_file(L"replace-source","C");check(MoveFileExW(L"replace-source",L"renamed-漢",MOVEFILE_REPLACE_EXISTING),__LINE__);check(read_file(L"renamed-漢")=='C',__LINE__);
 ExitProcess(0);
}
