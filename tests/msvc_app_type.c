#include <windows.h>
__declspec(dllimport) void _set_app_type(int);
__declspec(dllimport) int _query_app_type(void);
void entry(void) {
    if (_query_app_type()!=0) ExitProcess(1);
    for (int type=0;type<=2;type++) {
        _set_app_type(type);
        if (_query_app_type()!=type) ExitProcess(2);
    }
    HMODULE crt=LoadLibraryA("ucrtbase.dll");
    void (*legacy)(int)=(void *)GetProcAddress(crt,"__set_app_type");
    if (!legacy) ExitProcess(3);
    legacy(1);
    if (_query_app_type()!=1) ExitProcess(4);
    ExitProcess(0);
}
