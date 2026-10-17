/* Calls the bridge exactly as Windows x64 code would (ms_abi, MSVC vtables) against
 * fake_steamclient.so, and checks what reached the Linux side. */
#undef NDEBUG
#include "steam_bridge.h"
#include <assert.h>
#include <dlfcn.h>
#include <signal.h>
#include <stdbool.h>
#include <stdio.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>

#define WIN __attribute__((ms_abi))
#define SLOT(object,n,type) ((type)(*(void ***)(object))[n])

static int hook_severity;static char hook_text[64];
static void WIN windows_hook(int severity,const char *text) {hook_severity=severity;snprintf(hook_text,sizeof hook_text,"%s",text);}
static uint32_t WIN windows_check(int callback) {return (uint32_t)callback+1;}

typedef struct {int32_t user,callback;uint8_t *param;int32_t size;} WinMsg;

int main(int argc,char **argv) {
    assert(argc==2);
    void *fake=dlopen(argv[1],RTLD_NOW|RTLD_NOLOAD);
    assert(!fake);
    assert(!steam_bridge_open(argv[1]));
    fake=dlopen(argv[1],RTLD_NOW|RTLD_NOLOAD);
    assert(fake);

    void *(WIN *create)(const char *,int *)=steam_bridge_export("CreateInterface");
    assert(create && !steam_bridge_export("Steam_IsKnownInterface"));
    int code=-1;
    assert(!create("SteamClient016",&code) && code==1);
    void *client=create("SteamClient020",&code);
    assert(client && code==0);
    void *(*linux_create)(const char *,int *)=(void *(*)(const char *,int *))dlsym(fake,"CreateInterface");
    assert(client!=linux_create("SteamClient020",NULL)); /* never the Linux object */

    int32_t pipe=SLOT(client,0,int32_t(WIN *)(void *))(client);
    int32_t user=SLOT(client,2,int32_t(WIN *)(void *,int32_t))(client,pipe);
    assert(pipe==7 && user==3);
    void *steam_user=SLOT(client,5,void *(WIN *)(void *,int32_t,int32_t,const char *))(client,user,pipe,"SteamUser021");
    assert(steam_user);
    assert(steam_user==SLOT(client,12,void *(WIN *)(void *,int32_t,int32_t,const char *))(client,user,pipe,"SteamUser021"));
    assert(!SLOT(client,5,void *(WIN *)(void *,int32_t,int32_t,const char *))(client,user,pipe,"SteamUser023"));
    assert(!SLOT(client,8,void *(WIN *)(void *,int32_t,int32_t,const char *))(client,user,pipe,"SteamFriends018"));
    void *utils=SLOT(client,9,void *(WIN *)(void *,int32_t,const char *))(client,pipe,"SteamUtils010");
    assert(utils && SLOT(utils,9,uint32_t(WIN *)(void *))(utils)==952060);

    assert(SLOT(steam_user,0,int32_t(WIN *)(void *))(steam_user)==3);
    assert(SLOT(steam_user,1,bool(WIN *)(void *))(steam_user));
    uint64_t id=0;
    assert(SLOT(steam_user,2,uint64_t *(WIN *)(void *,uint64_t *))(steam_user,&id)==&id); /* MSVC hidden return */
    assert(id==0x0110000100000001ULL);
    assert(SLOT(steam_user,17,int(WIN *)(void *,uint64_t,uint32_t))(steam_user,id,952060)==0);
    assert(*(uint64_t *)dlsym(fake,"fake_license_steam_id")==id && *(uint32_t *)dlsym(fake,"fake_license_app")==952060);

    SLOT(client,22,void(WIN *)(void *,void(WIN *)(int,const char *)))(client,windows_hook);
    assert(hook_severity==2 && !strcmp(hook_text,"warning from Linux"));
    SLOT(client,34,void(WIN *)(void *,uint32_t(WIN *)(int)))(client,windows_check);
    assert(*(uint32_t *)dlsym(fake,"fake_check_result")==704);

    bool (WIN *get)(int32_t,WinMsg *,int32_t *)=steam_bridge_export("Steam_BGetCallback");
    bool (WIN *release)(int32_t)=steam_bridge_export("Steam_FreeLastCallback");
    WinMsg m;int32_t ignored=0;uint64_t owner;int32_t value;
    assert(get(7,&m,&ignored) && m.callback==143 && m.size==24);
    memcpy(&id,m.param,8);memcpy(&value,m.param+8,4);memcpy(&owner,m.param+16,8);
    assert(id==0x0110000100000001ULL && value==5 && owner==0x0110000100000002ULL);
    assert(release(7));
    assert(get(7,&m,&ignored) && m.callback==304 && m.size==16); /* unknown 9999 was dropped */
    memcpy(&id,m.param,8);memcpy(&value,m.param+8,4);
    assert(id==0x0110000100000003ULL && value==0x20);
    assert(release(7));
    assert(!get(7,&m,&ignored)); /* 143 with a wrong Linux size is dropped, not delivered */
    assert(*(int *)dlsym(fake,"fake_frees")==4);

    bool (WIN *result)(int32_t,uint64_t,void *,int,int,bool *)=steam_bridge_export("Steam_GetAPICallResult");
    uint8_t buffer[24];bool failed=false;
    assert(result(7,42,buffer,24,143,&failed) && !failed);
    memcpy(&owner,buffer+16,8);assert(owner==0x0110000100000002ULL);
    int calls=*(int *)dlsym(fake,"fake_result_calls");
    assert(!result(7,42,buffer,24,4242,&failed) && failed); /* layout not bridged: honest failure */
    assert(!result(7,42,buffer,20,143,&failed) && failed);  /* Windows passed the Linux size */
    assert(*(int *)dlsym(fake,"fake_result_calls")==calls);
    failed=false;memset(buffer,0,sizeof buffer);
    assert(SLOT(utils,13,bool(WIN *)(void *,uint64_t,void *,int,int,bool *))(utils,42,buffer,24,143,&failed) && !failed);
    memcpy(&owner,buffer+16,8);assert(owner==0x0110000100000002ULL);

    /* SteamClient017: version-specific slots, not an alias of 020. */
    code=-1;
    void *client17=create("SteamClient017",&code);
    assert(client17 && code==0 && client17!=client && *(void **)client17!=*(void **)client);
    int32_t pipe17=SLOT(client17,0,int32_t(WIN *)(void *))(client17);
    int32_t user17=SLOT(client17,2,int32_t(WIN *)(void *,int32_t))(client17,pipe17);
    assert(pipe17==8 && user17==4);
    assert(SLOT(client17,5,void *(WIN *)(void *,int32_t,int32_t,const char *))(client17,user17,pipe17,"SteamUser021")==steam_user);
    assert(SLOT(client17,9,void *(WIN *)(void *,int32_t,const char *))(client17,pipe17,"SteamUtils010")==utils);
    assert(!SLOT(client17,8,void *(WIN *)(void *,int32_t,int32_t,const char *))(client17,user17,pipe17,"SteamFriends015"));
    SLOT(client17,7,void(WIN *)(void *,uint32_t,uint16_t))(client17,0x7f000001,27015);
    assert(*(uint32_t *)dlsym(fake,"fake017_ip")==0x7f000001 && *(uint16_t *)dlsym(fake,"fake017_port")==27015);
    SLOT(client17,19,void(WIN *)(void *))(client17); /* RunFrame; GetISteamGameSearch in 020 */
    assert(*(int *)dlsym(fake,"fake017_run_frames")==1);
    assert(SLOT(client17,20,uint32_t(WIN *)(void *))(client17)==17017);
    hook_severity=0;
    SLOT(client17,21,void(WIN *)(void *,void(WIN *)(int,const char *)))(client17,windows_hook);
    assert(hook_severity==1 && !strcmp(hook_text,"warning from 017"));
    assert(SLOT(client17,22,bool(WIN *)(void *))(client17));
    SLOT(client17,33,void(WIN *)(void *,uint32_t(WIN *)(int)))(client17,windows_check);
    assert(*(uint32_t *)dlsym(fake,"fake017_check_result")==705);
    /* One Linux object requested as both versions gets two wrappers with their own vtables. */
    void *as017=SLOT(client,12,void *(WIN *)(void *,int32_t,int32_t,const char *))(client,user,pipe,"SteamClient017");
    assert(as017 && as017!=client && *(void **)as017==*(void **)client17);
    assert(as017==SLOT(client,12,void *(WIN *)(void *,int32_t,int32_t,const char *))(client,user,pipe,"SteamClient017"));
    assert(SLOT(as017,0,int32_t(WIN *)(void *))(as017)==7);

    /* SteamClient014: 017's first 29 slots. */
    void *client14=create("SteamClient014",&code);
    assert(client14 && code==0 && *(void **)client14!=*(void **)client17 && *(void **)client14!=*(void **)client);
    int32_t pipe14=SLOT(client14,0,int32_t(WIN *)(void *))(client14);
    int32_t user14=SLOT(client14,2,int32_t(WIN *)(void *,int32_t))(client14,pipe14);
    assert(pipe14==9 && user14==5);
    assert(SLOT(client14,5,void *(WIN *)(void *,int32_t,int32_t,const char *))(client14,user14,pipe14,"SteamUser021")==steam_user);
    assert(SLOT(client14,9,void *(WIN *)(void *,int32_t,const char *))(client14,pipe14,"SteamUtils010")==utils);
    SLOT(client14,7,void(WIN *)(void *,uint32_t,uint16_t))(client14,0x0a000001,27016);
    assert(*(uint32_t *)dlsym(fake,"fake014_ip")==0x0a000001 && *(uint16_t *)dlsym(fake,"fake014_port")==27016);
    SLOT(client14,19,void(WIN *)(void *))(client14);
    assert(*(int *)dlsym(fake,"fake014_run_frames")==1);
    hook_severity=0;
    SLOT(client14,21,void(WIN *)(void *,void(WIN *)(int,const char *)))(client14,windows_hook);
    assert(hook_severity==3 && !strcmp(hook_text,"warning from 014"));
    assert(!SLOT(client14,24,void *(WIN *)(void *,int32_t,int32_t,const char *))(client14,user14,pipe14,"STEAMUNIFIEDMESSAGES_INTERFACE_VERSION001"));
    void *as014=SLOT(client,12,void *(WIN *)(void *,int32_t,int32_t,const char *))(client,user,pipe,"SteamClient014");
    assert(as014 && as014!=as017 && as014!=client && *(void **)as014==*(void **)client14);
    assert(as014==SLOT(client,12,void *(WIN *)(void *,int32_t,int32_t,const char *))(client,user,pipe,"SteamClient014"));
    assert(SLOT(as014,0,int32_t(WIN *)(void *))(as014)==7);
    assert(!SLOT(client14,28,void *(WIN *)(void *,int32_t,int32_t,const char *))(client14,user14,pipe14,"STEAMMUSIC_INTERFACE_VERSION001"));

    pid_t child=fork();
    if(!child) {SLOT(steam_user,3,void(WIN *)(void *))(steam_user);_exit(0);} /* deprecated, unbridged */
    int status;assert(waitpid(child,&status,0)==child && WIFSIGNALED(status) && WTERMSIG(status)==SIGABRT);
    child=fork();
    if(!child) {SLOT(client17,31,void(WIN *)(void *))(client17);_exit(0);} /* 017 DEPRECATED_Set_..., unbridged */
    assert(waitpid(child,&status,0)==child && WIFSIGNALED(status) && WTERMSIG(status)==SIGABRT);

    puts("PASS: interface versions, SteamClient017/014 slots/identity, MSVC hidden returns, by-value CSteamID, hooks, callback layouts, call results, slot traps");
    return 0;
}
