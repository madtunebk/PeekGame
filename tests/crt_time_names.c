/* UCRT time-name functions msvcp140's _Locinfo calls (C locale). */
#include <windows.h>
#include <stddef.h>
#define CHECK(x) do{if(!(x))ExitProcess(__LINE__);}while(0)
char *__cdecl _Getdays(void);char *__cdecl _Getmonths(void);
wchar_t *__cdecl _W_Getdays(void);wchar_t *__cdecl _W_Getmonths(void);
void *__cdecl _Gettnames(void);void *__cdecl _W_Gettnames(void);
size_t __cdecl _Strftime(char*,size_t,const char*,const void*,void*);
void __cdecl free(void*);
struct lc_time{char*wday_abbr[7];char*wday[7];char*month_abbr[12];char*month[12];char*ampm[2];
    char*sdate;char*ldate;char*time;int caltype;long refcount;
    wchar_t*w_wday_abbr[7];wchar_t*w_wday[7];wchar_t*w_month_abbr[12];wchar_t*w_month[12];wchar_t*w_ampm[2];
    wchar_t*w_sdate;wchar_t*w_ldate;wchar_t*w_time;wchar_t*w_locale;};
_Static_assert(sizeof(struct lc_time)==704,"__crt_lc_time_data");
static int eq(const char*a,const char*b){while(*a&&*a==*b){a++;b++;}return *a==*b;}
static int weq(const wchar_t*a,const wchar_t*b){while(*a&&*a==*b){a++;b++;}return *a==*b;}
void entry(void){
    char *d=_Getdays();CHECK(d&&eq(d,":Sun:Sunday:Mon:Monday:Tue:Tuesday:Wed:Wednesday:Thu:Thursday:Fri:Friday:Sat:Saturday"));free(d);
    char *m=_Getmonths();CHECK(m&&m[0]==':'&&m[1]=='J'&&eq(m+sizeof(":Jan:January")-1,":Feb:February:Mar:March:Apr:April:May:May:Jun:June:Jul:July:Aug:August:Sep:September:Oct:October:Nov:November:Dec:December"));free(m);
    wchar_t *wd=_W_Getdays();CHECK(wd&&wd[0]==L':'&&wd[1]==L'S'&&wd[5]==L'S');free(wd);
    wchar_t *wm=_W_Getmonths();CHECK(wm&&wm[1]==L'J');free(wm);
    struct lc_time *t=_Gettnames();
    CHECK(t&&eq(t->wday_abbr[0],"Sun")&&eq(t->wday[6],"Saturday")&&eq(t->month_abbr[11],"Dec")&&eq(t->month[0],"January"));
    CHECK(eq(t->ampm[1],"PM")&&eq(t->sdate,"MM/dd/yy")&&eq(t->ldate,"dddd, MMMM dd, yyyy")&&eq(t->time,"HH:mm:ss")&&t->caltype==1);
    CHECK(weq(t->w_wday[1],L"Monday")&&weq(t->w_month[11],L"December")&&weq(t->w_time,L"HH:mm:ss")&&t->w_locale&&t->w_locale[0]==0);
    free(t);t=_W_Gettnames();CHECK(t&&weq(t->w_ampm[0],L"AM"));
    int tm[9]={5,4,3,18,9,126,0,290,0};char out[64];
    CHECK(_Strftime(out,sizeof(out),"%Y-%m-%d %H:%M:%S %a %b",tm,t)==27&&eq(out,"2026-10-18 03:04:05 Sun Oct"));
    free(t);
    ExitProcess(0);
}
