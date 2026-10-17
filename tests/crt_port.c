#define __USE_MINGW_ANSI_STDIO 0
#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <wchar.h>
#include <math.h>
#include <time.h>
#include <io.h>
#include <direct.h>
#include <errno.h>
#include <stdarg.h>
void _lock(int);void _unlock(int);
#define CHECK(x) do{if(!(x))ExitProcess(__LINE__);}while(0)
static int compare(const void *a,const void*b){int x=*(const int*)a,y=*(const int*)b;return(x>y)-(x<y);}
static double absolute(double x){return x<0?-x:x;}
static int shared;
static DWORD WINAPI work(void *unused){(void)unused;for(int i=0;i<500;i++){_lock(8);_lock(8);shared++;_unlock(8);_unlock(8);}return 0;}
static int format(char *s,size_t n,const char*f,...){va_list a;va_start(a,f);int r=_vsnprintf(s,n,f,a);va_end(a);return r;}
static int length(const char*f,...){va_list a;va_start(a,f);int r=_vscprintf(f,a);va_end(a);return r;}
void entry(void){
 int a[]={9,1,8,2,7,3,6,4,5,0};qsort(a,10,sizeof(int),compare);for(int i=0;i<10;i++)CHECK(a[i]==i);
 for(int count=0;count<=100;count++){int data[100];for(int i=0;i<count;i++)data[i]=(i*17+3)%11;qsort(data,count,sizeof(int),compare);for(int i=1;i<count;i++)CHECK(data[i-1]<=data[i]);}
 int key=7;CHECK(*(int*)bsearch(&key,a,10,sizeof(int),compare)==7);key=50;CHECK(!bsearch(&key,a,10,sizeof(int),compare));
 char *end;errno=0;CHECK(strtol("2147483648!",&end,10)==2147483647L&&errno==ERANGE&&*end=='!');
 CHECK(_strtoi64("-9223372036854775807",NULL,10)==-9223372036854775807LL);
 CHECK(_strtoui64("18446744073709551615",NULL,10)==~0ULL);
 wchar_t *wend;CHECK(wcstol(L"-42x",&wend,10)==-42&&*wend==L'x');
 char s[100];CHECK(strcpy_s(s,sizeof s,"abc")==0&&strcat_s(s,sizeof s,"def")==0&&strcmp(s,"abcdef")==0);
 CHECK(strcpy_s(s,2,"long")==ERANGE&&s[0]==0);CHECK(_itoa_s(-255,s,sizeof s,16)==0&&strcmp(s,"ffffff01")==0);
 wchar_t wide[16];CHECK(mbstowcs(wide,"A\xc3\xa9",16)==2&&wide[1]==0xe9&&wide[2]==0);CHECK(wcscpy_s(wide,16,L"abc")==0);
 CHECK(_stricmp("AbC","aBc")==0&&_memicmp("ABcd","abXX",2)==0);
 CHECK(absolute(sin(0.5)-0.479425538604203)<1e-12&&floor(-1.2)==-2&&_hypot(3,4)==5&&ldexp(1.5,3)==12);
 CHECK(absolute(atan2f(1,1)-0.785398163f)<1e-6f&&ceilf(-1.5f)==-1);
 srand(1);CHECK(rand()==41);CHECK(rand()==18467);
 CHECK(format(s,sizeof s,"%I64d/%d/%.2f/%s",1234567890123LL,7,1.25,"ok")==23);
 CHECK(strcmp(s,"1234567890123/7/1.25/ok")==0);CHECK(length("%d:%s",42,"ok")==5);
 for(int i=0;i<8;i++)s[i]='z';CHECK(format(s,3,"abcd")==-1&&s[0]=='a'&&s[2]=='c'&&s[3]=='z');
 FILE *f=fopen("port.txt","w+b");CHECK(f);CHECK(fprintf(f,"%d/%.1f/%I64d",7,1.5,9999999999LL)==16);CHECK(fflush(f)==0);
 rewind(f);CHECK(fgets(s,100,f)&&strcmp(s,"7/1.5/9999999999")==0);
 CHECK(_chsize(_fileno(f),4)==0&&_filelengthi64(_fileno(f))==4);CHECK(fclose(f)==0);
 f=fopen("port.txt","rb");CHECK(f);FILE *reopened=NULL;CHECK(freopen_s(&reopened,"other.txt","w+b",f)==0&&reopened==f);CHECK(fputs("ok",f)>=0&&fclose(f)==0);
 wchar_t *cwd=_wgetcwd(NULL,0);CHECK(cwd&&cwd[0]=='Z');CHECK(_wchdir(cwd)==0);free(cwd);
 __time64_t t=0;struct {struct tm tm;unsigned guard;} tm;tm.guard=0xfeedabcd;
 CHECK(_gmtime64_s(&tm.tm,&t)==0&&tm.tm.tm_year==70&&tm.tm.tm_mon==0&&tm.tm.tm_mday==1&&tm.guard==0xfeedabcd);
 CHECK(_gmtime64(&t)->tm_year==70);t=-1;CHECK(_gmtime64_s(&tm.tm,&t)==EINVAL&&tm.tm.tm_year==-1);
 CHECK(_time64(NULL)>1700000000LL);clock_t start=clock();Sleep(20);CHECK(clock()-start>=10);
 HANDLE h=CreateThread(NULL,0,work,NULL,0,NULL);CHECK(h);work(NULL);CHECK(WaitForSingleObject(h,5000)==WAIT_OBJECT_0&&shared==1000);CHECK(CloseHandle(h));
 ExitProcess(0);
}
