/* ISteamRemoteStorage014 and ISteamUserStats011 through the bridge, called the way Windows
 * code calls them (ms_abi, MSVC vtable order), against fake_steamclient.so. Exit 0 or the step. */
#include "steam_bridge.h"
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#define WIN __attribute__((ms_abi))
#define SLOT(o,n,type) ((type)(*(void ***)(o))[n])
#define CHECK(step,cond) do{if(!(cond)){fprintf(stderr,"test_cloud: step %d failed: %s\n",step,#cond);return step;}}while(0)

typedef void *(WIN *Factory)(const char *,int *);
typedef void *(WIN *Getter)(void *,int32_t,int32_t,const char *);

static int remote_storage(void *client) {
    void *rs=SLOT(client,12,Getter)(client,3,7,"STEAMREMOTESTORAGE_INTERFACE_VERSION014");
    CHECK(10,rs);
    CHECK(11,SLOT(client,17,Getter)(client,3,7,"STEAMREMOTESTORAGE_INTERFACE_VERSION014")==rs);
    CHECK(12,!SLOT(client,17,Getter)(client,3,7,"STEAMREMOTESTORAGE_INTERFACE_VERSION999"));
    const char save[]="slot0 data";
    CHECK(13,SLOT(rs,0,bool(WIN *)(void *,const char *,const void *,int32_t))(rs,"profile.sav",save,sizeof save));
    CHECK(14,SLOT(rs,13,bool(WIN *)(void *,const char *))(rs,"profile.sav"));
    CHECK(15,!SLOT(rs,13,bool(WIN *)(void *,const char *))(rs,"other.sav"));
    CHECK(16,SLOT(rs,15,int32_t(WIN *)(void *,const char *))(rs,"profile.sav")==(int32_t)sizeof save);
    CHECK(17,SLOT(rs,16,int64_t(WIN *)(void *,const char *))(rs,"profile.sav")==1760000000LL);
    char back[64]={0};
    CHECK(18,SLOT(rs,1,int32_t(WIN *)(void *,const char *,void *,int32_t))(rs,"profile.sav",back,sizeof back)==(int32_t)sizeof save);
    CHECK(19,!memcmp(back,save,sizeof save));
    CHECK(20,SLOT(rs,18,int32_t(WIN *)(void *))(rs)==1);
    int32_t size=0;
    const char *name=SLOT(rs,19,const char *(WIN *)(void *,int,int32_t *))(rs,0,&size);
    CHECK(21,name && !strcmp(name,"profile.sav") && size==(int32_t)sizeof save);
    uint64_t total=0,available=0;
    CHECK(22,SLOT(rs,20,bool(WIN *)(void *,uint64_t *,uint64_t *))(rs,&total,&available));
    CHECK(23,total==1ULL<<34 && available==total-sizeof save);
    CHECK(24,SLOT(rs,21,bool(WIN *)(void *))(rs) && SLOT(rs,22,bool(WIN *)(void *))(rs));
    SLOT(rs,23,void(WIN *)(void *,bool))(rs,false);
    extern int fake_cloud_enabled_set;
    CHECK(25,fake_cloud_enabled_set==0);
    /* Async write and read: call results converted from Linux pack 4 to Windows pack 8. */
    typedef bool (WIN *CallResult)(int32_t,uint64_t,void *,int,int,bool *);
    CallResult result=(CallResult)steam_bridge_export("Steam_GetAPICallResult");
    uint64_t call=SLOT(rs,2,uint64_t(WIN *)(void *,const char *,const void *,uint32_t))(rs,"profile.sav",save,sizeof save);
    CHECK(30,call==0x1331);
    int32_t written=0;bool failed=true;
    CHECK(31,result(7,call,&written,4,1331,&failed) && !failed && written==1);
    call=SLOT(rs,3,uint64_t(WIN *)(void *,const char *,uint32_t,uint32_t))(rs,"profile.sav",0,sizeof save);
    CHECK(32,call==0x1332);
    struct {uint64_t call;int32_t result;uint32_t offset,read;uint32_t pad;} done; /* Windows layout, 24 bytes */
    _Static_assert(sizeof done==24,"Windows RemoteStorageFileReadAsyncComplete_t");
    memset(&done,0xcc,sizeof done);failed=true;
    CHECK(33,result(7,call,&done,24,1332,&failed) && !failed);
    CHECK(34,done.call==0x1332 && done.result==1 && done.offset==0 && done.read==sizeof save && done.pad==0);
    CHECK(35,!result(7,call,&done,20,1332,&failed) && failed); /* Linux size from Windows code is refused */
    memset(back,0,sizeof back);
    CHECK(36,SLOT(rs,4,bool(WIN *)(void *,uint64_t,void *,uint32_t))(rs,call,back,sizeof save) && !memcmp(back,save,sizeof save));
    CHECK(26,SLOT(rs,6,bool(WIN *)(void *,const char *))(rs,"profile.sav"));
    CHECK(27,!SLOT(rs,13,bool(WIN *)(void *,const char *))(rs,"profile.sav"));
    return 0;
}

static int user_stats(void *client) {
    void *us=SLOT(client,12,Getter)(client,3,7,"STEAMUSERSTATS_INTERFACE_VERSION011");
    CHECK(40,us && SLOT(client,13,Getter)(client,3,7,"STEAMUSERSTATS_INTERFACE_VERSION011")==us);
    /* MSVC order: slot 1/3 take float, slot 2/4 take int32. */
    CHECK(41,SLOT(us,4,bool(WIN *)(void *,const char *,int32_t))(us,"kills",42));
    CHECK(42,SLOT(us,3,bool(WIN *)(void *,const char *,float))(us,"accuracy",0.75f));
    int32_t kills=0;float accuracy=0;
    CHECK(43,SLOT(us,2,bool(WIN *)(void *,const char *,int32_t *))(us,"kills",&kills) && kills==42);
    CHECK(44,SLOT(us,1,bool(WIN *)(void *,const char *,float *))(us,"accuracy",&accuracy) && accuracy==0.75f);
    extern float fake_avg_count;extern double fake_avg_length;
    CHECK(45,SLOT(us,5,bool(WIN *)(void *,const char *,float,double))(us,"rate",3.5f,120.25));
    CHECK(46,fake_avg_count==3.5f && fake_avg_length==120.25);
    CHECK(47,SLOT(us,7,bool(WIN *)(void *,const char *))(us,"ACH_B"));
    bool done=false;
    CHECK(48,SLOT(us,6,bool(WIN *)(void *,const char *,bool *))(us,"ACH_B",&done) && done);
    extern int fake_stores;
    CHECK(49,SLOT(us,10,bool(WIN *)(void *))(us) && fake_stores==1);
    CHECK(50,SLOT(us,14,uint32_t(WIN *)(void *))(us)==3);
    CHECK(51,!strcmp(SLOT(us,15,const char *(WIN *)(void *,uint32_t))(us,2),"ACH_C"));
    double avg=0;int64_t total=0;
    CHECK(52,SLOT(us,39,bool(WIN *)(void *,const char *,double *))(us,"avg",&avg) && avg==2.5);
    CHECK(53,SLOT(us,40,bool(WIN *)(void *,const char *,int64_t *))(us,"total",&total) && total==1LL<<40);
    /* Callbacks: Linux pack 4 -> Windows pack 8. */
    typedef struct {int32_t user,callback;uint8_t *param;int32_t size;} WinMsg;
    typedef bool (WIN *Get)(int32_t,WinMsg *,int32_t *);
    typedef bool (WIN *Free)(int32_t);
    Get get=(Get)steam_bridge_export("Steam_BGetCallback");
    Free release=(Free)steam_bridge_export("Steam_FreeLastCallback");
    WinMsg m;int32_t ignored=0;
    CHECK(54,get(11,&m,&ignored) && m.callback==1103 && m.size==152);
    uint64_t game;uint32_t cur,max;
    memcpy(&game,m.param,8);memcpy(&cur,m.param+140,4);memcpy(&max,m.param+144,4);
    CHECK(55,game==782330 && m.param[8]==1 && !strcmp((char *)m.param+9,"ACH_B") && cur==7 && max==10);
    CHECK(56,release(11));
    CHECK(57,get(11,&m,&ignored) && m.callback==1102 && m.size==16);
    int32_t result;memcpy(&game,m.param,8);memcpy(&result,m.param+8,4);
    CHECK(58,game==782330 && result==1);
    CHECK(59,release(11) && !get(11,&m,&ignored));
    return 0;
}

int main(int argc,char **argv) {
    if(argc!=2 || steam_bridge_open(argv[1])) {fprintf(stderr,"usage: test_cloud fake_steamclient.so\n");return 1;}
    Factory factory=(Factory)steam_bridge_export("CreateInterface");
    int code=-1;
    void *client=factory("SteamClient020",&code);
    CHECK(2,client && code==0);
    int result=remote_storage(client);
    if(result)return result;
    result=user_stats(client);
    if(result)return result;
    puts("PASS: RemoteStorage014 sync calls, async write/read call results; UserStats011 overload order, float/double args, achievement callbacks");
    return 0;
}
