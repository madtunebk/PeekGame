#include <windows.h>
typedef void *(*Factory)(const char *, int *);
typedef int (*Pipe)(void *);
void entry(void) {
 char mode[16];
 GetEnvironmentVariableA("PEEK_STEAM_TEST_MISSING", mode, sizeof mode);
 if (GetModuleHandleA("steamclient64.dll")) ExitProcess(1);
 HMODULE m=LoadLibraryA("steamclient64.dll");
 if (mode[0]=='1') { if(m || GetLastError()!=126) ExitProcess(2); ExitProcess(0); }
 if (!m || GetModuleHandleA("STEAMCLIENT64.DLL")!=m) ExitProcess(3);
 Factory f=(Factory)GetProcAddress(m,"CreateInterface");
 if(!f) ExitProcess(4);
 const char *names[]={"SteamClient020","SteamClient017","SteamClient014"};
 for(int i=0;i<3;i++) {
  int code=-1; void *o=f(names[i],&code);
  if(!o || code) ExitProcess(5);
  void **vt=*(void ***)o;
  if(((Pipe)vt[0])(o)!=7+i) ExitProcess(6);
 }
 int code=0;
 if(f("SteamClient999",&code) || !code) ExitProcess(7);
 if(GetProcAddress(m,"not_a_steam_export") || GetLastError()!=127) ExitProcess(8);
 if(GetProcAddress(m,(const char *)1) || GetLastError()!=127) ExitProcess(9);
 if(!FreeLibrary(m)) ExitProcess(10);
 ExitProcess(0);
}
