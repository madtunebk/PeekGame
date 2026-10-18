/* Host check: drives the real Linux steamclient.so through the bridge's Windows ABI, in the
 * order steam_api64's SteamAPI_Init uses. Run from a normal terminal with Steam running. */
#include "steam_bridge.h"
#include <stdbool.h>
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#define WIN __attribute__((ms_abi))
#define SLOT(object,n,type) ((type)(*(void ***)(object))[n])
typedef struct {int32_t user,callback;uint8_t *param;int32_t size;} WinMsg;

/* run_frame is the RunFrame slot: 20 in SteamClient020/019, 19 in SteamClient017/014. */
static int probe(const char *version,int run_frame,const char *utils_ver,const char *user_ver,uint32_t expected_app) {
    printf("== %s (%s / %s)\n",version,utils_ver,user_ver);
    void *(WIN *create)(const char *,int *)=steam_bridge_export("CreateInterface");
    int code=-1;void *client=create(version,&code);
    if(!client) {printf("FAIL: CreateInterface(%s) code %d\n",version,code);return 1;}
    int32_t pipe=SLOT(client,0,int32_t(WIN *)(void *))(client);
    if(!pipe) {puts("FAIL: CreateSteamPipe returned 0 (Steam not reachable)");return 1;}
    int32_t user=SLOT(client,2,int32_t(WIN *)(void *,int32_t))(client,pipe);
    if(!user) {puts("FAIL: ConnectToGlobalUser returned 0");SLOT(client,1,bool(WIN *)(void *,int32_t))(client,pipe);return 1;}
    printf("pipe=%d user=%d\n",pipe,user);
    int status=0;

    if(!strcmp(version,"SteamClient020")) {
        const char *optional_versions[]={"SteamMatchMakingServers002","SteamNetworking006",
            "SteamNetworkingSockets012","SteamNetworkingUtils004"};
        for(size_t i=0;i<sizeof optional_versions/sizeof *optional_versions;i++) {
            void *object=SLOT(client,12,void *(WIN *)(void *,int32_t,int32_t,const char *))(client,user,pipe,optional_versions[i]);
            printf("%s: %s\n",optional_versions[i],object?"available through Windows ABI":"unavailable");
            if(!object)status=1;
        }
    }

    void *utils=SLOT(client,9,void *(WIN *)(void *,int32_t,const char *))(client,pipe,utils_ver);
    void *steam_user=SLOT(client,5,void *(WIN *)(void *,int32_t,int32_t,const char *))(client,user,pipe,user_ver);
    if(!utils || !steam_user) {printf("FAIL: %s / %s unavailable\n",utils_ver,user_ver);status=1;}
    else {
        uint32_t app=SLOT(utils,9,uint32_t(WIN *)(void *))(utils);
        bool logged_on=SLOT(steam_user,1,bool(WIN *)(void *))(steam_user);
        uint64_t id=0;SLOT(steam_user,2,uint64_t *(WIN *)(void *,uint64_t *))(steam_user,&id);
        int license=SLOT(steam_user,17,int(WIN *)(void *,uint64_t,uint32_t))(steam_user,id,app);
        printf("AppID=%u logged_on=%d steam_id_valid=%d UserHasLicenseForApp=%d (peer-auth query, not local ownership proof)\n",
               app,logged_on,id!=0,license);
        if(app!=expected_app)printf("FAIL: expected AppID=%u, received %u\n",expected_app,app);
        if(app!=expected_app || !logged_on || !id)status=1;

        void *friends=SLOT(client,12,void *(WIN *)(void *,int32_t,int32_t,const char *))(client,user,pipe,"SteamFriends017");
        if(!friends)friends=SLOT(client,8,void *(WIN *)(void *,int32_t,int32_t,const char *))(client,user,pipe,"SteamFriends017");
        if(friends) {
            const char *persona=SLOT(friends,0,const char *(WIN *)(void *))(friends);
            int state=SLOT(friends,2,int(WIN *)(void *))(friends);
            int count=SLOT(friends,3,int(WIN *)(void *,int))(friends,0x04);
            printf("SteamFriends017: persona='%s' state=%d friend_count=%d\n",persona?persona:"NULL",state,count);
        } else {
            puts("FAIL: SteamFriends017 unavailable");status=1;
        }

        void *controller=SLOT(client,12,void *(WIN *)(void *,int32_t,int32_t,const char *))(client,user,pipe,"SteamController007");
        if(!controller)controller=SLOT(client,26,void *(WIN *)(void *,int32_t,int32_t,const char *))(client,user,pipe,"SteamController007");
        if(controller) {
            bool c_init=SLOT(controller,0,bool(WIN *)(void *))(controller);
            printf("SteamController007: init=%d\n",c_init);
        } else {
            puts("FAIL: SteamController007 unavailable");status=1;
        }
    }
    bool (WIN *get)(int32_t,WinMsg *,int32_t *)=steam_bridge_export("Steam_BGetCallback");
    bool (WIN *release)(int32_t)=steam_bridge_export("Steam_FreeLastCallback");
    time_t end=time(NULL)+2;int delivered=0;
    while(time(NULL)<end) {
        SLOT(client,run_frame,void(WIN *)(void *))(client);
        WinMsg m;int32_t ignored=0;
        while(get(pipe,&m,&ignored)) {printf("callback %d (%d bytes)\n",m.callback,m.size);delivered++;release(pipe);}
        struct timespec pause={0,50000000};nanosleep(&pause,NULL);
    }
    printf("%d callbacks delivered\n",delivered);
    SLOT(client,4,void(WIN *)(void *,int32_t,int32_t))(client,pipe,user);
    SLOT(client,1,bool(WIN *)(void *,int32_t))(client,pipe);
    printf(status?"FAIL: %s\n":"PASS: %s client connectivity through Windows ABI (game initialization not tested)\n",version);
    return status;
}

int main(int argc,char **argv) {
    if(argc>2) {fputs("usage: host_probe [path/to/linux64/steamclient.so]\n",stderr);return 2;}
    if(!getenv("SteamAppId") && (setenv("SteamAppId","952060",1) || setenv("SteamGameId","952060",1)))return 2;
    const char *app_text=getenv("SteamAppId");
    char *end=NULL;errno=0;
    unsigned long parsed=strtoul(app_text,&end,10);
    if(errno || !*app_text || app_text[0]<'0' || app_text[0]>'9' || *end || !parsed || parsed>UINT32_MAX) {
        fputs("SteamAppId must be a nonzero decimal uint32 AppID\n",stderr);return 2;
    }
    uint32_t expected_app=(uint32_t)parsed;
    printf("Expected AppID: %u\n",expected_app);
    uint32_t steam=steam_bridge_steam_pid();
    printf("Steam client pid: %u%s\n",steam,steam?"":" (not running)");
    if(steam_bridge_open(argc==2?argv[1]:NULL))return 2;
    int status=probe("SteamClient020",20,"SteamUtils010","SteamUser021",expected_app);
    status|=probe("SteamClient019",20,"SteamUtils009","SteamUser020",expected_app);
    status|=probe("SteamClient017",19,"SteamUtils010","SteamUser021",expected_app);
    status|=probe("SteamClient014",19,"SteamUtils010","SteamUser021",expected_app);
    return status;
}
