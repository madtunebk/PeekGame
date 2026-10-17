/* Test double for linux64/steamclient.so: plain SysV/Itanium-style vtables that record
 * what the bridge forwards. Used only by test_bridge; never by the loader. */
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>

typedef struct {void **vtbl;} Object;
static char cloud_name[64];static uint8_t cloud_data[256];static int32_t cloud_size=-1;
static void *client_vtbl[42],*client017_vtbl[37],*client014_vtbl[29],*user_vtbl[32],*utils_vtbl[39];
static Object client={client_vtbl},client017={client017_vtbl},client014={client014_vtbl},user={user_vtbl},utils={utils_vtbl};

uint64_t fake_license_steam_id;uint32_t fake_license_app;
int fake_hook_severity;char fake_hook_text[64];uint32_t fake_check_result;
int fake_frees,fake_result_calls;
uint32_t fake014_ip;uint16_t fake014_port;int fake014_run_frames;
uint32_t fake017_ip,fake017_check_result;uint16_t fake017_port;int fake017_run_frames;

static void unexpected(void) {abort();}
static int32_t create_pipe(void *self) {return self==&client?7:-1;}
static int32_t connect_user(void *self,int32_t pipe) {return self==&client && pipe==7?3:0;}
static void *get_user(void *self,int32_t u,int32_t pipe,const char *v) {
    return self==&client && u==3 && pipe==7 && !strcmp(v,"SteamUser021")?&user:NULL;
}
static void *get_utils(void *self,int32_t pipe,const char *v) {
    return self==&client && pipe==7 && !strcmp(v,"SteamUtils010")?&utils:NULL;
}
/* ISteamRemoteStorage014: one in-memory cloud file. */
static void *rs_vtbl[55];static Object remote={rs_vtbl};
int fake_cloud_enabled_set=-1;
static bool rs_write(void *self,const char *file,const void *data,int32_t size) {
    if(self!=&remote || size<0 || size>(int32_t)sizeof cloud_data)return false;
    snprintf(cloud_name,sizeof cloud_name,"%s",file);memcpy(cloud_data,data,size);cloud_size=size;return true;
}
static int32_t rs_read(void *self,const char *file,void *data,int32_t size) {
    if(self!=&remote || cloud_size<0 || strcmp(file,cloud_name))return 0;
    int32_t n=size<cloud_size?size:cloud_size;memcpy(data,cloud_data,n);return n;
}
static uint64_t rs_write_async(void *self,const char *file,const void *data,uint32_t size) {
    return rs_write(self,file,data,(int32_t)size)?0x1331:0;
}
static uint64_t rs_read_async(void *self,const char *file,uint32_t offset,uint32_t size) {
    return self==&remote && !strcmp(file,cloud_name) && offset==0 && size<=(uint32_t)cloud_size?0x1332:0;
}
static bool rs_read_complete(void *self,uint64_t call,void *buffer,uint32_t size) {
    if(self!=&remote || call!=0x1332 || size>(uint32_t)cloud_size)return false;
    memcpy(buffer,cloud_data,size);return true;
}
static bool rs_delete(void *self,const char *file) {
    if(self!=&remote || strcmp(file,cloud_name))return false;
    cloud_size=-1;cloud_name[0]=0;return true;
}
static bool rs_exists(void *self,const char *file) {return self==&remote && cloud_size>=0 && !strcmp(file,cloud_name);}
static int32_t rs_size(void *self,const char *file) {return rs_exists(self,file)?cloud_size:0;}
static int64_t rs_timestamp(void *self,const char *file) {return rs_exists(self,file)?1760000000LL:0;}
static int32_t rs_count(void *self) {return self==&remote && cloud_size>=0?1:0;}
static const char *rs_name_and_size(void *self,int index,int32_t *size) {
    if(self!=&remote || index!=0 || cloud_size<0)return "";
    *size=cloud_size;return cloud_name;
}
static bool rs_quota(void *self,uint64_t *total,uint64_t *available) {
    if(self!=&remote)return false;
    *total=1ULL<<34;*available=(1ULL<<34)-(cloud_size>0?(uint64_t)cloud_size:0);return true;
}
static bool rs_enabled(void *self) {return self==&remote;}
static void rs_set_enabled(void *self,bool enabled) {if(self==&remote)fake_cloud_enabled_set=enabled;}
static void *get_remote(void *self,int32_t u,int32_t pipe,const char *v) {
    return self==&client && u==3 && pipe==7 && !strcmp(v,"STEAMREMOTESTORAGE_INTERFACE_VERSION014")?&remote:NULL;
}
/* ISteamUserStats011 in GCC (declaration) order: int32 overloads before float ones. */
static void *us_vtbl[43];static Object stats={us_vtbl};
int32_t fake_stat_int;float fake_stat_float;float fake_avg_count;double fake_avg_length;int fake_stores;
char fake_achievement[64];
static bool us_get_int(void *self,const char *name,int32_t *v) {if(self!=&stats || strcmp(name,"kills"))return false;*v=fake_stat_int;return true;}
static bool us_get_float(void *self,const char *name,float *v) {if(self!=&stats || strcmp(name,"accuracy"))return false;*v=fake_stat_float;return true;}
static bool us_set_int(void *self,const char *name,int32_t v) {if(self!=&stats || strcmp(name,"kills"))return false;fake_stat_int=v;return true;}
static bool us_set_float(void *self,const char *name,float v) {if(self!=&stats || strcmp(name,"accuracy"))return false;fake_stat_float=v;return true;}
static bool us_avg(void *self,const char *name,float count,double length) {
    if(self!=&stats || strcmp(name,"rate"))return false;
    fake_avg_count=count;fake_avg_length=length;return true;
}
static bool us_get_achievement(void *self,const char *name,bool *done) {
    if(self!=&stats)return false;
    *done=!strcmp(name,fake_achievement);return true;
}
static bool us_set_achievement(void *self,const char *name) {
    if(self!=&stats)return false;
    snprintf(fake_achievement,sizeof fake_achievement,"%s",name);return true;
}
static bool us_store(void *self) {if(self==&stats)fake_stores++;return self==&stats;}
static uint32_t us_count(void *self) {return self==&stats?3:0;}
static const char *us_name(void *self,uint32_t i) {return self==&stats && i<3?(const char *[]){"ACH_A","ACH_B","ACH_C"}[i]:"";}
static bool us_global_int64(void *self,const char *name,int64_t *v) {if(self!=&stats || strcmp(name,"total"))return false;*v=1LL<<40;return true;}
static bool us_global_double(void *self,const char *name,double *v) {if(self!=&stats || strcmp(name,"avg"))return false;*v=2.5;return true;}
static void *get_stats(void *self,int32_t u,int32_t pipe,const char *v) {
    return self==&client && u==3 && pipe==7 && !strcmp(v,"STEAMUSERSTATS_INTERFACE_VERSION011")?&stats:NULL;
}
static void *mm_vtbl[38];static Object matchmaking={mm_vtbl};
uint64_t fake_mm_lobby;int fake_mm_type,fake_mm_members;char fake_mm_key[32],fake_mm_value[32];
static uint64_t mm_list(void *self) {return self==&matchmaking?0x123456789ULL:0;}
static uint64_t mm_lobby_by_index(void *self,int index) {return self==&matchmaking && index==0?0xdeadbeefULL:0;}
static uint64_t mm_lobby_owner(void *self,uint64_t lobby) {return self==&matchmaking && lobby==0xdeadbeefULL?0x0110000100000001ULL:0;}
static uint64_t mm_create(void *self,int type,int members) {
    if(self!=&matchmaking)return 0;
    fake_mm_type=type;fake_mm_members=members;return 0x987654321ULL;
}
static const char *mm_get_data(void *self,uint64_t lobby,const char *key) {
    return self==&matchmaking && lobby==fake_mm_lobby && !strcmp(key,fake_mm_key)?fake_mm_value:"";
}
static bool mm_set_data(void *self,uint64_t lobby,const char *key,const char *value) {
    if(self!=&matchmaking)return false;
    fake_mm_lobby=lobby;
    snprintf(fake_mm_key,sizeof fake_mm_key,"%s",key);
    snprintf(fake_mm_value,sizeof fake_mm_value,"%s",value);
    return true;
}
static void *get_matchmaking(void *self,int32_t u,int32_t pipe,const char *v) {
    return self==&client && u==3 && pipe==7 && !strcmp(v,"SteamMatchMaking009")?&matchmaking:NULL;
}
static void *get_generic(void *self,int32_t u,int32_t pipe,const char *v) {
    if(!strcmp(v,"SteamMatchMaking009"))return get_matchmaking(self,u,pipe,v);
    if(!strcmp(v,"STEAMUSERSTATS_INTERFACE_VERSION011"))return get_stats(self,u,pipe,v);
    if(!strcmp(v,"STEAMREMOTESTORAGE_INTERFACE_VERSION014"))return get_remote(self,u,pipe,v);
    if(!strcmp(v,"SteamUtils010"))return get_utils(self,pipe,v);
    if(!strcmp(v,"SteamClient017") || !strcmp(v,"SteamClient014"))return &client; /* one Linux object, several versions */
    return get_user(self,u,pipe,v);
}
static void set_hook(void *self,void (*hook)(int,const char *)) {(void)self;hook(2,"warning from Linux");}
static void set_check(void *self,uint32_t (*check)(int)) {(void)self;fake_check_result=check(703);}
/* SteamClient017: its own pipe/user numbers so the tests can tell which object answered. */
static int32_t create_pipe017(void *self) {return self==&client017?8:-1;}
static int32_t connect_user017(void *self,int32_t pipe) {return self==&client017 && pipe==8?4:0;}
static void *get_user017(void *self,int32_t u,int32_t pipe,const char *v) {
    return self==&client017 && u==4 && pipe==8 && !strcmp(v,"SteamUser021")?&user:NULL;
}
static void set_ip017(void *self,uint32_t ip,uint16_t port) {if(self==&client017) {fake017_ip=ip;fake017_port=port;}}
static void *get_utils017(void *self,int32_t pipe,const char *v) {
    return self==&client017 && pipe==8 && !strcmp(v,"SteamUtils010")?&utils:NULL;
}
static void run_frame017(void *self) {if(self==&client017)fake017_run_frames++;}
static uint32_t ipc_count017(void *self) {return self==&client017?17017:0;}
static void set_hook017(void *self,void (*hook)(int,const char *)) {if(self==&client017)hook(1,"warning from 017");}
static bool shutdown017(void *self) {return self==&client017;}
static void set_check017(void *self,uint32_t (*check)(int)) {if(self==&client017)fake017_check_result=check(704);}
/* SteamClient014 */
static int32_t create_pipe014(void *self) {return self==&client014?9:-1;}
static int32_t connect_user014(void *self,int32_t pipe) {return self==&client014 && pipe==9?5:0;}
static void *get_user014(void *self,int32_t u,int32_t pipe,const char *v) {
    return self==&client014 && u==5 && pipe==9 && !strcmp(v,"SteamUser021")?&user:NULL;
}
static void set_ip014(void *self,uint32_t ip,uint16_t port) {if(self==&client014) {fake014_ip=ip;fake014_port=port;}}
static void *get_utils014(void *self,int32_t pipe,const char *v) {
    return self==&client014 && pipe==9 && !strcmp(v,"SteamUtils010")?&utils:NULL;
}
static void run_frame014(void *self) {if(self==&client014)fake014_run_frames++;}
static void set_hook014(void *self,void (*hook)(int,const char *)) {if(self==&client014)hook(3,"warning from 014");}
static int32_t user_handle(void *self) {return self==&user?3:-1;}
static bool logged_on(void *self) {return self==&user;}
static uint64_t steam_id(void *self) {return self==&user?0x0110000100000001ULL:0;}
static int has_license(void *self,uint64_t id,uint32_t app) {
    if(self!=&user)return -1;
    fake_license_steam_id=id;fake_license_app=app;return 0;
}
static uint32_t app_id(void *self) {return self==&utils?952060:0;}

static void fill_validate(uint8_t *p) {
    uint64_t id=0x0110000100000001ULL,owner=0x0110000100000002ULL;int32_t response=5;
    memcpy(p,&id,8);memcpy(p+8,&response,4);memcpy(p+12,&owner,8);
}
static bool result(uint64_t call,void *data,int size,int expected,bool *failed) {
    fake_result_calls++;
    if(call==42 && expected==143 && size==20) {fill_validate(data);*failed=false;return true;}
    if(call==0x1331 && expected==1331 && size==4) {int32_t ok=1;memcpy(data,&ok,4);*failed=false;return true;}
    if(call==0x1332 && expected==1332 && size==20) { /* Linux pack 4: call, result, offset, read */
        uint64_t c=0x1332;int32_t ok=1;uint32_t offset=0,read=(uint32_t)cloud_size;
        memcpy(data,&c,8);memcpy((uint8_t *)data+8,&ok,4);memcpy((uint8_t *)data+12,&offset,4);memcpy((uint8_t *)data+16,&read,4);
        *failed=false;return true;
    }
    *failed=true;return false;
}
static bool utils_result(void *self,uint64_t call,void *data,int size,int expected,bool *failed) {
    return self==&utils && result(call,data,size,expected,failed);
}

static void init(void) {
    for(int i=0;i<42;i++)client_vtbl[i]=(void *)unexpected;
    for(int i=0;i<37;i++)client017_vtbl[i]=(void *)unexpected;
    for(int i=0;i<29;i++)client014_vtbl[i]=(void *)unexpected;
    client014_vtbl[0]=(void *)create_pipe014;client014_vtbl[2]=(void *)connect_user014;
    client014_vtbl[5]=(void *)get_user014;client014_vtbl[7]=(void *)set_ip014;client014_vtbl[9]=(void *)get_utils014;
    client014_vtbl[19]=(void *)run_frame014;client014_vtbl[21]=(void *)set_hook014;
    for(int i=0;i<32;i++)user_vtbl[i]=(void *)unexpected;
    for(int i=0;i<39;i++)utils_vtbl[i]=(void *)unexpected;
    for(int i=0;i<38;i++)mm_vtbl[i]=(void *)unexpected;
    mm_vtbl[4]=(void *)mm_list;mm_vtbl[12]=(void *)mm_lobby_by_index;
    mm_vtbl[13]=(void *)mm_create;mm_vtbl[35]=(void *)mm_lobby_owner;
    mm_vtbl[19]=(void *)mm_get_data;mm_vtbl[20]=(void *)mm_set_data;
    client_vtbl[0]=(void *)create_pipe;client_vtbl[2]=(void *)connect_user;client_vtbl[5]=(void *)get_user;
    client_vtbl[9]=(void *)get_utils;client_vtbl[10]=(void *)get_matchmaking;client_vtbl[12]=(void *)get_generic;client_vtbl[22]=(void *)set_hook;
    client_vtbl[34]=(void *)set_check;
    client017_vtbl[0]=(void *)create_pipe017;client017_vtbl[2]=(void *)connect_user017;
    client017_vtbl[5]=(void *)get_user017;client017_vtbl[7]=(void *)set_ip017;client017_vtbl[9]=(void *)get_utils017;
    client017_vtbl[19]=(void *)run_frame017;client017_vtbl[20]=(void *)ipc_count017;client017_vtbl[21]=(void *)set_hook017;
    client017_vtbl[22]=(void *)shutdown017;client017_vtbl[33]=(void *)set_check017;
    user_vtbl[0]=(void *)user_handle;user_vtbl[1]=(void *)logged_on;user_vtbl[2]=(void *)steam_id;
    user_vtbl[17]=(void *)has_license;
    utils_vtbl[9]=(void *)app_id;utils_vtbl[13]=(void *)utils_result;
    for(int i=0;i<55;i++)rs_vtbl[i]=(void *)unexpected;
    rs_vtbl[0]=(void *)rs_write;rs_vtbl[1]=(void *)rs_read;rs_vtbl[2]=(void *)rs_write_async;rs_vtbl[3]=(void *)rs_read_async;
    rs_vtbl[4]=(void *)rs_read_complete;rs_vtbl[6]=(void *)rs_delete;rs_vtbl[13]=(void *)rs_exists;rs_vtbl[15]=(void *)rs_size;
    rs_vtbl[16]=(void *)rs_timestamp;rs_vtbl[18]=(void *)rs_count;rs_vtbl[19]=(void *)rs_name_and_size;rs_vtbl[20]=(void *)rs_quota;
    rs_vtbl[21]=(void *)rs_enabled;rs_vtbl[22]=(void *)rs_enabled;rs_vtbl[23]=(void *)rs_set_enabled;
    client_vtbl[17]=(void *)get_remote;
    for(int i=0;i<43;i++)us_vtbl[i]=(void *)unexpected;
    us_vtbl[1]=(void *)us_get_int;us_vtbl[2]=(void *)us_get_float;us_vtbl[3]=(void *)us_set_int;us_vtbl[4]=(void *)us_set_float;
    us_vtbl[5]=(void *)us_avg;us_vtbl[6]=(void *)us_get_achievement;us_vtbl[7]=(void *)us_set_achievement;
    us_vtbl[10]=(void *)us_store;us_vtbl[14]=(void *)us_count;us_vtbl[15]=(void *)us_name;
    us_vtbl[39]=(void *)us_global_int64;us_vtbl[40]=(void *)us_global_double;
    client_vtbl[13]=(void *)get_stats;
}

void *CreateInterface(const char *version,int *code) {
    if(!client_vtbl[0])init();
    void *object=!strcmp(version,"SteamClient020")?&client:!strcmp(version,"SteamClient017")?&client017:
                 !strcmp(version,"SteamClient014")?&client014:NULL;
    if(code)*code=object?0:1;
    return object;
}

#pragma pack(push,4)
typedef struct {int32_t user,callback;uint8_t *param;int32_t size;} CallbackMsg;
#pragma pack(pop)
/* Queue: converted 143, unknown id, converted 304, 143 with a wrong size. */
static int step;
static uint8_t payload[32];
/* Pipe 11: achievement callbacks in Linux pack 4 (UserAchievementStored_t 148, UserStatsStored_t 12). */
static int stats_step;
static uint8_t stats_payload[160];
static bool stats_callback(CallbackMsg *m) {
    memset(stats_payload,0,sizeof stats_payload);m->user=3;m->param=stats_payload;
    uint64_t game=782330;int32_t ok=1;uint32_t cur=7,max=10;
    switch(stats_step++) {
    case 0:memcpy(stats_payload,&game,8);stats_payload[8]=1;memcpy(stats_payload+9,"ACH_B",6);
        memcpy(stats_payload+140,&cur,4);memcpy(stats_payload+144,&max,4);m->callback=1103;m->size=148;return true;
    case 1:memcpy(stats_payload,&game,8);memcpy(stats_payload+8,&ok,4);m->callback=1102;m->size=12;return true;
    default:return false;
    }
}
bool Steam_BGetCallback(int32_t pipe,CallbackMsg *m,int32_t *ignored) {
    (void)ignored;if(pipe==11)return stats_callback(m);
    if(pipe!=7)return false;
    memset(payload,0,sizeof payload);m->user=3;m->param=payload;
    switch(step++) {
    case 0:m->callback=143;m->size=20;fill_validate(payload);return true;
    case 1:m->callback=9999;m->size=4;return true;
    case 2:{uint64_t id=0x0110000100000003ULL;int32_t flags=0x20;memcpy(payload,&id,8);memcpy(payload+8,&flags,4);
        m->callback=304;m->size=12;return true;}
    case 3:m->callback=143;m->size=24;return true;
    default:return false;
    }
}
bool Steam_FreeLastCallback(int32_t pipe) {fake_frees++;return pipe==7 || pipe==11;}
bool Steam_GetAPICallResult(int32_t pipe,uint64_t call,void *data,int size,int expected,bool *failed) {
    return pipe==7 && result(call,data,size,expected,failed);
}
void Steam_ReleaseThreadLocalMemory(bool exit) {(void)exit;}
