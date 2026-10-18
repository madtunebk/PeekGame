/* UCRT functions PlayFab Party imports: _mkgmtime64, _difftime64, fmax, _dtest,
   __stdio_common_vsnprintf_s and __stdio_common_vsscanf. */
#include <windows.h>
#include <stdarg.h>
#include <errno.h>
#define CHECK(x) do{if(!(x))ExitProcess(__LINE__);}while(0)
typedef long long t64;
t64 __cdecl _mkgmtime64(int*);double __cdecl _difftime64(t64,t64);double __cdecl fmax(double,double);short __cdecl _dtest(double*);
int __cdecl __stdio_common_vsnprintf_s(unsigned long long,char*,size_t,size_t,const char*,void*,va_list);
int __cdecl __stdio_common_vsscanf(unsigned long long,const char*,size_t,const char*,void*,va_list);
static int snp(char*b,size_t n,size_t c,const char*f,...){va_list a;va_start(a,f);int r=__stdio_common_vsnprintf_s(0,b,n,c,f,NULL,a);va_end(a);return r;}
static int scan(unsigned long long o,const char*s,size_t n,const char*f,...){va_list a;va_start(a,f);int r=__stdio_common_vsscanf(o,s,n,f,NULL,a);va_end(a);return r;}
static int eq(const char*a,const char*b){while(*a&&*a==*b){a++;b++;}return *a==*b;}
void entry(void){
    int tm[9]={30,61,25,31,11,125,0,0,1}; /* 2025-12-31 25:61:30 -> 2026-01-01 02:01:30 */
    CHECK(_mkgmtime64(tm)==1767232890LL&&tm[5]==126&&tm[4]==0&&tm[3]==1&&tm[2]==2&&tm[1]==1&&tm[6]==4&&tm[7]==0&&tm[8]==0);
    int early[9]={0,0,0,1,0,60,0,0,0};errno=0;CHECK(_mkgmtime64(early)==-1&&errno==22);
    CHECK(_difftime64(100,40)==60.0);errno=0;CHECK(_difftime64(-1,0)==0.0&&errno==22);
    CHECK(fmax(1.0,2.0)==2.0);double nan=__builtin_nan("");CHECK(fmax(nan,3.0)==3.0);
    double inf=__builtin_inf(),zero=0.0,one=1.0;CHECK(_dtest(&inf)==1&&_dtest(&nan)==2&&_dtest(&zero)==0&&_dtest(&one)==-1);
    char b[8];
    CHECK(snp(b,sizeof(b),(size_t)-1,"%d",1234)==4&&eq(b,"1234"));
    CHECK(snp(b,sizeof(b),(size_t)-1,"%s","abcdefghij")==-1&&eq(b,"abcdefg"));
    CHECK(snp(b,sizeof(b),3,"%s","abcdef")==-1&&eq(b,"abc"));
    errno=0;CHECK(snp(b,4,10,"%s","abcdef")==-1&&b[0]==0&&errno==34);
    unsigned char ip[4];int n=0;
    CHECK(scan(0,"10.20.30.40",(size_t)-1,"%hhu.%hhu.%hhu.%hhu%n",&ip[0],&ip[1],&ip[2],&ip[3],&n)==4&&ip[0]==10&&ip[3]==40&&n==11);
    long long big=0;long l=0;unsigned u=0;double d=0;float f=0;
    CHECK(scan(0,"9000000000 -7 ff 2.5 1.25",(size_t)-1,"%I64d %ld %x %lf %f",&big,&l,&u,&d,&f)==5&&big==9000000000LL&&l==-7&&u==255&&d==2.5&&f==1.25f);
    char word[4];char rest[16];
    CHECK(scan(1,"abcdef tail",(size_t)-1,"%s %s",word,(unsigned)sizeof(word),rest,(unsigned)sizeof(rest))==2&&eq(word,"abc"));
    char set[8];CHECK(scan(0,"key=value",(size_t)-1,"%7[^=]=%7s",set,rest)==2&&eq(set,"key")&&eq(rest,"value"));
    int limited=0;CHECK(scan(0,"12345",3,"%d",&limited)==1&&limited==123);
    wchar_t wide[4];errno=0;CHECK(scan(0,"x",(size_t)-1,"%ls",wide)==-1&&errno==22);
    ExitProcess(0);
}
