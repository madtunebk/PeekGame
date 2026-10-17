#include <windows.h>
void entry(void) {
 SetLastError(1234);
 LANGID language=GetUserDefaultUILanguage();
 if(GetLastError()!=1234)ExitProcess(1);
 DWORD written=0;
 if(!WriteFile(GetStdHandle(STD_OUTPUT_HANDLE),&language,sizeof language,&written,NULL)||written!=sizeof language)ExitProcess(2);
 ExitProcess(0);
}
