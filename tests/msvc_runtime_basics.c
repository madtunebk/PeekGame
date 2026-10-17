#include <windows.h>
__declspec(dllimport) char **_get_initial_narrow_environment(void);
__declspec(dllimport) double trunc(double),round(double),exp2(double);
__declspec(dllimport) float truncf(float),roundf(float),exp2f(float);
static int equal(const char *a,const char *b) {while(*a&&*a==*b){a++;b++;}return *a==*b;}
void entry(void) {
    char **env=_get_initial_narrow_environment();int found=0;
    if (!env||env!=_get_initial_narrow_environment()) ExitProcess(1);
    for (int i=0;env[i];i++) if(equal(env[i],"PEEKGAME_ENV_PROBE=initial-value")) found=1;
    if (!found) ExitProcess(2);
    if (trunc(-2.9)!=-2||trunc(2.9)!=2||round(-2.5)!=-3||round(2.5)!=3) ExitProcess(3);
    if (truncf(-2.9f)!=-2||roundf(-2.5f)!=-3||roundf(2.5f)!=3) ExitProcess(4);
    if (exp2(5)!=32||exp2(-2)!=0.25||exp2f(5)!=32||exp2f(-2)!=0.25f) ExitProcess(5);
    union {double d;unsigned long long bits;} zero={.d=trunc(-0.25)};
    if (zero.bits!=0x8000000000000000ull) ExitProcess(6);
    ExitProcess(0);
}
