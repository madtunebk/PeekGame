/* Files, directory searches, semaphores, mutexes, multi-object waits and suspended
 * thread start. Run in an empty directory. Exit 0 passes; other codes name the check. */
#include <windows.h>
static HANDLE semaphore;
static volatile LONG started;
static DWORD WINAPI worker(void *p){(void)p;started=1;return ReleaseSemaphore(semaphore,1,0)?0:1;}
static volatile LONG apc_calls;
static void CALLBACK apc(ULONG_PTR data){apc_calls+=(LONG)data;}
static DWORD WINAPI alertable_sleeper(void *p){(void)p;return SleepEx(INFINITE,TRUE)==WAIT_IO_COMPLETION?0:1;}
static int same(const WCHAR *a,const WCHAR *b){while(*a&&*a==*b){a++;b++;}return *a==*b;}
void entry(void){
 HANDLE f=CreateFileW(L"data.bin",GENERIC_READ|GENERIC_WRITE,0,0,CREATE_NEW,0,0);
 if(f==INVALID_HANDLE_VALUE)ExitProcess(1);
 DWORD n=0;if(!WriteFile(f,"hello world",11,&n,0)||n!=11)ExitProcess(2);
 LARGE_INTEGER at={.QuadPart=6},pos;if(!SetFilePointerEx(f,at,&pos,FILE_BEGIN)||pos.QuadPart!=6)ExitProcess(3);
 char buf[16]={0};if(!ReadFile(f,buf,16,&n,0)||n!=5||buf[0]!='w')ExitProcess(4);
 LARGE_INTEGER size;if(!GetFileSizeEx(f,&size)||size.QuadPart!=11)ExitProcess(5);
 OVERLAPPED ov={0};ov.Offset=0;if(!ReadFile(f,buf,5,&n,&ov)||n!=5||buf[0]!='h')ExitProcess(6);
 if(!CloseHandle(f))ExitProcess(7);
 if(CreateFileW(L"data.bin",GENERIC_WRITE,0,0,CREATE_NEW,0,0)!=INVALID_HANDLE_VALUE||GetLastError()!=ERROR_FILE_EXISTS)ExitProcess(8);
 if(CreateFileW(L"missing\\x.bin",GENERIC_READ,0,0,OPEN_EXISTING,0,0)!=INVALID_HANDLE_VALUE||GetLastError()!=ERROR_PATH_NOT_FOUND)ExitProcess(9);
 if(!CreateDirectoryW(L"sub",0)||!MoveFileExW(L"data.bin",L"sub\\moved.bin",0))ExitProcess(10);
 WIN32_FIND_DATAW fd;HANDLE find=FindFirstFileW(L"sub\\*.BIN",&fd);
 if(find==INVALID_HANDLE_VALUE||!same(fd.cFileName,L"moved.bin")||fd.nFileSizeLow!=11)ExitProcess(11);
 if(FindNextFileW(find,&fd)||GetLastError()!=ERROR_NO_MORE_FILES||!FindClose(find))ExitProcess(12);
 WIN32_FILE_ATTRIBUTE_DATA data;if(!GetFileAttributesExW(L"sub",GetFileExInfoStandard,&data)||!(data.dwFileAttributes&FILE_ATTRIBUTE_DIRECTORY))ExitProcess(13);
 if(!DeleteFileW(L"sub\\moved.bin")||!RemoveDirectoryW(L"sub"))ExitProcess(14);
 semaphore=CreateSemaphoreW(0,0,2,0);HANDLE mutex=CreateMutexW(0,TRUE,0),event=CreateEventW(0,TRUE,FALSE,0);
 if(!semaphore||!mutex||!event)ExitProcess(15);
 HANDLE thread=CreateThread(0,0,worker,0,CREATE_SUSPENDED,0);
 Sleep(50);if(!thread||started)ExitProcess(16);
 if(ResumeThread(thread)!=1)ExitProcess(17);
 HANDLE both[2]={event,semaphore};
 if(WaitForMultipleObjects(2,both,FALSE,5000)!=WAIT_OBJECT_0+1)ExitProcess(18);
 if(WaitForSingleObject(semaphore,0)!=WAIT_TIMEOUT)ExitProcess(19);
 if(WaitForSingleObject(mutex,0)!=WAIT_OBJECT_0||!ReleaseMutex(mutex)||!ReleaseMutex(mutex)||ReleaseMutex(mutex))ExitProcess(20);
 SetEvent(event);HANDLE all[2]={event,thread};
 if(WaitForMultipleObjects(2,all,TRUE,5000)!=WAIT_OBJECT_0)ExitProcess(21);
 /* File mappings: a shared view writes through to the file; a named pagefile mapping
  * opened again by name shows the same memory. */
 f=CreateFileW(L"map.bin",GENERIC_READ|GENERIC_WRITE,0,0,CREATE_NEW,0,0);
 if(f==INVALID_HANDLE_VALUE||!WriteFile(f,"abcdefgh",8,&n,0))ExitProcess(22);
 HANDLE map=CreateFileMappingW(f,0,PAGE_READWRITE,0,0,0);
 char *view=map?MapViewOfFile(map,FILE_MAP_WRITE,0,0,0):0;
 if(!view||view[2]!='c')ExitProcess(23);
 view[0]='X';if(!FlushViewOfFile(view,0)||!UnmapViewOfFile(view)||UnmapViewOfFile(view)||!CloseHandle(map))ExitProcess(24);
 at.QuadPart=0;SetFilePointerEx(f,at,0,FILE_BEGIN);
 if(!ReadFile(f,buf,8,&n,0)||buf[0]!='X'||!CloseHandle(f)||!DeleteFileW(L"map.bin"))ExitProcess(25);
 HANDLE shared=CreateFileMappingW(INVALID_HANDLE_VALUE,0,PAGE_READWRITE,0,65536,L"peekgame-test");
 HANDLE again=OpenFileMappingW(FILE_MAP_ALL_ACCESS,FALSE,L"peekgame-test");
 int *a=shared?MapViewOfFile(shared,FILE_MAP_ALL_ACCESS,0,0,0):0,*b=again?MapViewOfFile(again,FILE_MAP_READ,0,0,4096):0;
 if(!a||!b)ExitProcess(26);
 a[10]=4321;if(b[10]!=4321)ExitProcess(27);
 /* APCs run in alertable waits, which then return WAIT_IO_COMPLETION. */
 if(!QueueUserAPC(apc,GetCurrentThread(),5)||WaitForSingleObjectEx(event,1000,TRUE)!=WAIT_IO_COMPLETION||apc_calls!=5)ExitProcess(28);
 if(WaitForSingleObjectEx(event,0,TRUE)!=WAIT_OBJECT_0)ExitProcess(29); /* still set (manual reset) */
 DWORD sleeper_id;HANDLE sleeper=CreateThread(0,0,alertable_sleeper,0,0,&sleeper_id);
 HANDLE opened=OpenThread(THREAD_ALL_ACCESS,FALSE,sleeper_id);
 if(!sleeper||!opened||GetThreadId(opened)!=sleeper_id)ExitProcess(30);
 Sleep(50);if(!QueueUserAPC(apc,opened,2))ExitProcess(31);
 DWORD code=1;if(WaitForSingleObject(sleeper,5000)!=WAIT_OBJECT_0||!GetExitCodeThread(sleeper,&code)||code||apc_calls!=7)ExitProcess(32);
 ExitProcess(0);
}
