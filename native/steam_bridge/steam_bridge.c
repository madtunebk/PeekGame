#define _GNU_SOURCE
#include "steam_bridge.h"
#include <dlfcn.h>
#include <errno.h>
#include <pthread.h>
#include <signal.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <execinfo.h>

/* Windows code calls through ms_abi vtables; Linux steamclient.so uses SysV/Itanium.
 * Each Windows object is a Wrapper whose first field is a vtable of ms_abi thunks.
 * Linux objects are never handed to Windows code. */
#define WIN __attribute__((ms_abi))

static void log_msg(const char *fmt,...) __attribute__((format(printf,1,2)));
static void log_msg(const char *fmt,...) {
    va_list ap;va_start(ap,fmt);fputs("steam_bridge: ",stderr);vfprintf(stderr,fmt,ap);fputc('\n',stderr);va_end(ap);
}

/* Optional interoperability trace: no tickets or authentication payloads. */
static bool trace_enabled(void) {
    const char *value=getenv("STEAM_BRIDGE_TRACE");
    return value && *value && strcmp(value,"0");
}

/* ---- Linux client ---- */
#pragma pack(push,4) /* VALVE_CALLBACK_PACK_SMALL on Linux */
typedef struct {int32_t user,callback;uint8_t *param;int32_t size;} LinCallbackMsg;
#pragma pack(pop)
typedef struct {int32_t user,callback;uint8_t *param;int32_t size;} WinCallbackMsg; /* pack 8 */
_Static_assert(sizeof(LinCallbackMsg)==20,"Linux CallbackMsg_t layout");
_Static_assert(sizeof(WinCallbackMsg)==24,"Windows CallbackMsg_t layout");

static struct {
    void *library;
    void *(*create_interface)(const char *,int *);
    bool (*bget_callback)(int32_t,LinCallbackMsg *,int32_t *);
    bool (*free_last_callback)(int32_t);
    bool (*get_api_call_result)(int32_t,uint64_t,void *,int,int,bool *);
    void (*release_thread_local_memory)(bool);
} lin;
static pthread_mutex_t lock=PTHREAD_MUTEX_INITIALIZER;

static void init_vtables(void);
int steam_bridge_open(const char *path) {
    pthread_mutex_lock(&lock);
    if(lin.library) {pthread_mutex_unlock(&lock);return 0;}
    char fallback[4096];
    if(!path)path=getenv("STEAM_BRIDGE_CLIENT");
    if(!path) {
        const char *home=getenv("HOME");
        if(!home || snprintf(fallback,sizeof fallback,"%s/.steam/sdk64/steamclient.so",home)>=(int)sizeof fallback) {
            log_msg("cannot locate steamclient.so; set STEAM_BRIDGE_CLIENT");pthread_mutex_unlock(&lock);return -1;
        }
        path=fallback;
    }
    void *library=dlopen(path,RTLD_NOW|RTLD_LOCAL);
    if(!library) {log_msg("dlopen %s: %s",path,dlerror());pthread_mutex_unlock(&lock);return -1;}
    *(void **)&lin.create_interface=dlsym(library,"CreateInterface");
    *(void **)&lin.bget_callback=dlsym(library,"Steam_BGetCallback");
    *(void **)&lin.free_last_callback=dlsym(library,"Steam_FreeLastCallback");
    *(void **)&lin.get_api_call_result=dlsym(library,"Steam_GetAPICallResult");
    *(void **)&lin.release_thread_local_memory=dlsym(library,"Steam_ReleaseThreadLocalMemory");
    if(!lin.create_interface || !lin.bget_callback || !lin.free_last_callback ||
       !lin.get_api_call_result || !lin.release_thread_local_memory) {
        log_msg("%s lacks required Steam_* exports",path);dlclose(library);memset(&lin,0,sizeof lin);
        pthread_mutex_unlock(&lock);return -1;
    }
    lin.library=library;init_vtables();
    if(!getenv("SteamAppId"))log_msg("warning: SteamAppId is not set in the Linux environment");
    log_msg("loaded %s",path);
    pthread_mutex_unlock(&lock);return 0;
}
static int ensure_open(void) {return lin.library?0:steam_bridge_open(NULL);}

uint32_t steam_bridge_steam_pid(void) {
    char path[4096];const char *home=getenv("HOME");
    if(!home || snprintf(path,sizeof path,"%s/.steam/steam.pid",home)>=(int)sizeof path)return 0;
    FILE *f=fopen(path,"r");if(!f)return 0;
    unsigned long pid=0;int ok=fscanf(f,"%lu",&pid)==1;fclose(f);
    if(!ok || !pid || pid>INT32_MAX)return 0;
    if(kill((pid_t)pid,0) && errno!=EPERM)return 0;
    return (uint32_t)pid;
}

/* ---- Callback layouts (Linux pack 4 vs Windows pack 8) ----
 * Only listed callbacks cross the bridge. The Linux size is checked so a wrong assumption
 * drops the callback with a log line instead of delivering misread data. */
typedef struct {uint8_t lin_offset,win_offset,size;} Field;
typedef struct {int id;const char *name;int lin_size,win_size;Field fields[4];} CallbackLayout;
#define SAME(id,name,size) {id,name,size,size,{{0,0,0}}}
static const CallbackLayout layouts[]={
    SAME(101,"SteamServersConnected_t",1),
    SAME(102,"SteamServersDisconnected_t",4),
    SAME(103,"SteamServerConnectFailure_t",8),
    SAME(117,"IPCFailure_t",1),
    SAME(125,"LicensesUpdated_t",1),
    SAME(154,"EncryptedAppTicketResponse_t",4),
    SAME(163,"GetAuthSessionTicketResponse_t",8),
    SAME(331,"GameOverlayActivated_t",-1), /* grew across SDKs; no 8-byte members */
    SAME(701,"IPCountry_t",1),
    SAME(702,"LowBatteryPower_t",1),
    SAME(703,"SteamAPICallCompleted_t",16),
    SAME(704,"SteamShutdown_t",1),
    SAME(1005,"DlcInstalled_t",4),
    SAME(1014,"NewUrlLaunchParameters_t",1),
    SAME(1331,"RemoteStorageFileWriteAsyncComplete_t",4),
    SAME(1107,"NumberOfCurrentPlayers_t",8),
    SAME(1108,"UserStatsUnloaded_t",8),
    SAME(1109,"UserAchievementIconFetched_t",144),
    {143,"ValidateAuthTicketResponse_t",20,24,{{0,0,8},{8,8,4},{12,16,8}}},
    {152,"MicroTxnAuthorizationResponse_t",16,24,{{0,0,4},{4,8,8},{12,16,1}}},
    {304,"PersonaStateChange_t",12,16,{{0,0,8},{8,8,4}}},
    {336,"FriendRichPresenceUpdate_t",12,16,{{0,0,8},{8,8,4}}}, /* CSteamID, AppId_t */
    {1332,"RemoteStorageFileReadAsyncComplete_t",20,24,{{0,0,8},{8,8,4},{12,12,4},{16,16,4}}},
    {1101,"UserStatsReceived_t",20,24,{{0,0,8},{8,8,4},{12,16,8}}},
    {1102,"UserStatsStored_t",12,16,{{0,0,8},{8,8,4}}},
    {1103,"UserAchievementStored_t",148,152,{{0,0,8},{8,8,1},{9,9,128},{140,140,8}}},
    {1104,"LeaderboardFindResult_t",12,16,{{0,0,8},{8,8,1}}},
    {1105,"LeaderboardScoresDownloaded_t",20,24,{{0,0,8},{8,8,8},{16,16,4}}},
    {1106,"LeaderboardScoreUploaded_t",28,32,{{0,0,1},{4,8,8},{12,16,16}}}, /* score..rank_previous: one block */
    {1110,"GlobalAchievementPercentagesReady_t",12,16,{{0,0,8},{8,8,4}}},
    {1112,"GlobalStatsReceived_t",12,16,{{0,0,8},{8,8,4}}},
    SAME(503,"LobbyInvite_t",24),
    {504,"LobbyEnter_t",20,24,{{0,0,8},{8,8,4},{12,12,1},{16,16,4}}},
    {505,"LobbyDataUpdate_t",20,24,{{0,0,16},{16,16,1}}},
    {506,"LobbyChatUpdate_t",28,32,{{0,0,24},{24,24,4}}},
    SAME(507,"LobbyChatMsg_t",24),
    SAME(509,"LobbyGameCreated_t",24),
    SAME(510,"LobbyMatchList_t",4),
    {512,"LobbyKicked_t",20,24,{{0,0,16},{16,16,1}}},
    {513,"LobbyCreated_t",12,16,{{0,0,4},{4,8,8}}},
    SAME(1040011,"SteamInput_1040011_t",260),
    SAME(1040015,"SteamInputDeviceConnected_t",20),
    SAME(1040044,"SteamInputDeviceStatusChanged_t",784),
    SAME(1270009,"SteamOverlayStateChanged_t",144),
};
static const CallbackLayout *find_layout(int id) {
    for(size_t i=0;i<sizeof layouts/sizeof *layouts;i++)if(layouts[i].id==id)return &layouts[i];
    return NULL;
}
static void convert(const CallbackLayout *l,const uint8_t *from,uint8_t *to) {
    memset(to,0,l->win_size);
    for(int i=0;i<4 && l->fields[i].size;i++)memcpy(to+l->fields[i].win_offset,from+l->fields[i].lin_offset,l->fields[i].size);
}

/* Converted callback data lives until Steam_FreeLastCallback on the same pipe. */
static struct {int32_t pipe;uint8_t *data;} pending[16];
static bool remember(int32_t pipe,uint8_t *data) {
    for(size_t i=0;i<16;i++)if(!pending[i].data) {pending[i].pipe=pipe;pending[i].data=data;return true;}
    return false;
}
static void forget(int32_t pipe) {
    for(size_t i=0;i<16;i++)if(pending[i].data && pending[i].pipe==pipe) {free(pending[i].data);pending[i].data=NULL;}
}

static bool WIN w_BGetCallback(int32_t pipe,WinCallbackMsg *out,int32_t *ignored) {
    if(ensure_open())return false;
    for(;;) {
        LinCallbackMsg m;
        if(!lin.bget_callback(pipe,&m,ignored))return false;
        const CallbackLayout *l=find_layout(m.callback);
        if(!l && m.callback>=100000) {
            out->user=m.user;out->callback=m.callback;out->param=m.param;out->size=m.size;return true;
        }
        if(!l || (l->lin_size>=0 && m.size!=l->lin_size)) {
            log_msg("dropping callback %d (%d bytes): %s",m.callback,m.size,l?"unexpected Linux size":"layout not bridged");
            lin.free_last_callback(pipe);continue;
        }
        out->user=m.user;out->callback=m.callback;
        if(l->lin_size==l->win_size || l->lin_size<0) {out->param=m.param;out->size=m.size;return true;}
        uint8_t *data=malloc(l->win_size);
        pthread_mutex_lock(&lock);
        bool stored=data && remember(pipe,data);
        pthread_mutex_unlock(&lock);
        if(!stored) {log_msg("cannot hold converted %s",l->name);free(data);lin.free_last_callback(pipe);return false;}
        convert(l,m.param,data);out->param=data;out->size=l->win_size;return true;
    }
}
static bool WIN w_FreeLastCallback(int32_t pipe) {
    if(ensure_open())return false;
    pthread_mutex_lock(&lock);forget(pipe);pthread_mutex_unlock(&lock);
    return lin.free_last_callback(pipe);
}

/* Shared by Steam_GetAPICallResult and ISteamUtils::GetAPICallResult. */
typedef bool (*FetchResult)(void *ctx,uint64_t call,void *data,int size,int expected,bool *failed);
static bool call_result(FetchResult fetch,void *ctx,uint64_t call,void *data,int size,int expected,bool *failed) {
    const CallbackLayout *l=find_layout(expected);
    if(!l || l->lin_size<0) {
        log_msg("call result %d not bridged; reporting failure",expected);
        if(failed)*failed=true;
        return false;
    }
    if(size!=l->win_size) {
        log_msg("call result %s: Windows size %d, expected %d",l->name,size,l->win_size);
        if(failed)*failed=true;
        return false;
    }
    if(l->lin_size==l->win_size)return fetch(ctx,call,data,size,expected,failed);
    uint8_t buffer[256];
    if(l->lin_size>(int)sizeof buffer) {if(failed)*failed=true;return false;}
    if(!fetch(ctx,call,buffer,l->lin_size,expected,failed))return false;
    convert(l,buffer,data);return true;
}
static bool fetch_export(void *ctx,uint64_t call,void *data,int size,int expected,bool *failed) {
    return lin.get_api_call_result(*(int32_t *)ctx,call,data,size,expected,failed);
}
static bool WIN w_GetAPICallResult(int32_t pipe,uint64_t call,void *data,int size,int expected,bool *failed) {
    if(ensure_open())return false;
    return call_result(fetch_export,&pipe,call,data,size,expected,failed);
}
static void WIN w_ReleaseThreadLocalMemory(bool thread_exit) {
    if(!ensure_open())lin.release_thread_local_memory(thread_exit);
}

/* Breakpad would install a Linux crash handler over the loader's own signal handlers. */
static void WIN w_MiniDumpInit(uint32_t code,const char *version,const char *date) {
    log_msg("Breakpad_SteamMiniDumpInit(%u, %s, %s): crash reporting not bridged",code,version?version:"NULL",date?date:"NULL");
}
static void WIN w_SetAppID(uint32_t app) {log_msg("Breakpad_SteamSetAppID(%u): crash reporting not bridged",app);}
static void WIN w_SetSteamID(uint64_t id) {log_msg("Breakpad_SteamSetSteamID(%llu): crash reporting not bridged",(unsigned long long)id);}
static void WIN w_SetComment(const char *s) {log_msg("Breakpad_SteamWriteMiniDumpSetComment(%s): not bridged",s?s:"NULL");}
static void WIN w_WriteMiniDump(uint32_t code,void *exception,uint32_t build) {
    (void)exception;log_msg("Breakpad_SteamWriteMiniDumpUsingExceptionInfoWithBuildId(0x%x, build %u): no minidump written",code,build);
}

/* ---- Interface wrappers ---- */
/* One wrapper per (Linux object, version): the client may serve several versions from one object. */
typedef enum {
    CLIENT,CLIENT019,CLIENT018,CLIENT017,CLIENT014,
    USER,USER020,USER019,USER018,USER017,
    UTILS,UTILS009,UTILS008,UTILS007,
    FRIENDS,CONTROLLER,APPS,REMOTESTORAGE,USERSTATS,MATCHMAKING,
    APPTICKET,NETWORKING,MATCHMAKING_SERVERS,NETWORKING_SOCKETS,NETWORKING_UTILS,
    KINDS
} Kind;
typedef struct Wrapper {void **vtbl;void *lin;Kind kind;struct Wrapper *next;} Wrapper;
static Wrapper *wrappers;
#define MAX_SLOTS 96
static void *vtables[KINDS][MAX_SLOTS];
static const char *const kind_names[KINDS]={
    "ISteamClient020","ISteamClient019","ISteamClient018","ISteamClient017","ISteamClient014",
    "ISteamUser021","ISteamUser020","ISteamUser019","ISteamUser018","ISteamUser017",
    "ISteamUtils010","ISteamUtils009","ISteamUtils008","ISteamUtils007",
    "ISteamFriends017","ISteamController007","ISteamApps008","ISteamRemoteStorage014","ISteamUserStats011","ISteamMatchmaking009",
    "ISteamAppTicket001","ISteamNetworking006","ISteamMatchmakingServers002",
    "ISteamNetworkingSockets012","ISteamNetworkingUtils004"
};
static const char *const versions[KINDS]={
    "SteamClient020","SteamClient019","SteamClient018","SteamClient017","SteamClient014",
    "SteamUser021","SteamUser020","SteamUser019","SteamUser018","SteamUser017",
    "SteamUtils010","SteamUtils009","SteamUtils008","SteamUtils007",
    "SteamFriends017","SteamController007","STEAMAPPS_INTERFACE_VERSION008",
    "STEAMREMOTESTORAGE_INTERFACE_VERSION014","STEAMUSERSTATS_INTERFACE_VERSION011","SteamMatchMaking009",
    "STEAMAPPTICKET_INTERFACE_VERSION001","SteamNetworking006","SteamMatchMakingServers002",
    "SteamNetworkingSockets012","SteamNetworkingUtils004"
};
static const int slot_counts[KINDS]={
    42,41,40,37,29,
    32,31,29,25,24,
    39,34,28,26,
    80,34,35,55,43,38,
    1,32,32,64,64
};
static const char *const method_names[KINDS][MAX_SLOTS]={
    [CLIENT]={"CreateSteamPipe","BReleaseSteamPipe","ConnectToGlobalUser","CreateLocalUser","ReleaseUser","GetISteamUser",
     "GetISteamGameServer","SetLocalIPBinding","GetISteamFriends","GetISteamUtils","GetISteamMatchmaking",
     "GetISteamMatchmakingServers","GetISteamGenericInterface","GetISteamUserStats","GetISteamGameServerStats",
     "GetISteamApps","GetISteamNetworking","GetISteamRemoteStorage","GetISteamScreenshots","GetISteamGameSearch",
     "RunFrame","GetIPCCallCount","SetWarningMessageHook","BShutdownIfAllPipesClosed","GetISteamHTTP",
     "DEPRECATED_GetISteamUnifiedMessages","GetISteamController","GetISteamUGC","GetISteamAppList","GetISteamMusic",
     "GetISteamMusicRemote","GetISteamHTMLSurface","DEPRECATED_Set_SteamAPI_CPostAPIResultInProcess",
     "DEPRECATED_Remove_SteamAPI_CPostAPIResultInProcess","Set_SteamAPI_CCheckCallbackRegisteredInProcess",
     "GetISteamInventory","GetISteamVideo","GetISteamParentalSettings","GetISteamInput","GetISteamParties",
     "GetISteamRemotePlay","DestroyAllInterfaces"},
    {"CreateSteamPipe","BReleaseSteamPipe","ConnectToGlobalUser","CreateLocalUser","ReleaseUser","GetISteamUser",
     "GetISteamGameServer","SetLocalIPBinding","GetISteamFriends","GetISteamUtils","GetISteamMatchmaking",
     "GetISteamMatchmakingServers","GetISteamGenericInterface","GetISteamUserStats","GetISteamGameServerStats",
     "GetISteamApps","GetISteamNetworking","GetISteamRemoteStorage","GetISteamScreenshots","GetISteamGameSearch",
     "RunFrame","GetIPCCallCount","SetWarningMessageHook","BShutdownIfAllPipesClosed","GetISteamHTTP",
     "DEPRECATED_GetISteamUnifiedMessages","GetISteamController","GetISteamUGC","GetISteamAppList","GetISteamMusic",
     "GetISteamMusicRemote","GetISteamHTMLSurface","DEPRECATED_Set_SteamAPI_CPostAPIResultInProcess",
     "DEPRECATED_Remove_SteamAPI_CPostAPIResultInProcess","Set_SteamAPI_CCheckCallbackRegisteredInProcess",
     "GetISteamInventory","GetISteamVideo","GetISteamParentalSettings","GetISteamInput","GetISteamParties",
     "GetISteamRemotePlay"},
    {"CreateSteamPipe","BReleaseSteamPipe","ConnectToGlobalUser","CreateLocalUser","ReleaseUser","GetISteamUser",
     "GetISteamGameServer","SetLocalIPBinding","GetISteamFriends","GetISteamUtils","GetISteamMatchmaking",
     "GetISteamMatchmakingServers","GetISteamGenericInterface","GetISteamUserStats","GetISteamGameServerStats",
     "GetISteamApps","GetISteamNetworking","GetISteamRemoteStorage","GetISteamScreenshots","GetISteamGameSearch",
     "RunFrame","GetIPCCallCount","SetWarningMessageHook","BShutdownIfAllPipesClosed","GetISteamHTTP",
     "DEPRECATED_GetISteamUnifiedMessages","GetISteamController","GetISteamUGC","GetISteamAppList","GetISteamMusic",
     "GetISteamMusicRemote","GetISteamHTMLSurface","DEPRECATED_Set_SteamAPI_CPostAPIResultInProcess",
     "DEPRECATED_Remove_SteamAPI_CPostAPIResultInProcess","Set_SteamAPI_CCheckCallbackRegisteredInProcess",
     "GetISteamInventory","GetISteamVideo","GetISteamParentalSettings","GetISteamInput","GetISteamParties"},
    {"CreateSteamPipe","BReleaseSteamPipe","ConnectToGlobalUser","CreateLocalUser","ReleaseUser","GetISteamUser",
     "GetISteamGameServer","SetLocalIPBinding","GetISteamFriends","GetISteamUtils","GetISteamMatchmaking",
     "GetISteamMatchmakingServers","GetISteamGenericInterface","GetISteamUserStats","GetISteamGameServerStats",
     "GetISteamApps","GetISteamNetworking","GetISteamRemoteStorage","GetISteamScreenshots","RunFrame",
     "GetIPCCallCount","SetWarningMessageHook","BShutdownIfAllPipesClosed","GetISteamHTTP",
     "DEPRECATED_GetISteamUnifiedMessages","GetISteamController","GetISteamUGC","GetISteamAppList","GetISteamMusic",
     "GetISteamMusicRemote","GetISteamHTMLSurface","DEPRECATED_Set_SteamAPI_CPostAPIResultInProcess",
     "DEPRECATED_Remove_SteamAPI_CPostAPIResultInProcess","Set_SteamAPI_CCheckCallbackRegisteredInProcess",
     "GetISteamInventory","GetISteamVideo","GetISteamParentalSettings"},
    {"CreateSteamPipe","BReleaseSteamPipe","ConnectToGlobalUser","CreateLocalUser","ReleaseUser","GetISteamUser",
     "GetISteamGameServer","SetLocalIPBinding","GetISteamFriends","GetISteamUtils","GetISteamMatchmaking",
     "GetISteamMatchmakingServers","GetISteamGenericInterface","GetISteamUserStats","GetISteamGameServerStats",
     "GetISteamApps","GetISteamNetworking","GetISteamRemoteStorage","GetISteamScreenshots","RunFrame",
     "GetIPCCallCount","SetWarningMessageHook","BShutdownIfAllPipesClosed","GetISteamHTTP",
     "GetISteamUnifiedMessages","GetISteamController","GetISteamUGC","GetISteamAppList","GetISteamMusic"},
    {"GetHSteamUser","BLoggedOn","GetSteamID","InitiateGameConnection_DEPRECATED","TerminateGameConnection_DEPRECATED",
     "TrackAppUsageEvent","GetUserDataFolder","StartVoiceRecording","StopVoiceRecording","GetAvailableVoice","GetVoice",
     "DecompressVoice","GetVoiceOptimalSampleRate","GetAuthSessionTicket","BeginAuthSession","EndAuthSession",
     "CancelAuthTicket","UserHasLicenseForApp","BIsBehindNAT","AdvertiseGame","RequestEncryptedAppTicket",
     "GetEncryptedAppTicket","GetGameBadgeLevel","GetPlayerSteamLevel","RequestStoreAuthURL","BIsPhoneVerified",
     "BIsTwoFactorEnabled","BIsPhoneIdentifying","BIsPhoneRequiringVerification","GetMarketEligibility",
     "GetDurationControl","BSetDurationControlOnlineState"},
    {"GetHSteamUser","BLoggedOn","GetSteamID","InitiateGameConnection_DEPRECATED","TerminateGameConnection_DEPRECATED",
     "TrackAppUsageEvent","GetUserDataFolder","StartVoiceRecording","StopVoiceRecording","GetAvailableVoice","GetVoice",
     "DecompressVoice","GetVoiceOptimalSampleRate","GetAuthSessionTicket","BeginAuthSession","EndAuthSession",
     "CancelAuthTicket","UserHasLicenseForApp","BIsBehindNAT","AdvertiseGame","RequestEncryptedAppTicket",
     "GetEncryptedAppTicket","GetGameBadgeLevel","GetPlayerSteamLevel","RequestStoreAuthURL","BIsPhoneVerified",
     "BIsTwoFactorEnabled","BIsPhoneIdentifying","BIsPhoneRequiringVerification","GetMarketEligibility",
     "GetDurationControl"},
    {"GetSecondsSinceAppActive","GetSecondsSinceComputerActive","GetConnectedUniverse","GetServerRealTime",
     "GetIPCountry","GetImageSize","GetImageRGBA","GetCSERIPPort","GetCurrentBatteryPower","GetAppID",
     "SetOverlayNotificationPosition","IsAPICallCompleted","GetAPICallFailureReason","GetAPICallResult","RunFrame",
     "GetIPCCallCount","SetWarningMessageHook","IsOverlayEnabled","BOverlayNeedsPresent","CheckFileSignature",
     "ShowGamepadTextInput","GetEnteredGamepadTextLength","GetEnteredGamepadTextInput","GetSteamUILanguage",
     "IsSteamRunningInVR","SetOverlayNotificationInset","IsSteamInBigPictureMode","StartVRDashboard",
     "IsVRHeadsetStreamingEnabled","SetVRHeadsetStreamingEnabled","IsSteamChinaLauncher","InitFilterText",
     "FilterText","GetIPv6ConnectivityState","IsSteamRunningOnSteamDeck","ShowFloatingGamepadTextInput",
     "SetGameLauncherMode","DismissFloatingGamepadTextInput","DismissGamepadTextInput"},
    {"GetSecondsSinceAppActive","GetSecondsSinceComputerActive","GetConnectedUniverse","GetServerRealTime",
     "GetIPCountry","GetImageSize","GetImageRGBA","GetCSERIPPort","GetCurrentBatteryPower","GetAppID",
     "SetOverlayNotificationPosition","IsAPICallCompleted","GetAPICallFailureReason","GetAPICallResult","RunFrame",
     "GetIPCCallCount","SetWarningMessageHook","IsOverlayEnabled","BOverlayNeedsPresent","CheckFileSignature",
     "ShowGamepadTextInput","GetEnteredGamepadTextLength","GetEnteredGamepadTextInput","GetSteamUILanguage",
     "IsSteamRunningInVR","SetOverlayNotificationInset","IsSteamInBigPictureMode","StartVRDashboard",
     "IsVRHeadsetStreamingEnabled","SetVRHeadsetStreamingEnabled","IsSteamChinaLauncher","InitFilterText",
     "FilterText","GetIPv6ConnectivityState"},
    {"GetPersonaName","SetPersonaName","GetPersonaState","GetFriendCount","GetFriendByIndex","GetFriendRelationship",
     "GetFriendPersonaState","GetFriendPersonaName","GetFriendGamePlayed","GetFriendPersonaNameHistory","GetFriendSteamLevel",
     "GetPlayerNickname","GetFriendsGroupCount","GetFriendsGroupIDByIndex","GetFriendsGroupName","GetFriendsGroupMembersCount",
     "GetFriendsGroupMembersList","HasFriend","GetClanCount","GetClanByIndex","GetClanName","GetClanTag","GetClanActivityCounts",
     "DownloadClanActivityCounts","GetFriendCountFromSource","GetFriendFromSourceByIndex","IsUserInSource","SetInGameVoiceSpeaking",
     "ActivateGameOverlay","ActivateGameOverlayToUser","ActivateGameOverlayToWebPage","ActivateGameOverlayToStore","SetPlayedWith",
     "ActivateGameOverlayInviteDialog","GetSmallFriendAvatar","GetMediumFriendAvatar","GetLargeFriendAvatar","RequestUserInformation",
     "RequestClanOfficerList","GetClanOwner","GetClanOfficerCount","GetClanOfficerByIndex","GetUserRestrictions","SetRichPresence",
     "ClearRichPresence","GetFriendRichPresence","GetFriendRichPresenceKeyCount","GetFriendRichPresenceKeyByIndex",
     "RequestFriendRichPresence","InviteUserToGame","GetCoplayFriendCount","GetCoplayFriend","GetFriendCoplayTime","GetFriendCoplayGame",
     "JoinClanChatRoom","LeaveClanChatRoom","GetClanChatMemberCount","GetChatMemberByIndex","SendClanChatMessage","GetClanChatMessage",
     "IsClanChatAdmin","IsClanChatWindowOpenInSteam","OpenClanChatWindowInSteam","CloseClanChatWindowInSteam","SetListenForFriendsMessages",
     "ReplyToFriendMessage","GetFriendMessage","GetFollowerCount","IsFollowing","EnumerateFollowingList","IsClanPublic",
     "IsClanOfficialGameGroup","GetNumChatsWithUnreadPriorityMessages","ActivateGameOverlayRemotePlayTogetherInviteDialog",
     "RegisterProtocolInOverlayBrowser","ActivateGameOverlayInviteDialogConnectString","RequestEquippedProfileItems",
     "BHasEquippedProfileItem","GetProfileItemPropertyString","GetProfileItemPropertyUint"},
    {"Init","Shutdown","RunFrame","GetConnectedControllers","GetActionSetHandle","ActivateActionSet",
     "GetCurrentActionSet","ActivateActionSetLayer","DeactivateActionSetLayer","DeactivateAllActionSetLayers",
     "GetActiveActionSetLayers","GetDigitalActionHandle","GetDigitalActionData","GetDigitalActionOrigins",
     "GetAnalogActionHandle","GetAnalogActionData","GetAnalogActionOrigins","GetGlyphForActionOrigin",
     "GetStringForActionOrigin","StopAnalogActionMomentum","GetMotionData","TriggerHapticPulse",
     "TriggerRepeatedHapticPulse","TriggerVibration","SetLEDColor","ShowBindingPanel","GetInputTypeForHandle",
     "GetControllerForGamepadIndex","GetGamepadIndexForController","GetStringForXboxOrigin","GetGlyphForXboxOrigin",
      "GetActionOriginFromXboxOrigin","TranslateActionOrigin","GetControllerBindingRevision"},
    {"BIsSubscribed","BIsLowViolence","BIsCybercafe","BIsVACBanned","GetCurrentGameLanguage",
     "GetAvailableGameLanguages","BIsSubscribedApp","BIsDlcInstalled","GetEarliestPurchaseUnixTime",
     "BIsSubscribedFromFreeWeekend","GetDLCCount","BGetDLCDataByIndex","InstallDLC","UninstallDLC",
     "RequestAppProofOfPurchaseKey","GetCurrentBetaName","MarkContentCorrupt","GetInstalledDepots",
     "GetAppInstallDir","BIsAppInstalled","GetAppOwner","GetLaunchQueryParam","GetDlcDownloadProgress",
     "GetAppBuildId","RequestAllProofOfPurchaseKeys","GetFileDetails","GetLaunchCommandLine",
     "BIsSubscribedFromFamilySharing","BIsTimedTrial","SetDlcContext","GetNumBetas","GetBetaInfo",
     "SetActiveBeta","SetGamePerformanceSetting","SetGameRenderResolution"},
    {"FileWrite","FileRead","FileWriteAsync","FileReadAsync","FileReadAsyncComplete","FileForget","FileDelete",
     "FileShare","SetSyncPlatforms","FileWriteStreamOpen","FileWriteStreamWriteChunk","FileWriteStreamClose",
     "FileWriteStreamCancel","FileExists","FilePersisted","GetFileSize","GetFileTimestamp","GetSyncPlatforms",
     "GetFileCount","GetFileNameAndSize","GetQuota","IsCloudEnabledForAccount","IsCloudEnabledForApp",
     "SetCloudEnabledForApp","UGCDownload","GetUGCDownloadProgress","GetUGCDetails","UGCRead","GetCachedUGCCount",
     "GetCachedUGCHandle","PublishWorkshopFile","CreatePublishedFileUpdateRequest","UpdatePublishedFileFile",
     "UpdatePublishedFilePreviewFile","UpdatePublishedFileTitle","UpdatePublishedFileDescription",
     "UpdatePublishedFileVisibility","UpdatePublishedFileTags","CommitPublishedFileUpdate","GetPublishedFileDetails",
     "DeletePublishedFile","EnumerateUserPublishedFiles","SubscribePublishedFile","EnumerateUserSubscribedFiles",
     "UnsubscribePublishedFile","UpdatePublishedFileSetChangeDescription","GetPublishedItemVoteDetails",
     "UpdateUserPublishedItemVote","GetUserPublishedItemVoteDetails","EnumerateUserSharedWorkshopFiles","PublishVideo",
     "SetUserPublishedFileAction","EnumeratePublishedFilesByUserAction","EnumeratePublishedWorkshopFiles",
     "UGCDownloadToLocation"},
    /* Windows (MSVC) order: each overload group is reversed relative to the header. */
    {"RequestCurrentStats","GetStat(float)","GetStat(int32)","SetStat(float)","SetStat(int32)","UpdateAvgRateStat",
     "GetAchievement","SetAchievement","ClearAchievement","GetAchievementAndUnlockTime","StoreStats",
     "GetAchievementIcon","GetAchievementDisplayAttribute","IndicateAchievementProgress","GetNumAchievements",
     "GetAchievementName","RequestUserStats","GetUserStat(float)","GetUserStat(int32)","GetUserAchievement",
     "GetUserAchievementAndUnlockTime","ResetAllStats","FindOrCreateLeaderboard","FindLeaderboard",
     "GetLeaderboardName","GetLeaderboardEntryCount","GetLeaderboardSortMethod","GetLeaderboardDisplayType",
     "DownloadLeaderboardEntries","DownloadLeaderboardEntriesForUsers","GetDownloadedLeaderboardEntry",
     "UploadLeaderboardScore","AttachLeaderboardUGC","GetNumberOfCurrentPlayers","RequestGlobalAchievementPercentages",
     "GetMostAchievedAchievementInfo","GetNextMostAchievedAchievementInfo","GetAchievementAchievedPercent",
     "RequestGlobalStats","GetGlobalStat(double)","GetGlobalStat(int64)","GetGlobalStatHistory(double)",
     "GetGlobalStatHistory(int64)"},
    {"GetFavoriteGameCount","GetFavoriteGame","AddFavoriteGame","RemoveFavoriteGame",
     "RequestLobbyList","AddRequestLobbyListStringFilter","AddRequestLobbyListNumericalFilter",
     "AddRequestLobbyListNearValueFilter","AddRequestLobbyListFilterSlotsAvailable",
     "AddRequestLobbyListDistanceFilter","AddRequestLobbyListResultCountFilter",
     "AddRequestLobbyListCompatibleMembersFilter","GetLobbyByIndex","CreateLobby","JoinLobby",
     "LeaveLobby","InviteUserToLobby","GetNumLobbyMembers","GetLobbyMemberByIndex",
     "GetLobbyData","SetLobbyData","GetLobbyDataCount","GetLobbyDataByIndex",
     "DeleteLobbyData","GetLobbyMemberData","SetLobbyMemberData","SendLobbyChatMsg",
     "GetLobbyChatEntry","RequestLobbyData","SetLobbyGameServer","GetLobbyGameServer",
     "SetLobbyMemberLimit","GetLobbyMemberLimit","SetLobbyType","SetLobbyJoinable",
      "GetLobbyOwner","SetLobbyOwner","SetLinkedLobby"},
    [APPTICKET]={"GetAppOwnershipTicketData"},
    [NETWORKING]={
        "SendP2PPacket","IsP2PPacketAvailable","ReadP2PPacket","AcceptP2PSessionWithUser",
        "CloseP2PSessionWithUser","CloseP2PChannelWithUser","GetP2PSessionState","AllowP2PPacketRelay",
        "CreateListenSocket","CreateP2PConnectionSocket","CreateConnectionSocket","DestroySocket",
        "DestroyListenSocket","SendDataOnSocket","IsDataAvailableOnSocket","RetrieveDataFromSocket",
        "IsDataAvailable","RetrieveData","GetSocketInfo","GetListenSocketInfo",
        "GetSocketConnectionType","GetMaxPacketSize"
    },
    [NETWORKING_SOCKETS]={[22]="InitAuthentication",[23]="GetAuthenticationStatus"},
    [NETWORKING_UTILS]={[13]="SetDebugOutputFunction"},
};

/* Unbridged slots stop with the method name, matching the loader's UD2 traps. */
static void unbridged(Kind kind,int slot) {
    const char *name = (slot < slot_counts[kind] && method_names[kind][slot]) ? method_names[kind][slot] : NULL;
    if(!name) {
        if(kind==USER020 || kind==USER019 || kind==USER018 || kind==USER017) name=method_names[USER][slot];
        else if(kind==UTILS009 || kind==UTILS008 || kind==UTILS007) name=method_names[UTILS][slot];
        else if(kind==CLIENT019 || kind==CLIENT018 || kind==CLIENT017 || kind==CLIENT014) name=method_names[CLIENT][slot];
    }
    fprintf(stderr,"STOP: steam_bridge: %s::%s (slot %d) not bridged\n",kind_names[kind],name?name:"unknown",slot);
    abort();
}
#define TRAP(kind,n) static void WIN trap_##kind##_##n(void) {unbridged(kind,n);}
#define SLOTS0_47(X,k) X(k,0) X(k,1) X(k,2) X(k,3) X(k,4) X(k,5) X(k,6) X(k,7) X(k,8) X(k,9) X(k,10) X(k,11) \
    X(k,12) X(k,13) X(k,14) X(k,15) X(k,16) X(k,17) X(k,18) X(k,19) X(k,20) X(k,21) X(k,22) X(k,23) X(k,24) \
    X(k,25) X(k,26) X(k,27) X(k,28) X(k,29) X(k,30) X(k,31) X(k,32) X(k,33) X(k,34) X(k,35) X(k,36) X(k,37) \
    X(k,38) X(k,39) X(k,40) X(k,41) X(k,42) X(k,43) X(k,44) X(k,45) X(k,46) X(k,47)
#define SLOTS48_95(X,k) X(k,48) X(k,49) X(k,50) X(k,51) X(k,52) X(k,53) X(k,54) X(k,55) X(k,56) X(k,57) X(k,58) X(k,59) \
    X(k,60) X(k,61) X(k,62) X(k,63) X(k,64) X(k,65) X(k,66) X(k,67) X(k,68) X(k,69) X(k,70) X(k,71) X(k,72) \
    X(k,73) X(k,74) X(k,75) X(k,76) X(k,77) X(k,78) X(k,79) X(k,80) X(k,81) X(k,82) X(k,83) X(k,84) X(k,85) \
    X(k,86) X(k,87) X(k,88) X(k,89) X(k,90) X(k,91) X(k,92) X(k,93) X(k,94) X(k,95)
#define SLOTS(X,k) SLOTS0_47(X,k) SLOTS48_95(X,k)
SLOTS(TRAP,CLIENT) SLOTS(TRAP,CLIENT019) SLOTS(TRAP,CLIENT018) SLOTS(TRAP,CLIENT017) SLOTS(TRAP,CLIENT014) \
SLOTS(TRAP,USER) SLOTS(TRAP,USER020) SLOTS(TRAP,USER019) SLOTS(TRAP,USER018) SLOTS(TRAP,USER017) \
SLOTS(TRAP,UTILS) SLOTS(TRAP,UTILS009) SLOTS(TRAP,UTILS008) SLOTS(TRAP,UTILS007) \
SLOTS(TRAP,FRIENDS) SLOTS(TRAP,CONTROLLER) SLOTS(TRAP,APPS) SLOTS(TRAP,REMOTESTORAGE) SLOTS(TRAP,USERSTATS) SLOTS(TRAP,MATCHMAKING) SLOTS(TRAP,APPTICKET) SLOTS(TRAP,NETWORKING) SLOTS(TRAP,MATCHMAKING_SERVERS) SLOTS(TRAP,NETWORKING_SOCKETS) SLOTS(TRAP,NETWORKING_UTILS)
#define TRAP_ADDRESS(kind,n) (void *)trap_##kind##_##n,
static void *const traps[KINDS][MAX_SLOTS]={
    [CLIENT]={SLOTS(TRAP_ADDRESS,CLIENT)},
    [CLIENT019]={SLOTS(TRAP_ADDRESS,CLIENT019)},
    [CLIENT018]={SLOTS(TRAP_ADDRESS,CLIENT018)},
    [CLIENT017]={SLOTS(TRAP_ADDRESS,CLIENT017)},
    [CLIENT014]={SLOTS(TRAP_ADDRESS,CLIENT014)},
    [USER]={SLOTS(TRAP_ADDRESS,USER)},
    [USER020]={SLOTS(TRAP_ADDRESS,USER020)},
    [USER019]={SLOTS(TRAP_ADDRESS,USER019)},
    [USER018]={SLOTS(TRAP_ADDRESS,USER018)},
    [USER017]={SLOTS(TRAP_ADDRESS,USER017)},
    [UTILS]={SLOTS(TRAP_ADDRESS,UTILS)},
    [UTILS009]={SLOTS(TRAP_ADDRESS,UTILS009)},
    [UTILS008]={SLOTS(TRAP_ADDRESS,UTILS008)},
    [UTILS007]={SLOTS(TRAP_ADDRESS,UTILS007)},
    [FRIENDS]={SLOTS(TRAP_ADDRESS,FRIENDS)},
    [CONTROLLER]={SLOTS(TRAP_ADDRESS,CONTROLLER)},
    [APPS]={SLOTS(TRAP_ADDRESS,APPS)},
    [REMOTESTORAGE]={SLOTS(TRAP_ADDRESS,REMOTESTORAGE)},
    [USERSTATS]={SLOTS(TRAP_ADDRESS,USERSTATS)},
    [MATCHMAKING]={SLOTS(TRAP_ADDRESS,MATCHMAKING)},
    [APPTICKET]={SLOTS(TRAP_ADDRESS,APPTICKET)},
    [NETWORKING]={SLOTS(TRAP_ADDRESS,NETWORKING)},
    [MATCHMAKING_SERVERS]={SLOTS(TRAP_ADDRESS,MATCHMAKING_SERVERS)},
    [NETWORKING_SOCKETS]={SLOTS(TRAP_ADDRESS,NETWORKING_SOCKETS)},
    [NETWORKING_UTILS]={SLOTS(TRAP_ADDRESS,NETWORKING_UTILS)}
};

static void *wrap(void *object,Kind kind) {
    if(!object)return NULL;
    pthread_mutex_lock(&lock);
    Wrapper *w=wrappers;
    while(w && (w->lin!=object || w->kind!=kind))w=w->next;
    if(!w && (w=malloc(sizeof *w))) {*w=(Wrapper){vtables[kind],object,kind,wrappers};wrappers=w;}
    pthread_mutex_unlock(&lock);
    if(!w)log_msg("wrapper allocation failed for %s",kind_names[kind]);
    return w;
}
static int bridged_kind(const char *version) {
    if(!version) return -1;
    for(int k=0;k<KINDS;k++)if(!strcmp(version,versions[k]))return k;
    if(!strncmp(version,"STEAMREMOTESTORAGE_INTERFACE_VERSION",36)) return REMOTESTORAGE;
    if(!strncmp(version,"STEAMUSERSTATS_INTERFACE_VERSION",32)) return USERSTATS;
    if(!strncmp(version,"SteamInput",10) || !strncmp(version,"STEAMINPUT_INTERFACE_VERSION",28)) return CONTROLLER;
    if(!strncmp(version,"SteamController",15)) return CONTROLLER;
    if(!strncmp(version,"SteamNetworking",15)) return NETWORKING;
    return -1;
}

#define L(self) (((Wrapper *)(self))->lin)
#define M(self,slot,type) ((type)(*(void ***)L(self))[slot])

/* ISteamClient thunks are shared by both versions and named by their SteamClient020 slot.
 * SteamClient017 lacks GetISteamGameSearch (020 slot 19), so 020 slots 20-37 are one lower,
 * and it has no slots for 020's 38-41. SteamClient014 is 017's first 29 slots (020 slots 0-18, 20-29).
 * Thunks for methods a version lacks are never installed in its vtable. */
static int client_slot(void *self,int slot020) {
    if(((Wrapper *)self)->kind==CLIENT || ((Wrapper *)self)->kind==CLIENT019 || ((Wrapper *)self)->kind==CLIENT018)return slot020;
    if(slot020<19)return slot020;
    if(slot020==19 || slot020>37 || (((Wrapper *)self)->kind==CLIENT014 && slot020>29))abort();
    return slot020-1;
}
#define CM(self,slot020,type) ((type)(*(void ***)L(self))[client_slot(self,slot020)])

/* ISteamClient020 and ISteamClient017 */
static int32_t WIN client_CreateSteamPipe(void *self) {
    int32_t result=CM(self,0,int32_t(*)(void *))(L(self));
    if(trace_enabled())log_msg("trace %s::CreateSteamPipe -> %d",kind_names[((Wrapper *)self)->kind],result);
    return result;
}
static bool WIN client_BReleaseSteamPipe(void *self,int32_t pipe) {return CM(self,1,bool(*)(void *,int32_t))(L(self),pipe);}
static int32_t WIN client_ConnectToGlobalUser(void *self,int32_t pipe) {
    int32_t result=CM(self,2,int32_t(*)(void *,int32_t))(L(self),pipe);
    if(trace_enabled())log_msg("trace %s::ConnectToGlobalUser(pipe=%d) -> %d",kind_names[((Wrapper *)self)->kind],pipe,result);
    return result;
}
static int32_t WIN client_CreateLocalUser(void *self,int32_t *pipe,int type) {return CM(self,3,int32_t(*)(void *,int32_t *,int))(L(self),pipe,type);}
static void WIN client_ReleaseUser(void *self,int32_t pipe,int32_t user) {CM(self,4,void(*)(void *,int32_t,int32_t))(L(self),pipe,user);}
static void *WIN client_GetISteamUser(void *self,int32_t user,int32_t pipe,const char *version) {
    int kind=bridged_kind(version);
    if(kind!=USER && kind!=USER020 && kind!=USER019 && kind!=USER018 && kind!=USER017) {log_msg("GetISteamUser(%s): version not bridged",version?version:"NULL");return NULL;}
    void *linux_object=CM(self,5,void *(*)(void *,int32_t,int32_t,const char *))(L(self),user,pipe,version);
    void *windows_object=wrap(linux_object,(Kind)kind);
    if(trace_enabled())log_msg("trace GetISteamUser(%s, user=%d, pipe=%d) -> Linux=%p Windows=%p",version,user,pipe,linux_object,windows_object);
    return windows_object;
}
static void WIN client_SetLocalIPBinding(void *self,const void *address,uint16_t port) {
    CM(self,7,void(*)(void *,const void *,uint16_t))(L(self),address,port); /* SteamIPAddress_t: 20 bytes on both */
}
/* SteamClient017 takes a host-order IPv4 address; Proton's unix thunk loads it as 32 bits. */
static void WIN client017_SetLocalIPBinding(void *self,uint32_t ip,uint16_t port) {
    M(self,7,void(*)(void *,uint32_t,uint16_t))(L(self),ip,port);
}
static void *WIN client_GetISteamFriends(void *self,int32_t user,int32_t pipe,const char *version) {
    int kind=bridged_kind(version);
    if(kind!=FRIENDS) {log_msg("GetISteamFriends(%s): version not bridged",version?version:"NULL");return NULL;}
    return wrap(CM(self,8,void *(*)(void *,int32_t,int32_t,const char *))(L(self),user,pipe,version),(Kind)kind);
}
static void *WIN client_GetISteamMatchmaking(void *self,int32_t user,int32_t pipe,const char *version) {
    int kind=bridged_kind(version);
    if(kind!=MATCHMAKING) {log_msg("GetISteamMatchmaking(%s): version not bridged",version?version:"NULL");return NULL;}
    return wrap(CM(self,10,void *(*)(void *,int32_t,int32_t,const char *))(L(self),user,pipe,version),(Kind)kind);
}
static void *WIN client_GetISteamController(void *self,int32_t user,int32_t pipe,const char *version) {
    int kind=bridged_kind(version);
    if(kind!=CONTROLLER) {log_msg("GetISteamController(%s): version not bridged",version?version:"NULL");return NULL;}
    return wrap(CM(self,26,void *(*)(void *,int32_t,int32_t,const char *))(L(self),user,pipe,version),(Kind)kind);
}
static void *WIN client_GetISteamInput(void *self,int32_t user,int32_t pipe,const char *version) {
    int kind=bridged_kind(version);
    if(kind!=CONTROLLER) {log_msg("GetISteamInput(%s): version not bridged",version?version:"NULL");return NULL;}
    return wrap(CM(self,38,void *(*)(void *,int32_t,int32_t,const char *))(L(self),user,pipe,version),(Kind)kind);
}
static void *WIN client_GetISteamNetworking(void *self,int32_t user,int32_t pipe,const char *version) {
    int kind=bridged_kind(version);
    if(kind!=NETWORKING) {log_msg("GetISteamNetworking(%s): version not bridged",version?version:"NULL");return NULL;}
    return wrap(CM(self,16,void *(*)(void *,int32_t,int32_t,const char *))(L(self),user,pipe,version),(Kind)kind);
}
static void *WIN client_GetISteamUtils(void *self,int32_t pipe,const char *version) {
    int kind=bridged_kind(version);
    if(kind!=UTILS && kind!=UTILS009 && kind!=UTILS008 && kind!=UTILS007) {log_msg("GetISteamUtils(%s): version not bridged",version?version:"NULL");return NULL;}
    return wrap(CM(self,9,void *(*)(void *,int32_t,const char *))(L(self),pipe,version),(Kind)kind);
}
static void *WIN client_GetISteamApps(void *self,int32_t user,int32_t pipe,const char *version) {
    int kind=bridged_kind(version);
    if(kind!=APPS) {log_msg("GetISteamApps(%s): version not bridged",version?version:"NULL");return NULL;}
    return wrap(CM(self,15,void *(*)(void *,int32_t,int32_t,const char *))(L(self),user,pipe,version),(Kind)kind);
}
static void log_unbridged_once(const char *version);
static void *WIN client_GetISteamGenericInterface(void *self,int32_t user,int32_t pipe,const char *version) {
    int kind=bridged_kind(version);
    if(kind<0) {log_unbridged_once(version);return NULL;}
    void *linux_object=CM(self,12,void *(*)(void *,int32_t,int32_t,const char *))(L(self),user,pipe,version);
    void *windows_object=wrap(linux_object,(Kind)kind);
    if(trace_enabled())log_msg("trace GetISteamGenericInterface(%s, user=%d, pipe=%d) -> Linux=%p Windows=%p",version,user,pipe,linux_object,windows_object);
    return windows_object;
}
static void *WIN client_GetISteamUserStats(void *self,int32_t user,int32_t pipe,const char *version) {
    int kind=bridged_kind(version);
    if(kind!=USERSTATS) {log_msg("GetISteamUserStats(%s): version not bridged",version?version:"NULL");return NULL;}
    return wrap(CM(self,13,void *(*)(void *,int32_t,int32_t,const char *))(L(self),user,pipe,version),(Kind)kind);
}
static void *WIN client_GetISteamRemoteStorage(void *self,int32_t user,int32_t pipe,const char *version) {
    int kind=bridged_kind(version);
    if(kind!=REMOTESTORAGE) {log_msg("GetISteamRemoteStorage(%s): version not bridged",version?version:"NULL");return NULL;}
    return wrap(CM(self,17,void *(*)(void *,int32_t,int32_t,const char *))(L(self),user,pipe,version),(Kind)kind);
}
/* Every other GetISteam* getter: (user, pipe, version). Returning NULL is Steam's own "not available". */
/* Games ask again every frame (SteamInput); each version is reported once. */
static void log_unbridged_once(const char *version) {
    static pthread_mutex_t lock=PTHREAD_MUTEX_INITIALIZER;
    static char seen[32][128];
    static int count;
    const char *name=version?version:"NULL";
    pthread_mutex_lock(&lock);
    bool known=false;
    for(int i=0;i<count && !known;i++) known=strncmp(seen[i],name,sizeof seen[i]-1)==0;
    if(!known && count<32) snprintf(seen[count++],sizeof seen[0],"%s",name);
    pthread_mutex_unlock(&lock);
    if(!known) log_msg("interface %s not bridged",name);
}
static void *WIN client_unbridged_getter(void *self,int32_t user,int32_t pipe,const char *version) {
    (void)self;(void)user;(void)pipe;log_unbridged_once(version);return NULL;
}
static void WIN client_RunFrame(void *self) {CM(self,20,void(*)(void *))(L(self));}
static uint32_t WIN client_GetIPCCallCount(void *self) {return CM(self,21,uint32_t(*)(void *))(L(self));}

/* Linux calls these hooks with SysV; they forward to the Windows function with ms_abi. */
typedef void (WIN *WinWarningHook)(int,const char *);
static WinWarningHook client_hooks[5],utils_hook;
static void lin_client_hook(int severity,const char *text) {client_hooks[CLIENT](severity,text);}
static void lin_client019_hook(int severity,const char *text) {client_hooks[CLIENT019](severity,text);}
static void lin_client018_hook(int severity,const char *text) {client_hooks[CLIENT018](severity,text);}
static void lin_client017_hook(int severity,const char *text) {client_hooks[CLIENT017](severity,text);}
static void lin_client014_hook(int severity,const char *text) {client_hooks[CLIENT014](severity,text);}
static void (*const lin_client_hooks[5])(int,const char *)={lin_client_hook,lin_client019_hook,lin_client018_hook,lin_client017_hook,lin_client014_hook};
static void lin_utils_hook(int severity,const char *text) {utils_hook(severity,text);}
static void WIN client_SetWarningMessageHook(void *self,WinWarningHook hook) {
    Kind kind=((Wrapper *)self)->kind;client_hooks[kind]=hook;
    CM(self,22,void(*)(void *,void(*)(int,const char *)))(L(self),hook?lin_client_hooks[kind]:NULL);
}
static bool WIN client_BShutdownIfAllPipesClosed(void *self) {return CM(self,23,bool(*)(void *))(L(self));}
typedef uint32_t (WIN *WinCheckCallback)(int);
static WinCheckCallback check_callbacks[KINDS]; /* SteamClient014 has no such slot */
static uint32_t lin_check_callback(int callback) {return check_callbacks[CLIENT](callback);}
static uint32_t lin_check_callback019(int callback) {return check_callbacks[CLIENT019](callback);}
static uint32_t lin_check_callback018(int callback) {return check_callbacks[CLIENT018](callback);}
static uint32_t lin_check_callback017(int callback) {return check_callbacks[CLIENT017](callback);}
static void WIN client_SetCheckCallbackRegistered(void *self,WinCheckCallback function) {
    Kind kind=((Wrapper *)self)->kind;check_callbacks[kind]=function;
    void *cb=NULL;
    if(function) {
        if(kind==CLIENT)cb=(void *)lin_check_callback;
        else if(kind==CLIENT019)cb=(void *)lin_check_callback019;
        else if(kind==CLIENT018)cb=(void *)lin_check_callback018;
        else cb=(void *)lin_check_callback017;
    }
    CM(self,34,void(*)(void *,uint32_t(*)(int)))(L(self),cb);
}
static void WIN client_DestroyAllInterfaces(void *self) {CM(self,41,void(*)(void *))(L(self));}

/* ISteamUser021. CSteamID/CGameID are trivially copyable 8-byte classes: passed in a register
 * by both ABIs, but MSVC returns them from member functions through a hidden pointer (RDX). */
static int32_t WIN user_GetHSteamUser(void *self) {return M(self,0,int32_t(*)(void *))(L(self));}
static bool WIN user_BLoggedOn(void *self) {return M(self,1,bool(*)(void *))(L(self));}
static uint64_t *WIN user_GetSteamID(void *self,uint64_t *result) {*result=M(self,2,uint64_t(*)(void *))(L(self));return result;}
static void WIN user_TrackAppUsageEvent(void *self,uint64_t game,int event,const char *extra) {
    M(self,5,void(*)(void *,uint64_t,int,const char *))(L(self),game,event,extra);
}
static bool WIN user_GetUserDataFolder(void *self,char *buffer,int size) {return M(self,6,bool(*)(void *,char *,int))(L(self),buffer,size);}
static uint32_t WIN user_GetAuthSessionTicket(void *self,void *ticket,int size,uint32_t *written,const void *identity) {
    uint32_t result;
    if(((Wrapper *)self)->kind==USER)
        result=M(self,13,uint32_t(*)(void *,void *,int,uint32_t *,const void *))(L(self),ticket,size,written,identity);
    else
        result=M(self,13,uint32_t(*)(void *,void *,int,uint32_t *))(L(self),ticket,size,written);
    if(trace_enabled())log_msg("trace ISteamUser::GetAuthSessionTicket(version=%s, max=%d) -> handle=%u size=%u",kind_names[((Wrapper *)self)->kind],size,result,written?*written:0);
    return result;
}
static int WIN user_BeginAuthSession(void *self,const void *ticket,int size,uint64_t steam_id) {
    return M(self,14,int(*)(void *,const void *,int,uint64_t))(L(self),ticket,size,steam_id);
}
static void WIN user_EndAuthSession(void *self,uint64_t steam_id) {M(self,15,void(*)(void *,uint64_t))(L(self),steam_id);}
static void WIN user_CancelAuthTicket(void *self,uint32_t ticket) {M(self,16,void(*)(void *,uint32_t))(L(self),ticket);}
static int WIN user_UserHasLicenseForApp(void *self,uint64_t steam_id,uint32_t app) {
    return M(self,17,int(*)(void *,uint64_t,uint32_t))(L(self),steam_id,app);
}
static bool WIN user_BIsBehindNAT(void *self) {return M(self,18,bool(*)(void *))(L(self));}
static uint64_t WIN user_RequestEncryptedAppTicket(void *self,void *data,int size) {
    return M(self,20,uint64_t(*)(void *,void *,int))(L(self),data,size);
}
static bool WIN user_GetEncryptedAppTicket(void *self,void *ticket,int size,uint32_t *written) {
    return M(self,21,bool(*)(void *,void *,int,uint32_t *))(L(self),ticket,size,written);
}
static int WIN user_GetGameBadgeLevel(void *self,int series,bool foil) {return M(self,22,int(*)(void *,int,bool))(L(self),series,foil);}
static int WIN user_GetPlayerSteamLevel(void *self) {return M(self,23,int(*)(void *))(L(self));}
static bool WIN user_BIsPhoneVerified(void *self) {return M(self,25,bool(*)(void *))(L(self));}
static bool WIN user_BIsTwoFactorEnabled(void *self) {return M(self,26,bool(*)(void *))(L(self));}
static bool WIN user_BIsPhoneIdentifying(void *self) {return M(self,27,bool(*)(void *))(L(self));}
static bool WIN user_BIsPhoneRequiringVerification(void *self) {return M(self,28,bool(*)(void *))(L(self));}

/* ISteamUtils010 */
static uint32_t WIN utils_GetSecondsSinceAppActive(void *self) {return M(self,0,uint32_t(*)(void *))(L(self));}
static uint32_t WIN utils_GetSecondsSinceComputerActive(void *self) {return M(self,1,uint32_t(*)(void *))(L(self));}
static int WIN utils_GetConnectedUniverse(void *self) {return M(self,2,int(*)(void *))(L(self));}
static uint32_t WIN utils_GetServerRealTime(void *self) {return M(self,3,uint32_t(*)(void *))(L(self));}
static const char *WIN utils_GetIPCountry(void *self) {return M(self,4,const char *(*)(void *))(L(self));}
static bool WIN utils_GetImageSize(void *self,int image,uint32_t *width,uint32_t *height) {
    return M(self,5,bool(*)(void *,int,uint32_t *,uint32_t *))(L(self),image,width,height);
}
static bool WIN utils_GetImageRGBA(void *self,int image,uint8_t *out,int size) {
    return M(self,6,bool(*)(void *,int,uint8_t *,int))(L(self),image,out,size);
}
static bool WIN utils_GetCSERIPPort(void *self,uint32_t *ip,uint16_t *port) {return M(self,7,bool(*)(void *,uint32_t *,uint16_t *))(L(self),ip,port);}
static uint8_t WIN utils_GetCurrentBatteryPower(void *self) {return M(self,8,uint8_t(*)(void *))(L(self));}
static uint32_t WIN utils_GetAppID(void *self) {return M(self,9,uint32_t(*)(void *))(L(self));}
static void WIN utils_SetOverlayNotificationPosition(void *self,int position) {M(self,10,void(*)(void *,int))(L(self),position);}
static bool WIN utils_IsAPICallCompleted(void *self,uint64_t call,bool *failed) {return M(self,11,bool(*)(void *,uint64_t,bool *))(L(self),call,failed);}
static int WIN utils_GetAPICallFailureReason(void *self,uint64_t call) {return M(self,12,int(*)(void *,uint64_t))(L(self),call);}
static bool fetch_utils(void *self,uint64_t call,void *data,int size,int expected,bool *failed) {
    return M(self,13,bool(*)(void *,uint64_t,void *,int,int,bool *))(L(self),call,data,size,expected,failed);
}
static bool WIN utils_GetAPICallResult(void *self,uint64_t call,void *data,int size,int expected,bool *failed) {
    return call_result(fetch_utils,self,call,data,size,expected,failed);
}
static void WIN utils_RunFrame(void *self) {M(self,14,void(*)(void *))(L(self));}
static uint32_t WIN utils_GetIPCCallCount(void *self) {return M(self,15,uint32_t(*)(void *))(L(self));}
static void WIN utils_SetWarningMessageHook(void *self,WinWarningHook hook) {
    utils_hook=hook;M(self,16,void(*)(void *,void(*)(int,const char *)))(L(self),hook?lin_utils_hook:NULL);
}
static bool WIN utils_IsOverlayEnabled(void *self) {return M(self,17,bool(*)(void *))(L(self));}
static bool WIN utils_BOverlayNeedsPresent(void *self) {return M(self,18,bool(*)(void *))(L(self));}
static const char *WIN utils_GetSteamUILanguage(void *self) {return M(self,23,const char *(*)(void *))(L(self));}
static bool WIN utils_IsSteamRunningInVR(void *self) {return M(self,24,bool(*)(void *))(L(self));}
static void WIN utils_SetOverlayNotificationInset(void *self,int x,int y) {M(self,25,void(*)(void *,int,int))(L(self),x,y);}
static bool WIN utils_IsSteamInBigPictureMode(void *self) {return M(self,26,bool(*)(void *))(L(self));}
static bool WIN utils_IsSteamChinaLauncher(void *self) {return M(self,30,bool(*)(void *))(L(self));}
static bool WIN utils_IsSteamRunningOnSteamDeck(void *self) {return M(self,34,bool(*)(void *))(L(self));}

/* ISteamFriends017 */
static const char *WIN friends_GetPersonaName(void *self) {return M(self,0,const char *(*)(void *))(L(self));}
static uint64_t WIN friends_SetPersonaName(void *self,const char *name) {return M(self,1,uint64_t(*)(void *,const char *))(L(self),name);}
static int WIN friends_GetPersonaState(void *self) {return M(self,2,int(*)(void *))(L(self));}
static int WIN friends_GetFriendCount(void *self,int flags) {return M(self,3,int(*)(void *,int))(L(self),flags);}
static uint64_t *WIN friends_GetFriendByIndex(void *self,uint64_t *result,int index,int flags) {
    *result=M(self,4,uint64_t(*)(void *,int,int))(L(self),index,flags);return result;
}
static int WIN friends_GetFriendRelationship(void *self,uint64_t friend_id) {return M(self,5,int(*)(void *,uint64_t))(L(self),friend_id);}
static int WIN friends_GetFriendPersonaState(void *self,uint64_t friend_id) {return M(self,6,int(*)(void *,uint64_t))(L(self),friend_id);}
static const char *WIN friends_GetFriendPersonaName(void *self,uint64_t friend_id) {return M(self,7,const char *(*)(void *,uint64_t))(L(self),friend_id);}
static bool WIN friends_GetFriendGamePlayed(void *self,uint64_t friend_id,void *info) {
    return M(self,8,bool(*)(void *,uint64_t,void *))(L(self),friend_id,info);
}
static const char *WIN friends_GetFriendPersonaNameHistory(void *self,uint64_t friend_id,int index) {
    return M(self,9,const char *(*)(void *,uint64_t,int))(L(self),friend_id,index);
}
static int WIN friends_GetFriendSteamLevel(void *self,uint64_t friend_id) {return M(self,10,int(*)(void *,uint64_t))(L(self),friend_id);}
static const char *WIN friends_GetPlayerNickname(void *self,uint64_t friend_id) {return M(self,11,const char *(*)(void *,uint64_t))(L(self),friend_id);}
static int WIN friends_GetFriendsGroupCount(void *self) {return M(self,12,int(*)(void *))(L(self));}
static int16_t WIN friends_GetFriendsGroupIDByIndex(void *self,int index) {return M(self,13,int16_t(*)(void *,int))(L(self),index);}
static const char *WIN friends_GetFriendsGroupName(void *self,int16_t group_id) {return M(self,14,const char *(*)(void *,int16_t))(L(self),group_id);}
static int WIN friends_GetFriendsGroupMembersCount(void *self,int16_t group_id) {return M(self,15,int(*)(void *,int16_t))(L(self),group_id);}
static void WIN friends_GetFriendsGroupMembersList(void *self,int16_t group_id,uint64_t *out,int count) {
    M(self,16,void(*)(void *,int16_t,uint64_t *,int))(L(self),group_id,out,count);
}
static bool WIN friends_HasFriend(void *self,uint64_t friend_id,int flags) {return M(self,17,bool(*)(void *,uint64_t,int))(L(self),friend_id,flags);}
static int WIN friends_GetClanCount(void *self) {return M(self,18,int(*)(void *))(L(self));}
static uint64_t *WIN friends_GetClanByIndex(void *self,uint64_t *result,int index) {
    *result=M(self,19,uint64_t(*)(void *,int))(L(self),index);return result;
}
static const char *WIN friends_GetClanName(void *self,uint64_t clan_id) {return M(self,20,const char *(*)(void *,uint64_t))(L(self),clan_id);}
static const char *WIN friends_GetClanTag(void *self,uint64_t clan_id) {return M(self,21,const char *(*)(void *,uint64_t))(L(self),clan_id);}
static bool WIN friends_GetClanActivityCounts(void *self,uint64_t clan_id,int *online,int *in_game,int *chatting) {
    return M(self,22,bool(*)(void *,uint64_t,int *,int *,int *))(L(self),clan_id,online,in_game,chatting);
}
static uint64_t WIN friends_DownloadClanActivityCounts(void *self,uint64_t *clans,int count) {
    return M(self,23,uint64_t(*)(void *,uint64_t *,int))(L(self),clans,count);
}
static int WIN friends_GetFriendCountFromSource(void *self,uint64_t source_id) {return M(self,24,int(*)(void *,uint64_t))(L(self),source_id);}
static uint64_t *WIN friends_GetFriendFromSourceByIndex(void *self,uint64_t *result,uint64_t source_id,int index) {
    *result=M(self,25,uint64_t(*)(void *,uint64_t,int))(L(self),source_id,index);return result;
}
static bool WIN friends_IsUserInSource(void *self,uint64_t user_id,uint64_t source_id) {
    return M(self,26,bool(*)(void *,uint64_t,uint64_t))(L(self),user_id,source_id);
}
static void WIN friends_SetInGameVoiceSpeaking(void *self,uint64_t user_id,bool speaking) {
    M(self,27,void(*)(void *,uint64_t,bool))(L(self),user_id,speaking);
}
static void WIN friends_ActivateGameOverlay(void *self,const char *dialog) {M(self,28,void(*)(void *,const char *))(L(self),dialog);}
static void WIN friends_ActivateGameOverlayToUser(void *self,const char *dialog,uint64_t steam_id) {
    M(self,29,void(*)(void *,const char *,uint64_t))(L(self),dialog,steam_id);
}
static void WIN friends_ActivateGameOverlayToWebPage(void *self,const char *url,int mode) {
    M(self,30,void(*)(void *,const char *,int))(L(self),url,mode);
}
static void WIN friends_ActivateGameOverlayToStore(void *self,uint32_t app_id,int flag) {
    M(self,31,void(*)(void *,uint32_t,int))(L(self),app_id,flag);
}
static void WIN friends_SetPlayedWith(void *self,uint64_t steam_id) {M(self,32,void(*)(void *,uint64_t))(L(self),steam_id);}
static void WIN friends_ActivateGameOverlayInviteDialog(void *self,uint64_t lobby_id) {
    M(self,33,void(*)(void *,uint64_t))(L(self),lobby_id);
}
static int WIN friends_GetSmallFriendAvatar(void *self,uint64_t steam_id) {return M(self,34,int(*)(void *,uint64_t))(L(self),steam_id);}
static int WIN friends_GetMediumFriendAvatar(void *self,uint64_t steam_id) {return M(self,35,int(*)(void *,uint64_t))(L(self),steam_id);}
static int WIN friends_GetLargeFriendAvatar(void *self,uint64_t steam_id) {return M(self,36,int(*)(void *,uint64_t))(L(self),steam_id);}
static bool WIN friends_RequestUserInformation(void *self,uint64_t steam_id,bool require_name_only) {
    return M(self,37,bool(*)(void *,uint64_t,bool))(L(self),steam_id,require_name_only);
}
static uint64_t WIN friends_RequestClanOfficerList(void *self,uint64_t clan_id) {
    return M(self,38,uint64_t(*)(void *,uint64_t))(L(self),clan_id);
}
static uint64_t *WIN friends_GetClanOwner(void *self,uint64_t *result,uint64_t clan_id) {
    *result=M(self,39,uint64_t(*)(void *,uint64_t))(L(self),clan_id);return result;
}
static int WIN friends_GetClanOfficerCount(void *self,uint64_t clan_id) {return M(self,40,int(*)(void *,uint64_t))(L(self),clan_id);}
static uint64_t *WIN friends_GetClanOfficerByIndex(void *self,uint64_t *result,uint64_t clan_id,int index) {
    *result=M(self,41,uint64_t(*)(void *,uint64_t,int))(L(self),clan_id,index);return result;
}
static uint32_t WIN friends_GetUserRestrictions(void *self) {return M(self,42,uint32_t(*)(void *))(L(self));}
static bool WIN friends_SetRichPresence(void *self,const char *key,const char *val) {
    return M(self,43,bool(*)(void *,const char *,const char *))(L(self),key,val);
}
static void WIN friends_ClearRichPresence(void *self) {M(self,44,void(*)(void *))(L(self));}
static const char *WIN friends_GetFriendRichPresence(void *self,uint64_t steam_id,const char *key) {
    return M(self,45,const char *(*)(void *,uint64_t,const char *))(L(self),steam_id,key);
}
static int WIN friends_GetFriendRichPresenceKeyCount(void *self,uint64_t steam_id) {
    return M(self,46,int(*)(void *,uint64_t))(L(self),steam_id);
}
static const char *WIN friends_GetFriendRichPresenceKeyByIndex(void *self,uint64_t steam_id,int index) {
    return M(self,47,const char *(*)(void *,uint64_t,int))(L(self),steam_id,index);
}
static void WIN friends_RequestFriendRichPresence(void *self,uint64_t steam_id) {
    M(self,48,void(*)(void *,uint64_t))(L(self),steam_id);
}
static bool WIN friends_InviteUserToGame(void *self,uint64_t steam_id,const char *connect_string) {
    return M(self,49,bool(*)(void *,uint64_t,const char *))(L(self),steam_id,connect_string);
}
static int WIN friends_GetCoplayFriendCount(void *self) {return M(self,50,int(*)(void *))(L(self));}
static uint64_t *WIN friends_GetCoplayFriend(void *self,uint64_t *result,int index) {
    *result=M(self,51,uint64_t(*)(void *,int))(L(self),index);return result;
}
static int WIN friends_GetFriendCoplayTime(void *self,uint64_t steam_id) {return M(self,52,int(*)(void *,uint64_t))(L(self),steam_id);}
static uint32_t WIN friends_GetFriendCoplayGame(void *self,uint64_t steam_id) {return M(self,53,uint32_t(*)(void *,uint64_t))(L(self),steam_id);}
static uint64_t WIN friends_JoinClanChatRoom(void *self,uint64_t clan_id) {return M(self,54,uint64_t(*)(void *,uint64_t))(L(self),clan_id);}
static bool WIN friends_LeaveClanChatRoom(void *self,uint64_t clan_id) {return M(self,55,bool(*)(void *,uint64_t))(L(self),clan_id);}
static int WIN friends_GetClanChatMemberCount(void *self,uint64_t clan_id) {return M(self,56,int(*)(void *,uint64_t))(L(self),clan_id);}
static uint64_t *WIN friends_GetChatMemberByIndex(void *self,uint64_t *result,uint64_t clan_id,int index) {
    *result=M(self,57,uint64_t(*)(void *,uint64_t,int))(L(self),clan_id,index);return result;
}
static bool WIN friends_SendClanChatMessage(void *self,uint64_t chat_id,const char *text) {
    return M(self,58,bool(*)(void *,uint64_t,const char *))(L(self),chat_id,text);
}
static int WIN friends_GetClanChatMessage(void *self,uint64_t chat_id,int msg,void *text,int max,int *type,uint64_t *chatter) {
    return M(self,59,int(*)(void *,uint64_t,int,void *,int,int *,uint64_t *))(L(self),chat_id,msg,text,max,type,chatter);
}
static bool WIN friends_IsClanChatAdmin(void *self,uint64_t chat_id,uint64_t user_id) {
    return M(self,60,bool(*)(void *,uint64_t,uint64_t))(L(self),chat_id,user_id);
}
static bool WIN friends_IsClanChatWindowOpenInSteam(void *self,uint64_t chat_id) {
    return M(self,61,bool(*)(void *,uint64_t))(L(self),chat_id);
}
static bool WIN friends_OpenClanChatWindowInSteam(void *self,uint64_t chat_id) {
    return M(self,62,bool(*)(void *,uint64_t))(L(self),chat_id);
}
static bool WIN friends_CloseClanChatWindowInSteam(void *self,uint64_t chat_id) {
    return M(self,63,bool(*)(void *,uint64_t))(L(self),chat_id);
}
static bool WIN friends_SetListenForFriendsMessages(void *self,bool intercept) {
    return M(self,64,bool(*)(void *,bool))(L(self),intercept);
}
static bool WIN friends_ReplyToFriendMessage(void *self,uint64_t friend_id,const char *msg) {
    return M(self,65,bool(*)(void *,uint64_t,const char *))(L(self),friend_id,msg);
}
static int WIN friends_GetFriendMessage(void *self,uint64_t friend_id,int id,void *data,int max,int *type) {
    return M(self,66,int(*)(void *,uint64_t,int,void *,int,int *))(L(self),friend_id,id,data,max,type);
}
static uint64_t WIN friends_GetFollowerCount(void *self,uint64_t steam_id) {return M(self,67,uint64_t(*)(void *,uint64_t))(L(self),steam_id);}
static uint64_t WIN friends_IsFollowing(void *self,uint64_t steam_id) {return M(self,68,uint64_t(*)(void *,uint64_t))(L(self),steam_id);}
static uint64_t WIN friends_EnumerateFollowingList(void *self,uint32_t start) {return M(self,69,uint64_t(*)(void *,uint32_t))(L(self),start);}
static bool WIN friends_IsClanPublic(void *self,uint64_t clan_id) {return M(self,70,bool(*)(void *,uint64_t))(L(self),clan_id);}
static bool WIN friends_IsClanOfficialGameGroup(void *self,uint64_t clan_id) {return M(self,71,bool(*)(void *,uint64_t))(L(self),clan_id);}
static int WIN friends_GetNumChatsWithUnreadPriorityMessages(void *self) {return M(self,72,int(*)(void *))(L(self));}
static void WIN friends_ActivateGameOverlayRemotePlayTogetherInviteDialog(void *self,uint64_t lobby_id) {
    M(self,73,void(*)(void *,uint64_t))(L(self),lobby_id);
}
static bool WIN friends_RegisterProtocolInOverlayBrowser(void *self,const char *protocol) {
    return M(self,74,bool(*)(void *,const char *))(L(self),protocol);
}
static void WIN friends_ActivateGameOverlayInviteDialogConnectString(void *self,const char *connect_string) {
    M(self,75,void(*)(void *,const char *))(L(self),connect_string);
}
static uint64_t WIN friends_RequestEquippedProfileItems(void *self,uint64_t steam_id) {
    return M(self,76,uint64_t(*)(void *,uint64_t))(L(self),steam_id);
}
static bool WIN friends_BHasEquippedProfileItem(void *self,uint64_t steam_id,int item_type) {
    return M(self,77,bool(*)(void *,uint64_t,int))(L(self),steam_id,item_type);
}
static const char *WIN friends_GetProfileItemPropertyString(void *self,uint64_t steam_id,int item_type,int prop) {
    return M(self,78,const char *(*)(void *,uint64_t,int,int))(L(self),steam_id,item_type,prop);
}
static uint32_t WIN friends_GetProfileItemPropertyUint(void *self,uint64_t steam_id,int item_type,int prop) {
    return M(self,79,uint32_t(*)(void *,uint64_t,int,int))(L(self),steam_id,item_type,prop);
}

/* ISteamController007 */
#pragma pack(push,1)
typedef struct {int mode;float x,y;bool active;} AnalogActionData;
typedef struct {bool state;bool active;} DigitalActionData;
typedef struct {float rot[4];float pos[3];float vel[3];} MotionData;
#pragma pack(pop)

static bool WIN controller_Init(void *self) {return M(self,0,bool(*)(void *))(L(self));}
static bool WIN controller_Shutdown(void *self) {return M(self,1,bool(*)(void *))(L(self));}
static void WIN controller_RunFrame(void *self) {M(self,2,void(*)(void *))(L(self));}
static int WIN controller_GetConnectedControllers(void *self,uint64_t *out) {
    return M(self,3,int(*)(void *,uint64_t *))(L(self),out);
}
static uint64_t WIN controller_GetActionSetHandle(void *self,const char *name) {
    return M(self,4,uint64_t(*)(void *,const char *))(L(self),name);
}
static void WIN controller_ActivateActionSet(void *self,uint64_t handle,uint64_t set) {
    M(self,5,void(*)(void *,uint64_t,uint64_t))(L(self),handle,set);
}
static uint64_t WIN controller_GetCurrentActionSet(void *self,uint64_t handle) {
    return M(self,6,uint64_t(*)(void *,uint64_t))(L(self),handle);
}
static void WIN controller_ActivateActionSetLayer(void *self,uint64_t handle,uint64_t layer) {
    M(self,7,void(*)(void *,uint64_t,uint64_t))(L(self),handle,layer);
}
static void WIN controller_DeactivateActionSetLayer(void *self,uint64_t handle,uint64_t layer) {
    M(self,8,void(*)(void *,uint64_t,uint64_t))(L(self),handle,layer);
}
static void WIN controller_DeactivateAllActionSetLayers(void *self,uint64_t handle) {
    M(self,9,void(*)(void *,uint64_t))(L(self),handle);
}
static int WIN controller_GetActiveActionSetLayers(void *self,uint64_t handle,uint64_t *out) {
    return M(self,10,int(*)(void *,uint64_t,uint64_t *))(L(self),handle,out);
}
static uint64_t WIN controller_GetDigitalActionHandle(void *self,const char *name) {
    return M(self,11,uint64_t(*)(void *,const char *))(L(self),name);
}
static DigitalActionData *WIN controller_GetDigitalActionData(void *self,DigitalActionData *result,uint64_t handle,uint64_t action) {
    *result=M(self,12,DigitalActionData(*)(void *,uint64_t,uint64_t))(L(self),handle,action);return result;
}
static int WIN controller_GetDigitalActionOrigins(void *self,uint64_t handle,uint64_t set,uint64_t action,int *out) {
    return M(self,13,int(*)(void *,uint64_t,uint64_t,uint64_t,int *))(L(self),handle,set,action,out);
}
static uint64_t WIN controller_GetAnalogActionHandle(void *self,const char *name) {
    return M(self,14,uint64_t(*)(void *,const char *))(L(self),name);
}
static AnalogActionData *WIN controller_GetAnalogActionData(void *self,AnalogActionData *result,uint64_t handle,uint64_t action) {
    *result=M(self,15,AnalogActionData(*)(void *,uint64_t,uint64_t))(L(self),handle,action);return result;
}
static int WIN controller_GetAnalogActionOrigins(void *self,uint64_t handle,uint64_t set,uint64_t action,int *out) {
    return M(self,16,int(*)(void *,uint64_t,uint64_t,uint64_t,int *))(L(self),handle,set,action,out);
}
static const char *WIN controller_GetGlyphForActionOrigin(void *self,int origin) {
    return M(self,17,const char *(*)(void *,int))(L(self),origin);
}
static const char *WIN controller_GetStringForActionOrigin(void *self,int origin) {
    return M(self,18,const char *(*)(void *,int))(L(self),origin);
}
static void WIN controller_StopAnalogActionMomentum(void *self,uint64_t handle,uint64_t action) {
    M(self,19,void(*)(void *,uint64_t,uint64_t))(L(self),handle,action);
}
static MotionData *WIN controller_GetMotionData(void *self,MotionData *result,uint64_t handle) {
    *result=M(self,20,MotionData(*)(void *,uint64_t))(L(self),handle);return result;
}
static void WIN controller_TriggerHapticPulse(void *self,uint64_t handle,int pad,uint16_t duration) {
    M(self,21,void(*)(void *,uint64_t,int,uint16_t))(L(self),handle,pad,duration);
}
static void WIN controller_TriggerRepeatedHapticPulse(void *self,uint64_t handle,int pad,uint16_t duration,uint16_t off,uint16_t repeat,uint32_t flags) {
    M(self,22,void(*)(void *,uint64_t,int,uint16_t,uint16_t,uint16_t,uint32_t))(L(self),handle,pad,duration,off,repeat,flags);
}
static void WIN controller_TriggerVibration(void *self,uint64_t handle,uint16_t left,uint16_t right) {
    M(self,23,void(*)(void *,uint64_t,uint16_t,uint16_t))(L(self),handle,left,right);
}
static void WIN controller_SetLEDColor(void *self,uint64_t handle,uint8_t r,uint8_t g,uint8_t b,uint32_t flags) {
    M(self,24,void(*)(void *,uint64_t,uint8_t,uint8_t,uint8_t,uint32_t))(L(self),handle,r,g,b,flags);
}
static bool WIN controller_ShowBindingPanel(void *self,uint64_t handle) {
    return M(self,25,bool(*)(void *,uint64_t))(L(self),handle);
}
static int WIN controller_GetInputTypeForHandle(void *self,uint64_t handle) {
    return M(self,26,int(*)(void *,uint64_t))(L(self),handle);
}
static uint64_t WIN controller_GetControllerForGamepadIndex(void *self,int index) {
    return M(self,27,uint64_t(*)(void *,int))(L(self),index);
}
static int WIN controller_GetGamepadIndexForController(void *self,uint64_t handle) {
    return M(self,28,int(*)(void *,uint64_t))(L(self),handle);
}
static const char *WIN controller_GetStringForXboxOrigin(void *self,int origin) {
    return M(self,29,const char *(*)(void *,int))(L(self),origin);
}
static const char *WIN controller_GetGlyphForXboxOrigin(void *self,int origin) {
    return M(self,30,const char *(*)(void *,int))(L(self),origin);
}
static int WIN controller_GetActionOriginFromXboxOrigin(void *self,uint64_t handle,int origin) {
    return M(self,31,int(*)(void *,uint64_t,int))(L(self),handle,origin);
}
static int WIN controller_TranslateActionOrigin(void *self,int dest,int src) {
    return M(self,32,int(*)(void *,int,int))(L(self),dest,src);
}
static bool WIN controller_GetControllerBindingRevision(void *self,uint64_t handle,int *major,int *minor) {
    return M(self,33,bool(*)(void *,uint64_t,int *,int *))(L(self),handle,major,minor);
}

/* ISteamApps008 */
static bool WIN apps_BIsSubscribed(void *self) {return M(self,0,bool(*)(void *))(L(self));}
static bool WIN apps_BIsLowViolence(void *self) {return M(self,1,bool(*)(void *))(L(self));}
static bool WIN apps_BIsCybercafe(void *self) {return M(self,2,bool(*)(void *))(L(self));}
static bool WIN apps_BIsVACBanned(void *self) {return M(self,3,bool(*)(void *))(L(self));}
static const char *WIN apps_GetCurrentGameLanguage(void *self) {return M(self,4,const char *(*)(void *))(L(self));}
static const char *WIN apps_GetAvailableGameLanguages(void *self) {return M(self,5,const char *(*)(void *))(L(self));}
static bool WIN apps_BIsSubscribedApp(void *self,uint32_t app) {return M(self,6,bool(*)(void *,uint32_t))(L(self),app);}
static bool WIN apps_BIsDlcInstalled(void *self,uint32_t app) {return M(self,7,bool(*)(void *,uint32_t))(L(self),app);}
static uint32_t WIN apps_GetEarliestPurchaseUnixTime(void *self,uint32_t app) {return M(self,8,uint32_t(*)(void *,uint32_t))(L(self),app);}
static bool WIN apps_BIsSubscribedFromFreeWeekend(void *self) {return M(self,9,bool(*)(void *))(L(self));}
static int WIN apps_GetDLCCount(void *self) {return M(self,10,int(*)(void *))(L(self));}
static bool WIN apps_BGetDLCDataByIndex(void *self,int i,uint32_t *app,bool *avail,char *name,int max_len) {
    return M(self,11,bool(*)(void *,int,uint32_t *,bool *,char *,int))(L(self),i,app,avail,name,max_len);
}
static void WIN apps_InstallDLC(void *self,uint32_t app) {M(self,12,void(*)(void *,uint32_t))(L(self),app);}
static void WIN apps_UninstallDLC(void *self,uint32_t app) {M(self,13,void(*)(void *,uint32_t))(L(self),app);}
static void WIN apps_RequestAppProofOfPurchaseKey(void *self,uint32_t app) {M(self,14,void(*)(void *,uint32_t))(L(self),app);}
static bool WIN apps_GetCurrentBetaName(void *self,char *name,int max_len) {return M(self,15,bool(*)(void *,char *,int))(L(self),name,max_len);}
static bool WIN apps_MarkContentCorrupt(void *self,bool missing) {return M(self,16,bool(*)(void *,bool))(L(self),missing);}
static uint32_t WIN apps_GetInstalledDepots(void *self,uint32_t app,uint32_t *depots,uint32_t max_depots) {
    return M(self,17,uint32_t(*)(void *,uint32_t,uint32_t *,uint32_t))(L(self),app,depots,max_depots);
}
static uint32_t WIN apps_GetAppInstallDir(void *self,uint32_t app,char *folder,uint32_t max_folder) {
    return M(self,18,uint32_t(*)(void *,uint32_t,char *,uint32_t))(L(self),app,folder,max_folder);
}
static bool WIN apps_BIsAppInstalled(void *self,uint32_t app) {return M(self,19,bool(*)(void *,uint32_t))(L(self),app);}
static uint64_t *WIN apps_GetAppOwner(void *self,uint64_t *result) {
    *result=M(self,20,uint64_t(*)(void *))(L(self));
    return result;
}
static const char *WIN apps_GetLaunchQueryParam(void *self,const char *key) {return M(self,21,const char *(*)(void *,const char *))(L(self),key);}
static bool WIN apps_GetDlcDownloadProgress(void *self,uint32_t app,uint64_t *dl,uint64_t *total) {
    return M(self,22,bool(*)(void *,uint32_t,uint64_t *,uint64_t *))(L(self),app,dl,total);
}
static int WIN apps_GetAppBuildId(void *self) {return M(self,23,int(*)(void *))(L(self));}
static void WIN apps_RequestAllProofOfPurchaseKeys(void *self) {M(self,24,void(*)(void *))(L(self));}
static uint64_t WIN apps_GetFileDetails(void *self,const char *name) {return M(self,25,uint64_t(*)(void *,const char *))(L(self),name);}
static int WIN apps_GetLaunchCommandLine(void *self,char *cmdline,int max_cmdline) {
    return M(self,26,int(*)(void *,char *,int))(L(self),cmdline,max_cmdline);
}
static bool WIN apps_BIsSubscribedFromFamilySharing(void *self) {return M(self,27,bool(*)(void *))(L(self));}
static bool WIN apps_BIsTimedTrial(void *self,uint32_t *allowed,uint32_t *played) {
    return M(self,28,bool(*)(void *,uint32_t *,uint32_t *))(L(self),allowed,played);
}
static bool WIN apps_SetDlcContext(void *self,uint32_t app) {return M(self,29,bool(*)(void *,uint32_t))(L(self),app);}
static int WIN apps_GetNumBetas(void *self,int *avail,int *priv) {return M(self,30,int(*)(void *,int *,int *))(L(self),avail,priv);}
static bool WIN apps_GetBetaInfo(void *self,int i,uint32_t *flags,uint32_t *build_id,char *beta_name,int cch_beta,char *desc,int cch_desc,uint32_t *updated) {
    return M(self,31,bool(*)(void *,int,uint32_t *,uint32_t *,char *,int,char *,int,uint32_t *))(L(self),i,flags,build_id,beta_name,cch_beta,desc,cch_desc,updated);
}
static bool WIN apps_SetActiveBeta(void *self,const char *beta) {return M(self,32,bool(*)(void *,const char *))(L(self),beta);}
static void WIN apps_SetGamePerformanceSetting(void *self,int setting) {M(self,33,void(*)(void *,int))(L(self),setting);}
static void WIN apps_SetGameRenderResolution(void *self,uint32_t w,uint32_t h) {M(self,34,void(*)(void *,uint32_t,uint32_t))(L(self),w,h);}

/* ISteamRemoteStorage014: Steam Cloud files. Same slot order on both sides; no structures
 * by value. Async calls return a SteamAPICall_t whose result crosses through call_result. */
static bool WIN rs_FileWrite(void *self,const char *file,const void *data,int32_t size) {
    return M(self,0,bool(*)(void *,const char *,const void *,int32_t))(L(self),file,data,size);
}
static int32_t WIN rs_FileRead(void *self,const char *file,void *data,int32_t size) {
    return M(self,1,int32_t(*)(void *,const char *,void *,int32_t))(L(self),file,data,size);
}
static uint64_t WIN rs_FileWriteAsync(void *self,const char *file,const void *data,uint32_t size) {
    return M(self,2,uint64_t(*)(void *,const char *,const void *,uint32_t))(L(self),file,data,size);
}
static uint64_t WIN rs_FileReadAsync(void *self,const char *file,uint32_t offset,uint32_t size) {
    return M(self,3,uint64_t(*)(void *,const char *,uint32_t,uint32_t))(L(self),file,offset,size);
}
static bool WIN rs_FileReadAsyncComplete(void *self,uint64_t call,void *buffer,uint32_t size) {
    return M(self,4,bool(*)(void *,uint64_t,void *,uint32_t))(L(self),call,buffer,size);
}
static bool WIN rs_FileForget(void *self,const char *file) {return M(self,5,bool(*)(void *,const char *))(L(self),file);}
static bool WIN rs_FileDelete(void *self,const char *file) {return M(self,6,bool(*)(void *,const char *))(L(self),file);}
static uint64_t WIN rs_FileShare(void *self,const char *file) {return M(self,7,uint64_t(*)(void *,const char *))(L(self),file);}
static bool WIN rs_SetSyncPlatforms(void *self,const char *file,int platforms) {
    return M(self,8,bool(*)(void *,const char *,int))(L(self),file,platforms);
}
static uint64_t WIN rs_FileWriteStreamOpen(void *self,const char *file) {return M(self,9,uint64_t(*)(void *,const char *))(L(self),file);}
static bool WIN rs_FileWriteStreamWriteChunk(void *self,uint64_t stream,const void *data,int32_t size) {
    return M(self,10,bool(*)(void *,uint64_t,const void *,int32_t))(L(self),stream,data,size);
}
static bool WIN rs_FileWriteStreamClose(void *self,uint64_t stream) {return M(self,11,bool(*)(void *,uint64_t))(L(self),stream);}
static bool WIN rs_FileWriteStreamCancel(void *self,uint64_t stream) {return M(self,12,bool(*)(void *,uint64_t))(L(self),stream);}
static bool WIN rs_FileExists(void *self,const char *file) {return M(self,13,bool(*)(void *,const char *))(L(self),file);}
static bool WIN rs_FilePersisted(void *self,const char *file) {return M(self,14,bool(*)(void *,const char *))(L(self),file);}
static int32_t WIN rs_GetFileSize(void *self,const char *file) {return M(self,15,int32_t(*)(void *,const char *))(L(self),file);}
static int64_t WIN rs_GetFileTimestamp(void *self,const char *file) {return M(self,16,int64_t(*)(void *,const char *))(L(self),file);}
static int WIN rs_GetSyncPlatforms(void *self,const char *file) {return M(self,17,int(*)(void *,const char *))(L(self),file);}
static int32_t WIN rs_GetFileCount(void *self) {return M(self,18,int32_t(*)(void *))(L(self));}
static const char *WIN rs_GetFileNameAndSize(void *self,int index,int32_t *size) {
    return M(self,19,const char *(*)(void *,int,int32_t *))(L(self),index,size);
}
static bool WIN rs_GetQuota(void *self,uint64_t *total,uint64_t *available) {
    return M(self,20,bool(*)(void *,uint64_t *,uint64_t *))(L(self),total,available);
}
static bool WIN rs_IsCloudEnabledForAccount(void *self) {return M(self,21,bool(*)(void *))(L(self));}
static bool WIN rs_IsCloudEnabledForApp(void *self) {return M(self,22,bool(*)(void *))(L(self));}
static void WIN rs_SetCloudEnabledForApp(void *self,bool enabled) {M(self,23,void(*)(void *,bool))(L(self),enabled);}

/* ISteamUserStats011. MSVC reverses each overload group in the vtable, so Windows slots 1-4,
 * 17-18 and 39-42 reach the Linux (declaration-order) slots of the other overload.
 * GetDownloadedLeaderboardEntry stays a trap: LeaderboardEntry_t is 28 bytes on Linux, 32 on Windows. */
static bool WIN us_RequestCurrentStats(void *self) {return M(self,0,bool(*)(void *))(L(self));}
static bool WIN us_GetStatFloat(void *self,const char *name,float *data) {return M(self,2,bool(*)(void *,const char *,float *))(L(self),name,data);}
static bool WIN us_GetStatInt(void *self,const char *name,int32_t *data) {return M(self,1,bool(*)(void *,const char *,int32_t *))(L(self),name,data);}
static bool WIN us_SetStatFloat(void *self,const char *name,float data) {return M(self,4,bool(*)(void *,const char *,float))(L(self),name,data);}
static bool WIN us_SetStatInt(void *self,const char *name,int32_t data) {return M(self,3,bool(*)(void *,const char *,int32_t))(L(self),name,data);}
static bool WIN us_UpdateAvgRateStat(void *self,const char *name,float count,double length) {
    return M(self,5,bool(*)(void *,const char *,float,double))(L(self),name,count,length);
}
static bool WIN us_GetAchievement(void *self,const char *name,bool *achieved) {return M(self,6,bool(*)(void *,const char *,bool *))(L(self),name,achieved);}
static bool WIN us_SetAchievement(void *self,const char *name) {return M(self,7,bool(*)(void *,const char *))(L(self),name);}
static bool WIN us_ClearAchievement(void *self,const char *name) {return M(self,8,bool(*)(void *,const char *))(L(self),name);}
static bool WIN us_GetAchievementAndUnlockTime(void *self,const char *name,bool *achieved,uint32_t *time) {
    return M(self,9,bool(*)(void *,const char *,bool *,uint32_t *))(L(self),name,achieved,time);
}
static bool WIN us_StoreStats(void *self) {return M(self,10,bool(*)(void *))(L(self));}
static int WIN us_GetAchievementIcon(void *self,const char *name) {return M(self,11,int(*)(void *,const char *))(L(self),name);}
static const char *WIN us_GetAchievementDisplayAttribute(void *self,const char *name,const char *key) {
    return M(self,12,const char *(*)(void *,const char *,const char *))(L(self),name,key);
}
static bool WIN us_IndicateAchievementProgress(void *self,const char *name,uint32_t current,uint32_t max) {
    return M(self,13,bool(*)(void *,const char *,uint32_t,uint32_t))(L(self),name,current,max);
}
static uint32_t WIN us_GetNumAchievements(void *self) {return M(self,14,uint32_t(*)(void *))(L(self));}
static const char *WIN us_GetAchievementName(void *self,uint32_t index) {return M(self,15,const char *(*)(void *,uint32_t))(L(self),index);}
static uint64_t WIN us_RequestUserStats(void *self,uint64_t user) {return M(self,16,uint64_t(*)(void *,uint64_t))(L(self),user);}
static bool WIN us_GetUserStatFloat(void *self,uint64_t user,const char *name,float *data) {
    return M(self,18,bool(*)(void *,uint64_t,const char *,float *))(L(self),user,name,data);
}
static bool WIN us_GetUserStatInt(void *self,uint64_t user,const char *name,int32_t *data) {
    return M(self,17,bool(*)(void *,uint64_t,const char *,int32_t *))(L(self),user,name,data);
}
static bool WIN us_GetUserAchievement(void *self,uint64_t user,const char *name,bool *achieved) {
    return M(self,19,bool(*)(void *,uint64_t,const char *,bool *))(L(self),user,name,achieved);
}
static bool WIN us_GetUserAchievementAndUnlockTime(void *self,uint64_t user,const char *name,bool *achieved,uint32_t *time) {
    return M(self,20,bool(*)(void *,uint64_t,const char *,bool *,uint32_t *))(L(self),user,name,achieved,time);
}
static bool WIN us_ResetAllStats(void *self,bool achievements) {return M(self,21,bool(*)(void *,bool))(L(self),achievements);}
static uint64_t WIN us_FindOrCreateLeaderboard(void *self,const char *name,int sort,int display) {
    return M(self,22,uint64_t(*)(void *,const char *,int,int))(L(self),name,sort,display);
}
static uint64_t WIN us_FindLeaderboard(void *self,const char *name) {return M(self,23,uint64_t(*)(void *,const char *))(L(self),name);}
static const char *WIN us_GetLeaderboardName(void *self,uint64_t board) {return M(self,24,const char *(*)(void *,uint64_t))(L(self),board);}
static int WIN us_GetLeaderboardEntryCount(void *self,uint64_t board) {return M(self,25,int(*)(void *,uint64_t))(L(self),board);}
static int WIN us_GetLeaderboardSortMethod(void *self,uint64_t board) {return M(self,26,int(*)(void *,uint64_t))(L(self),board);}
static int WIN us_GetLeaderboardDisplayType(void *self,uint64_t board) {return M(self,27,int(*)(void *,uint64_t))(L(self),board);}
static uint64_t WIN us_DownloadLeaderboardEntries(void *self,uint64_t board,int request,int start,int end) {
    return M(self,28,uint64_t(*)(void *,uint64_t,int,int,int))(L(self),board,request,start,end);
}
static uint64_t WIN us_DownloadLeaderboardEntriesForUsers(void *self,uint64_t board,uint64_t *users,int count) {
    return M(self,29,uint64_t(*)(void *,uint64_t,uint64_t *,int))(L(self),board,users,count);
}
static uint64_t WIN us_UploadLeaderboardScore(void *self,uint64_t board,int method,int32_t score,const int32_t *details,int count) {
    return M(self,31,uint64_t(*)(void *,uint64_t,int,int32_t,const int32_t *,int))(L(self),board,method,score,details,count);
}
static uint64_t WIN us_AttachLeaderboardUGC(void *self,uint64_t board,uint64_t ugc) {
    return M(self,32,uint64_t(*)(void *,uint64_t,uint64_t))(L(self),board,ugc);
}
static uint64_t WIN us_GetNumberOfCurrentPlayers(void *self) {return M(self,33,uint64_t(*)(void *))(L(self));}
static uint64_t WIN us_RequestGlobalAchievementPercentages(void *self) {return M(self,34,uint64_t(*)(void *))(L(self));}
static int WIN us_GetMostAchievedAchievementInfo(void *self,char *name,uint32_t length,float *percent,bool *achieved) {
    return M(self,35,int(*)(void *,char *,uint32_t,float *,bool *))(L(self),name,length,percent,achieved);
}
static int WIN us_GetNextMostAchievedAchievementInfo(void *self,int previous,char *name,uint32_t length,float *percent,bool *achieved) {
    return M(self,36,int(*)(void *,int,char *,uint32_t,float *,bool *))(L(self),previous,name,length,percent,achieved);
}
static bool WIN us_GetAchievementAchievedPercent(void *self,const char *name,float *percent) {
    return M(self,37,bool(*)(void *,const char *,float *))(L(self),name,percent);
}
static uint64_t WIN us_RequestGlobalStats(void *self,int days) {return M(self,38,uint64_t(*)(void *,int))(L(self),days);}
static bool WIN us_GetGlobalStatDouble(void *self,const char *name,double *data) {
    return M(self,40,bool(*)(void *,const char *,double *))(L(self),name,data);
}
static bool WIN us_GetGlobalStatInt64(void *self,const char *name,int64_t *data) {
    return M(self,39,bool(*)(void *,const char *,int64_t *))(L(self),name,data);
}
static int32_t WIN us_GetGlobalStatHistoryDouble(void *self,const char *name,double *data,uint32_t size) {
    return M(self,42,int32_t(*)(void *,const char *,double *,uint32_t))(L(self),name,data,size);
}
static int32_t WIN us_GetGlobalStatHistoryInt64(void *self,const char *name,int64_t *data,uint32_t size) {
    return M(self,41,int32_t(*)(void *,const char *,int64_t *,uint32_t))(L(self),name,data,size);
}

/* ISteamMatchmaking009: every call crosses the Windows/SysV ABI boundary and
 * operates on the real Linux Steam object's state. No local lobby is invented. */
static int WIN mm_GetFavoriteGameCount(void *self) {return M(self,0,int(*)(void *))(L(self));}
static bool WIN mm_GetFavoriteGame(void *self,int index,uint32_t *app,uint32_t *ip,uint16_t *conn,uint16_t *query,uint32_t *flags,uint32_t *last) {
    return M(self,1,bool(*)(void *,int,uint32_t *,uint32_t *,uint16_t *,uint16_t *,uint32_t *,uint32_t *))(L(self),index,app,ip,conn,query,flags,last);
}
static int WIN mm_AddFavoriteGame(void *self,uint32_t app,uint32_t ip,uint16_t conn,uint16_t query,uint32_t flags,uint32_t last) {
    return M(self,2,int(*)(void *,uint32_t,uint32_t,uint16_t,uint16_t,uint32_t,uint32_t))(L(self),app,ip,conn,query,flags,last);
}
static bool WIN mm_RemoveFavoriteGame(void *self,uint32_t app,uint32_t ip,uint16_t conn,uint16_t query,uint32_t flags) {
    return M(self,3,bool(*)(void *,uint32_t,uint32_t,uint16_t,uint16_t,uint32_t))(L(self),app,ip,conn,query,flags);
}
static uint64_t WIN mm_RequestLobbyList(void *self) {return M(self,4,uint64_t(*)(void *))(L(self));}
static void WIN mm_AddRequestLobbyListStringFilter(void *self,const char *key,const char *value,int comparison) {
    M(self,5,void(*)(void *,const char *,const char *,int))(L(self),key,value,comparison);
}
static void WIN mm_AddRequestLobbyListNumericalFilter(void *self,const char *key,int value,int comparison) {
    M(self,6,void(*)(void *,const char *,int,int))(L(self),key,value,comparison);
}
static void WIN mm_AddRequestLobbyListNearValueFilter(void *self,const char *key,int value) {
    M(self,7,void(*)(void *,const char *,int))(L(self),key,value);
}
static void WIN mm_AddRequestLobbyListFilterSlotsAvailable(void *self,int slots) {M(self,8,void(*)(void *,int))(L(self),slots);}
static void WIN mm_AddRequestLobbyListDistanceFilter(void *self,int distance) {M(self,9,void(*)(void *,int))(L(self),distance);}
static void WIN mm_AddRequestLobbyListResultCountFilter(void *self,int count) {M(self,10,void(*)(void *,int))(L(self),count);}
static void WIN mm_AddRequestLobbyListCompatibleMembersFilter(void *self,uint64_t lobby) {M(self,11,void(*)(void *,uint64_t))(L(self),lobby);}
static uint64_t *WIN mm_GetLobbyByIndex(void *self,uint64_t *result,int index) {
    *result=M(self,12,uint64_t(*)(void *,int))(L(self),index);return result;
}
static uint64_t WIN mm_CreateLobby(void *self,int type,int members) {return M(self,13,uint64_t(*)(void *,int,int))(L(self),type,members);}
static uint64_t WIN mm_JoinLobby(void *self,uint64_t lobby) {return M(self,14,uint64_t(*)(void *,uint64_t))(L(self),lobby);}
static void WIN mm_LeaveLobby(void *self,uint64_t lobby) {M(self,15,void(*)(void *,uint64_t))(L(self),lobby);}
static bool WIN mm_InviteUserToLobby(void *self,uint64_t lobby,uint64_t user) {return M(self,16,bool(*)(void *,uint64_t,uint64_t))(L(self),lobby,user);}
static int WIN mm_GetNumLobbyMembers(void *self,uint64_t lobby) {return M(self,17,int(*)(void *,uint64_t))(L(self),lobby);}
static uint64_t *WIN mm_GetLobbyMemberByIndex(void *self,uint64_t *result,uint64_t lobby,int index) {
    *result=M(self,18,uint64_t(*)(void *,uint64_t,int))(L(self),lobby,index);return result;
}
static const char *WIN mm_GetLobbyData(void *self,uint64_t lobby,const char *key) {return M(self,19,const char *(*)(void *,uint64_t,const char *))(L(self),lobby,key);}
static bool WIN mm_SetLobbyData(void *self,uint64_t lobby,const char *key,const char *value) {return M(self,20,bool(*)(void *,uint64_t,const char *,const char *))(L(self),lobby,key,value);}
static int WIN mm_GetLobbyDataCount(void *self,uint64_t lobby) {return M(self,21,int(*)(void *,uint64_t))(L(self),lobby);}
static bool WIN mm_GetLobbyDataByIndex(void *self,uint64_t lobby,int index,char *key,int key_size,char *value,int value_size) {
    return M(self,22,bool(*)(void *,uint64_t,int,char *,int,char *,int))(L(self),lobby,index,key,key_size,value,value_size);
}
static bool WIN mm_DeleteLobbyData(void *self,uint64_t lobby,const char *key) {return M(self,23,bool(*)(void *,uint64_t,const char *))(L(self),lobby,key);}
static const char *WIN mm_GetLobbyMemberData(void *self,uint64_t lobby,uint64_t member,const char *key) {
    return M(self,24,const char *(*)(void *,uint64_t,uint64_t,const char *))(L(self),lobby,member,key);
}
static void WIN mm_SetLobbyMemberData(void *self,uint64_t lobby,const char *key,const char *value) {
    M(self,25,void(*)(void *,uint64_t,const char *,const char *))(L(self),lobby,key,value);
}
static bool WIN mm_SendLobbyChatMsg(void *self,uint64_t lobby,const void *body,int size) {
    return M(self,26,bool(*)(void *,uint64_t,const void *,int))(L(self),lobby,body,size);
}
static int WIN mm_GetLobbyChatEntry(void *self,uint64_t lobby,int chat,uint64_t *user,void *data,int size,int *type) {
    return M(self,27,int(*)(void *,uint64_t,int,uint64_t *,void *,int,int *))(L(self),lobby,chat,user,data,size,type);
}
static bool WIN mm_RequestLobbyData(void *self,uint64_t lobby) {return M(self,28,bool(*)(void *,uint64_t))(L(self),lobby);}
static void WIN mm_SetLobbyGameServer(void *self,uint64_t lobby,uint32_t ip,uint16_t port,uint64_t server) {
    M(self,29,void(*)(void *,uint64_t,uint32_t,uint16_t,uint64_t))(L(self),lobby,ip,port,server);
}
static bool WIN mm_GetLobbyGameServer(void *self,uint64_t lobby,uint32_t *ip,uint16_t *port,uint64_t *server) {
    return M(self,30,bool(*)(void *,uint64_t,uint32_t *,uint16_t *,uint64_t *))(L(self),lobby,ip,port,server);
}
static bool WIN mm_SetLobbyMemberLimit(void *self,uint64_t lobby,int limit) {return M(self,31,bool(*)(void *,uint64_t,int))(L(self),lobby,limit);}
static int WIN mm_GetLobbyMemberLimit(void *self,uint64_t lobby) {return M(self,32,int(*)(void *,uint64_t))(L(self),lobby);}
static bool WIN mm_SetLobbyType(void *self,uint64_t lobby,int type) {return M(self,33,bool(*)(void *,uint64_t,int))(L(self),lobby,type);}
static bool WIN mm_SetLobbyJoinable(void *self,uint64_t lobby,bool joinable) {return M(self,34,bool(*)(void *,uint64_t,bool))(L(self),lobby,joinable);}
static uint64_t *WIN mm_GetLobbyOwner(void *self,uint64_t *result,uint64_t lobby) {
    *result=M(self,35,uint64_t(*)(void *,uint64_t))(L(self),lobby);return result;
}
static bool WIN mm_SetLobbyOwner(void *self,uint64_t lobby,uint64_t owner) {return M(self,36,bool(*)(void *,uint64_t,uint64_t))(L(self),lobby,owner);}
static bool WIN mm_SetLinkedLobby(void *self,uint64_t lobby,uint64_t dependent) {return M(self,37,bool(*)(void *,uint64_t,uint64_t))(L(self),lobby,dependent);}

/* ISteamNetworkingSockets012: these slots are shared by the Windows and Linux SDK. */
static int WIN sockets_InitAuthentication(void *self) {
    int result=M(self,22,int(*)(void *))(L(self));
    if(trace_enabled())log_msg("trace ISteamNetworkingSockets012::InitAuthentication -> %d",result);
    return result;
}
static int WIN sockets_GetAuthenticationStatus(void *self,void *details) {
    int result=M(self,23,int(*)(void *,void *))(L(self),details);
    if(trace_enabled())log_msg("trace ISteamNetworkingSockets012::GetAuthenticationStatus -> %d",result);
    return result;
}

typedef void (WIN *WinNetworkingDebugHook)(int,const char *);
static WinNetworkingDebugHook networking_debug_hook;
static void lin_networking_debug_hook(int level,const char *message) {
    WinNetworkingDebugHook hook=networking_debug_hook;
    if(hook)hook(level,message);
}
static void WIN networking_utils_SetDebugOutputFunction(void *self,int level,WinNetworkingDebugHook hook) {
    networking_debug_hook=hook;
    M(self,13,void(*)(void *,int,void(*)(int,const char *)))(
        L(self),level,hook?lin_networking_debug_hook:NULL);
    if(trace_enabled())log_msg("trace ISteamNetworkingUtils004::SetDebugOutputFunction(level=%d, enabled=%d)",level,hook!=NULL);
}

/* ISteamNetworking006 */
static bool WIN net_SendP2PPacket(void *self,uint64_t remote,const void *data,uint32_t size,int type,int channel) {
    return M(self,0,bool(*)(void *,uint64_t,const void *,uint32_t,int,int))(L(self),remote,data,size,type,channel);
}
static bool WIN net_IsP2PPacketAvailable(void *self,uint32_t *size,int channel) {
    return M(self,1,bool(*)(void *,uint32_t *,int))(L(self),size,channel);
}
static bool WIN net_ReadP2PPacket(void *self,void *dest,uint32_t max_size,uint32_t *msg_size,uint64_t *remote,int channel) {
    return M(self,2,bool(*)(void *,void *,uint32_t,uint32_t *,uint64_t *,int))(L(self),dest,max_size,msg_size,remote,channel);
}
static bool WIN net_AcceptP2PSessionWithUser(void *self,uint64_t remote) {
    return M(self,3,bool(*)(void *,uint64_t))(L(self),remote);
}
static bool WIN net_CloseP2PSessionWithUser(void *self,uint64_t remote) {
    return M(self,4,bool(*)(void *,uint64_t))(L(self),remote);
}
static bool WIN net_CloseP2PChannelWithUser(void *self,uint64_t remote,int channel) {
    return M(self,5,bool(*)(void *,uint64_t,int))(L(self),remote,channel);
}
static bool WIN net_GetP2PSessionState(void *self,uint64_t remote,void *state) {
    return M(self,6,bool(*)(void *,uint64_t,void *))(L(self),remote,state);
}
static bool WIN net_AllowP2PPacketRelay(void *self,bool allow) {
    return M(self,7,bool(*)(void *,bool))(L(self),allow);
}
static uint32_t WIN net_CreateListenSocket(void *self,int port,void *ip,uint16_t nport,bool relay) {
    return M(self,8,uint32_t(*)(void *,int,void *,uint16_t,bool))(L(self),port,ip,nport,relay);
}
static uint32_t WIN net_CreateP2PConnectionSocket(void *self,uint64_t target,int port,int timeout,bool relay) {
    return M(self,9,uint32_t(*)(void *,uint64_t,int,int,bool))(L(self),target,port,timeout,relay);
}
static uint32_t WIN net_CreateConnectionSocket(void *self,void *ip,uint16_t port,int timeout) {
    return M(self,10,uint32_t(*)(void *,void *,uint16_t,int))(L(self),ip,port,timeout);
}
static bool WIN net_DestroySocket(void *self,uint32_t socket,bool notify) {
    return M(self,11,bool(*)(void *,uint32_t,bool))(L(self),socket,notify);
}
static bool WIN net_DestroyListenSocket(void *self,uint32_t socket,bool notify) {
    return M(self,12,bool(*)(void *,uint32_t,bool))(L(self),socket,notify);
}
static bool WIN net_SendDataOnSocket(void *self,uint32_t socket,void *data,uint32_t size,bool reliable) {
    return M(self,13,bool(*)(void *,uint32_t,void *,uint32_t,bool))(L(self),socket,data,size,reliable);
}
static bool WIN net_IsDataAvailableOnSocket(void *self,uint32_t socket,uint32_t *size) {
    return M(self,14,bool(*)(void *,uint32_t,uint32_t *))(L(self),socket,size);
}
static bool WIN net_RetrieveDataFromSocket(void *self,uint32_t socket,void *dest,uint32_t max_size,uint32_t *msg_size) {
    return M(self,15,bool(*)(void *,uint32_t,void *,uint32_t,uint32_t *))(L(self),socket,dest,max_size,msg_size);
}
static bool WIN net_IsDataAvailable(void *self,uint32_t listen_socket,uint32_t *size,uint32_t *socket) {
    return M(self,16,bool(*)(void *,uint32_t,uint32_t *,uint32_t *))(L(self),listen_socket,size,socket);
}
static bool WIN net_RetrieveData(void *self,uint32_t listen_socket,void *dest,uint32_t max_size,uint32_t *msg_size,uint32_t *socket) {
    return M(self,17,bool(*)(void *,uint32_t,void *,uint32_t,uint32_t *,uint32_t *))(L(self),listen_socket,dest,max_size,msg_size,socket);
}
static bool WIN net_GetSocketInfo(void *self,uint32_t socket,uint64_t *remote,int *status,void *ip,uint16_t *port) {
    return M(self,18,bool(*)(void *,uint32_t,uint64_t *,int *,void *,uint16_t *))(L(self),socket,remote,status,ip,port);
}
static bool WIN net_GetListenSocketInfo(void *self,uint32_t listen_socket,void *ip,uint16_t *port) {
    return M(self,19,bool(*)(void *,uint32_t,void *,uint16_t *))(L(self),listen_socket,ip,port);
}
static int WIN net_GetSocketConnectionType(void *self,uint32_t socket) {
    return M(self,20,int(*)(void *,uint32_t))(L(self),socket);
}
static int WIN net_GetMaxPacketSize(void *self,uint32_t socket) {
    return M(self,21,int(*)(void *,uint32_t))(L(self),socket);
}

/* ISteamAppTicket001 */
static uint32_t WIN appticket_GetAppOwnershipTicketData(void *self,uint32_t app_id,void *buffer,uint32_t size,uint32_t *piTicketLength,uint32_t *piAppId,uint32_t *piSteamId,uint32_t *piSignature) {
    if(trace_enabled())log_msg("trace ISteamAppTicket::GetAppOwnershipTicketData(app=%u, size=%u)",app_id,size);
    uint32_t result=M(self,0,uint32_t(*)(void *,uint32_t,void *,uint32_t,uint32_t *,uint32_t *,uint32_t *,uint32_t *))(
        L(self),app_id,buffer,size,piTicketLength,piAppId,piSteamId,piSignature);
    if(trace_enabled())log_msg("trace ISteamAppTicket::GetAppOwnershipTicketData -> %u",result);
    return result;
}

static void init_vtables(void) {
    for(int k=0;k<KINDS;k++)for(int s=0;s<MAX_SLOTS;s++)vtables[k][s]=traps[k][s];
    void **c=vtables[CLIENT],**u=vtables[USER],**t=vtables[UTILS],**f=vtables[FRIENDS],**ct=vtables[CONTROLLER],**ap=vtables[APPS],**mm=vtables[MATCHMAKING],**at=vtables[APPTICKET],**sockets=vtables[NETWORKING_SOCKETS],**net_utils=vtables[NETWORKING_UTILS],**net=vtables[NETWORKING];
    at[0]=(void *)appticket_GetAppOwnershipTicketData;
    sockets[22]=(void *)sockets_InitAuthentication;
    sockets[23]=(void *)sockets_GetAuthenticationStatus;
    net_utils[13]=(void *)networking_utils_SetDebugOutputFunction;
    net[0]=(void *)net_SendP2PPacket;net[1]=(void *)net_IsP2PPacketAvailable;
    net[2]=(void *)net_ReadP2PPacket;net[3]=(void *)net_AcceptP2PSessionWithUser;
    net[4]=(void *)net_CloseP2PSessionWithUser;net[5]=(void *)net_CloseP2PChannelWithUser;
    net[6]=(void *)net_GetP2PSessionState;net[7]=(void *)net_AllowP2PPacketRelay;
    net[8]=(void *)net_CreateListenSocket;net[9]=(void *)net_CreateP2PConnectionSocket;
    net[10]=(void *)net_CreateConnectionSocket;net[11]=(void *)net_DestroySocket;
    net[12]=(void *)net_DestroyListenSocket;net[13]=(void *)net_SendDataOnSocket;
    net[14]=(void *)net_IsDataAvailableOnSocket;net[15]=(void *)net_RetrieveDataFromSocket;
    net[16]=(void *)net_IsDataAvailable;net[17]=(void *)net_RetrieveData;
    net[18]=(void *)net_GetSocketInfo;net[19]=(void *)net_GetListenSocketInfo;
    net[20]=(void *)net_GetSocketConnectionType;net[21]=(void *)net_GetMaxPacketSize;
    c[0]=(void *)client_CreateSteamPipe;c[1]=(void *)client_BReleaseSteamPipe;c[2]=(void *)client_ConnectToGlobalUser;
    c[3]=(void *)client_CreateLocalUser;c[4]=(void *)client_ReleaseUser;c[5]=(void *)client_GetISteamUser;
    c[7]=(void *)client_SetLocalIPBinding;c[8]=(void *)client_GetISteamFriends;c[9]=(void *)client_GetISteamUtils;
    c[10]=(void *)client_GetISteamMatchmaking;
    c[12]=(void *)client_GetISteamGenericInterface;c[15]=(void *)client_GetISteamApps;c[26]=(void *)client_GetISteamController;
    c[13]=(void *)client_GetISteamUserStats;c[17]=(void *)client_GetISteamRemoteStorage;
    c[16]=(void *)client_GetISteamNetworking;
    c[38]=(void *)client_GetISteamInput;
    static const int getters[]={6,11,14,18,19,24,25,27,28,29,30,31,35,36,37,39,40};
    for(size_t i=0;i<sizeof getters/sizeof *getters;i++)c[getters[i]]=(void *)client_unbridged_getter;
    c[20]=(void *)client_RunFrame;c[21]=(void *)client_GetIPCCallCount;c[22]=(void *)client_SetWarningMessageHook;
    c[23]=(void *)client_BShutdownIfAllPipesClosed;c[34]=(void *)client_SetCheckCallbackRegistered;
    c[41]=(void *)client_DestroyAllInterfaces;
    mm[0]=(void *)mm_GetFavoriteGameCount;mm[1]=(void *)mm_GetFavoriteGame;
    mm[2]=(void *)mm_AddFavoriteGame;mm[3]=(void *)mm_RemoveFavoriteGame;
    mm[4]=(void *)mm_RequestLobbyList;mm[5]=(void *)mm_AddRequestLobbyListStringFilter;
    mm[6]=(void *)mm_AddRequestLobbyListNumericalFilter;mm[7]=(void *)mm_AddRequestLobbyListNearValueFilter;
    mm[8]=(void *)mm_AddRequestLobbyListFilterSlotsAvailable;mm[9]=(void *)mm_AddRequestLobbyListDistanceFilter;
    mm[10]=(void *)mm_AddRequestLobbyListResultCountFilter;mm[11]=(void *)mm_AddRequestLobbyListCompatibleMembersFilter;
    mm[12]=(void *)mm_GetLobbyByIndex;mm[13]=(void *)mm_CreateLobby;mm[14]=(void *)mm_JoinLobby;
    mm[15]=(void *)mm_LeaveLobby;mm[16]=(void *)mm_InviteUserToLobby;mm[17]=(void *)mm_GetNumLobbyMembers;
    mm[18]=(void *)mm_GetLobbyMemberByIndex;mm[19]=(void *)mm_GetLobbyData;mm[20]=(void *)mm_SetLobbyData;
    mm[21]=(void *)mm_GetLobbyDataCount;mm[22]=(void *)mm_GetLobbyDataByIndex;mm[23]=(void *)mm_DeleteLobbyData;
    mm[24]=(void *)mm_GetLobbyMemberData;mm[25]=(void *)mm_SetLobbyMemberData;
    mm[26]=(void *)mm_SendLobbyChatMsg;mm[27]=(void *)mm_GetLobbyChatEntry;
    mm[28]=(void *)mm_RequestLobbyData;mm[29]=(void *)mm_SetLobbyGameServer;
    mm[30]=(void *)mm_GetLobbyGameServer;mm[31]=(void *)mm_SetLobbyMemberLimit;
    mm[32]=(void *)mm_GetLobbyMemberLimit;mm[33]=(void *)mm_SetLobbyType;
    mm[34]=(void *)mm_SetLobbyJoinable;mm[35]=(void *)mm_GetLobbyOwner;
    mm[36]=(void *)mm_SetLobbyOwner;mm[37]=(void *)mm_SetLinkedLobby;
    void **c19=vtables[CLIENT019];
    for(int s=0;s<slot_counts[CLIENT019];s++) {
        c19[s]=c[s]==traps[CLIENT][s]?traps[CLIENT019][s]:c[s];
    }
    c19[7]=(void *)client017_SetLocalIPBinding;
    void **c18=vtables[CLIENT018];
    for(int s=0;s<slot_counts[CLIENT018];s++) {
        c18[s]=c[s]==traps[CLIENT][s]?traps[CLIENT018][s]:c[s];
    }
    c18[7]=(void *)client017_SetLocalIPBinding;
    void **c17=vtables[CLIENT017];
    for(int s=0;s<slot_counts[CLIENT017];s++) {
        int slot020=s<19?s:s+1;
        c17[s]=c[slot020]==traps[CLIENT][slot020]?traps[CLIENT017][s]:c[slot020];
    }
    c17[7]=(void *)client017_SetLocalIPBinding;
    void **c14=vtables[CLIENT014]; /* Proton: 014 thunk bodies equal 017's for every shared slot */
    for(int s=0;s<slot_counts[CLIENT014];s++)c14[s]=c17[s]==traps[CLIENT017][s]?traps[CLIENT014][s]:c17[s];
    u[0]=(void *)user_GetHSteamUser;u[1]=(void *)user_BLoggedOn;u[2]=(void *)user_GetSteamID;
    u[5]=(void *)user_TrackAppUsageEvent;u[6]=(void *)user_GetUserDataFolder;u[13]=(void *)user_GetAuthSessionTicket;
    u[14]=(void *)user_BeginAuthSession;u[15]=(void *)user_EndAuthSession;u[16]=(void *)user_CancelAuthTicket;
    u[17]=(void *)user_UserHasLicenseForApp;u[18]=(void *)user_BIsBehindNAT;u[20]=(void *)user_RequestEncryptedAppTicket;
    u[21]=(void *)user_GetEncryptedAppTicket;u[22]=(void *)user_GetGameBadgeLevel;u[23]=(void *)user_GetPlayerSteamLevel;
    u[25]=(void *)user_BIsPhoneVerified;u[26]=(void *)user_BIsTwoFactorEnabled;u[27]=(void *)user_BIsPhoneIdentifying;
    u[28]=(void *)user_BIsPhoneRequiringVerification;
    t[0]=(void *)utils_GetSecondsSinceAppActive;t[1]=(void *)utils_GetSecondsSinceComputerActive;
    t[2]=(void *)utils_GetConnectedUniverse;t[3]=(void *)utils_GetServerRealTime;t[4]=(void *)utils_GetIPCountry;
    t[5]=(void *)utils_GetImageSize;t[6]=(void *)utils_GetImageRGBA;t[7]=(void *)utils_GetCSERIPPort;
    t[8]=(void *)utils_GetCurrentBatteryPower;t[9]=(void *)utils_GetAppID;t[10]=(void *)utils_SetOverlayNotificationPosition;
    t[11]=(void *)utils_IsAPICallCompleted;t[12]=(void *)utils_GetAPICallFailureReason;t[13]=(void *)utils_GetAPICallResult;
    t[14]=(void *)utils_RunFrame;t[15]=(void *)utils_GetIPCCallCount;t[16]=(void *)utils_SetWarningMessageHook;
    t[17]=(void *)utils_IsOverlayEnabled;t[18]=(void *)utils_BOverlayNeedsPresent;t[23]=(void *)utils_GetSteamUILanguage;
    t[24]=(void *)utils_IsSteamRunningInVR;t[25]=(void *)utils_SetOverlayNotificationInset;
    t[26]=(void *)utils_IsSteamInBigPictureMode;t[30]=(void *)utils_IsSteamChinaLauncher;
    t[34]=(void *)utils_IsSteamRunningOnSteamDeck;
    void **u20=vtables[USER020];
    for(int s=0;s<slot_counts[USER020];s++) {
        u20[s]=u[s]==traps[USER][s]?traps[USER020][s]:u[s];
    }
    void **u19=vtables[USER019];
    for(int s=0;s<slot_counts[USER019];s++) {
        u19[s]=u[s]==traps[USER][s]?traps[USER019][s]:u[s];
    }
    void **u18=vtables[USER018];
    for(int s=0;s<slot_counts[USER018];s++) {
        u18[s]=u[s]==traps[USER][s]?traps[USER018][s]:u[s];
    }
    void **u17=vtables[USER017];
    for(int s=0;s<slot_counts[USER017];s++) {
        u17[s]=u[s]==traps[USER][s]?traps[USER017][s]:u[s];
    }
    void **t009=vtables[UTILS009];
    for(int s=0;s<slot_counts[UTILS009];s++) {
        t009[s]=t[s]==traps[UTILS][s]?traps[UTILS009][s]:t[s];
    }
    void **t008=vtables[UTILS008];
    for(int s=0;s<slot_counts[UTILS008];s++) {
        t008[s]=t[s]==traps[UTILS][s]?traps[UTILS008][s]:t[s];
    }
    void **t007=vtables[UTILS007];
    for(int s=0;s<slot_counts[UTILS007];s++) {
        t007[s]=t[s]==traps[UTILS][s]?traps[UTILS007][s]:t[s];
    }
    f[0]=(void *)friends_GetPersonaName;f[1]=(void *)friends_SetPersonaName;f[2]=(void *)friends_GetPersonaState;
    f[3]=(void *)friends_GetFriendCount;f[4]=(void *)friends_GetFriendByIndex;f[5]=(void *)friends_GetFriendRelationship;
    f[6]=(void *)friends_GetFriendPersonaState;f[7]=(void *)friends_GetFriendPersonaName;f[8]=(void *)friends_GetFriendGamePlayed;
    f[9]=(void *)friends_GetFriendPersonaNameHistory;f[10]=(void *)friends_GetFriendSteamLevel;f[11]=(void *)friends_GetPlayerNickname;
    f[12]=(void *)friends_GetFriendsGroupCount;f[13]=(void *)friends_GetFriendsGroupIDByIndex;f[14]=(void *)friends_GetFriendsGroupName;
    f[15]=(void *)friends_GetFriendsGroupMembersCount;f[16]=(void *)friends_GetFriendsGroupMembersList;f[17]=(void *)friends_HasFriend;
    f[18]=(void *)friends_GetClanCount;f[19]=(void *)friends_GetClanByIndex;f[20]=(void *)friends_GetClanName;
    f[21]=(void *)friends_GetClanTag;f[22]=(void *)friends_GetClanActivityCounts;f[23]=(void *)friends_DownloadClanActivityCounts;
    f[24]=(void *)friends_GetFriendCountFromSource;f[25]=(void *)friends_GetFriendFromSourceByIndex;f[26]=(void *)friends_IsUserInSource;
    f[27]=(void *)friends_SetInGameVoiceSpeaking;f[28]=(void *)friends_ActivateGameOverlay;f[29]=(void *)friends_ActivateGameOverlayToUser;
    f[30]=(void *)friends_ActivateGameOverlayToWebPage;f[31]=(void *)friends_ActivateGameOverlayToStore;f[32]=(void *)friends_SetPlayedWith;
    f[33]=(void *)friends_ActivateGameOverlayInviteDialog;f[34]=(void *)friends_GetSmallFriendAvatar;f[35]=(void *)friends_GetMediumFriendAvatar;
    f[36]=(void *)friends_GetLargeFriendAvatar;f[37]=(void *)friends_RequestUserInformation;f[38]=(void *)friends_RequestClanOfficerList;
    f[39]=(void *)friends_GetClanOwner;f[40]=(void *)friends_GetClanOfficerCount;f[41]=(void *)friends_GetClanOfficerByIndex;
    f[42]=(void *)friends_GetUserRestrictions;f[43]=(void *)friends_SetRichPresence;f[44]=(void *)friends_ClearRichPresence;
    f[45]=(void *)friends_GetFriendRichPresence;f[46]=(void *)friends_GetFriendRichPresenceKeyCount;f[47]=(void *)friends_GetFriendRichPresenceKeyByIndex;
    f[48]=(void *)friends_RequestFriendRichPresence;f[49]=(void *)friends_InviteUserToGame;f[50]=(void *)friends_GetCoplayFriendCount;
    f[51]=(void *)friends_GetCoplayFriend;f[52]=(void *)friends_GetFriendCoplayTime;f[53]=(void *)friends_GetFriendCoplayGame;
    f[54]=(void *)friends_JoinClanChatRoom;f[55]=(void *)friends_LeaveClanChatRoom;f[56]=(void *)friends_GetClanChatMemberCount;
    f[57]=(void *)friends_GetChatMemberByIndex;f[58]=(void *)friends_SendClanChatMessage;f[59]=(void *)friends_GetClanChatMessage;
    f[60]=(void *)friends_IsClanChatAdmin;f[61]=(void *)friends_IsClanChatWindowOpenInSteam;f[62]=(void *)friends_OpenClanChatWindowInSteam;
    f[63]=(void *)friends_CloseClanChatWindowInSteam;f[64]=(void *)friends_SetListenForFriendsMessages;f[65]=(void *)friends_ReplyToFriendMessage;
    f[66]=(void *)friends_GetFriendMessage;f[67]=(void *)friends_GetFollowerCount;f[68]=(void *)friends_IsFollowing;
    f[69]=(void *)friends_EnumerateFollowingList;f[70]=(void *)friends_IsClanPublic;f[71]=(void *)friends_IsClanOfficialGameGroup;
    f[72]=(void *)friends_GetNumChatsWithUnreadPriorityMessages;f[73]=(void *)friends_ActivateGameOverlayRemotePlayTogetherInviteDialog;
    f[74]=(void *)friends_RegisterProtocolInOverlayBrowser;f[75]=(void *)friends_ActivateGameOverlayInviteDialogConnectString;
    f[76]=(void *)friends_RequestEquippedProfileItems;f[77]=(void *)friends_BHasEquippedProfileItem;
    f[78]=(void *)friends_GetProfileItemPropertyString;f[79]=(void *)friends_GetProfileItemPropertyUint;
    ct[0]=(void *)controller_Init;ct[1]=(void *)controller_Shutdown;ct[2]=(void *)controller_RunFrame;
    ct[3]=(void *)controller_GetConnectedControllers;ct[4]=(void *)controller_GetActionSetHandle;
    ct[5]=(void *)controller_ActivateActionSet;ct[6]=(void *)controller_GetCurrentActionSet;
    ct[7]=(void *)controller_ActivateActionSetLayer;ct[8]=(void *)controller_DeactivateActionSetLayer;
    ct[9]=(void *)controller_DeactivateAllActionSetLayers;ct[10]=(void *)controller_GetActiveActionSetLayers;
    ct[11]=(void *)controller_GetDigitalActionHandle;ct[12]=(void *)controller_GetDigitalActionData;
    ct[13]=(void *)controller_GetDigitalActionOrigins;ct[14]=(void *)controller_GetAnalogActionHandle;
    ct[15]=(void *)controller_GetAnalogActionData;ct[16]=(void *)controller_GetAnalogActionOrigins;
    ct[17]=(void *)controller_GetGlyphForActionOrigin;ct[18]=(void *)controller_GetStringForActionOrigin;
    ct[19]=(void *)controller_StopAnalogActionMomentum;ct[20]=(void *)controller_GetMotionData;
    ct[21]=(void *)controller_TriggerHapticPulse;ct[22]=(void *)controller_TriggerRepeatedHapticPulse;
    ct[23]=(void *)controller_TriggerVibration;ct[24]=(void *)controller_SetLEDColor;
    ct[25]=(void *)controller_ShowBindingPanel;ct[26]=(void *)controller_GetInputTypeForHandle;
    ct[27]=(void *)controller_GetControllerForGamepadIndex;ct[28]=(void *)controller_GetGamepadIndexForController;
    ct[29]=(void *)controller_GetStringForXboxOrigin;ct[30]=(void *)controller_GetGlyphForXboxOrigin;
    ct[31]=(void *)controller_GetActionOriginFromXboxOrigin;ct[32]=(void *)controller_TranslateActionOrigin;
    ct[33]=(void *)controller_GetControllerBindingRevision;
    ap[0]=(void *)apps_BIsSubscribed;ap[1]=(void *)apps_BIsLowViolence;ap[2]=(void *)apps_BIsCybercafe;
    ap[3]=(void *)apps_BIsVACBanned;ap[4]=(void *)apps_GetCurrentGameLanguage;ap[5]=(void *)apps_GetAvailableGameLanguages;
    ap[6]=(void *)apps_BIsSubscribedApp;ap[7]=(void *)apps_BIsDlcInstalled;ap[8]=(void *)apps_GetEarliestPurchaseUnixTime;
    ap[9]=(void *)apps_BIsSubscribedFromFreeWeekend;ap[10]=(void *)apps_GetDLCCount;ap[11]=(void *)apps_BGetDLCDataByIndex;
    ap[12]=(void *)apps_InstallDLC;ap[13]=(void *)apps_UninstallDLC;ap[14]=(void *)apps_RequestAppProofOfPurchaseKey;
    ap[15]=(void *)apps_GetCurrentBetaName;ap[16]=(void *)apps_MarkContentCorrupt;ap[17]=(void *)apps_GetInstalledDepots;
    ap[18]=(void *)apps_GetAppInstallDir;ap[19]=(void *)apps_BIsAppInstalled;ap[20]=(void *)apps_GetAppOwner;
    ap[21]=(void *)apps_GetLaunchQueryParam;ap[22]=(void *)apps_GetDlcDownloadProgress;ap[23]=(void *)apps_GetAppBuildId;
    ap[24]=(void *)apps_RequestAllProofOfPurchaseKeys;ap[25]=(void *)apps_GetFileDetails;ap[26]=(void *)apps_GetLaunchCommandLine;
    ap[27]=(void *)apps_BIsSubscribedFromFamilySharing;ap[28]=(void *)apps_BIsTimedTrial;ap[29]=(void *)apps_SetDlcContext;
    ap[30]=(void *)apps_GetNumBetas;ap[31]=(void *)apps_GetBetaInfo;ap[32]=(void *)apps_SetActiveBeta;
    ap[33]=(void *)apps_SetGamePerformanceSetting;ap[34]=(void *)apps_SetGameRenderResolution;
    void **rs=vtables[REMOTESTORAGE];
    rs[0]=(void *)rs_FileWrite;rs[1]=(void *)rs_FileRead;rs[2]=(void *)rs_FileWriteAsync;rs[3]=(void *)rs_FileReadAsync;
    rs[4]=(void *)rs_FileReadAsyncComplete;rs[5]=(void *)rs_FileForget;rs[6]=(void *)rs_FileDelete;rs[7]=(void *)rs_FileShare;
    rs[8]=(void *)rs_SetSyncPlatforms;rs[9]=(void *)rs_FileWriteStreamOpen;rs[10]=(void *)rs_FileWriteStreamWriteChunk;
    rs[11]=(void *)rs_FileWriteStreamClose;rs[12]=(void *)rs_FileWriteStreamCancel;rs[13]=(void *)rs_FileExists;
    rs[14]=(void *)rs_FilePersisted;rs[15]=(void *)rs_GetFileSize;rs[16]=(void *)rs_GetFileTimestamp;
    rs[17]=(void *)rs_GetSyncPlatforms;rs[18]=(void *)rs_GetFileCount;rs[19]=(void *)rs_GetFileNameAndSize;
    rs[20]=(void *)rs_GetQuota;rs[21]=(void *)rs_IsCloudEnabledForAccount;rs[22]=(void *)rs_IsCloudEnabledForApp;
    rs[23]=(void *)rs_SetCloudEnabledForApp;
    void **us=vtables[USERSTATS];
    us[0]=(void *)us_RequestCurrentStats;us[1]=(void *)us_GetStatFloat;us[2]=(void *)us_GetStatInt;
    us[3]=(void *)us_SetStatFloat;us[4]=(void *)us_SetStatInt;us[5]=(void *)us_UpdateAvgRateStat;
    us[6]=(void *)us_GetAchievement;us[7]=(void *)us_SetAchievement;us[8]=(void *)us_ClearAchievement;
    us[9]=(void *)us_GetAchievementAndUnlockTime;us[10]=(void *)us_StoreStats;us[11]=(void *)us_GetAchievementIcon;
    us[12]=(void *)us_GetAchievementDisplayAttribute;us[13]=(void *)us_IndicateAchievementProgress;
    us[14]=(void *)us_GetNumAchievements;us[15]=(void *)us_GetAchievementName;us[16]=(void *)us_RequestUserStats;
    us[17]=(void *)us_GetUserStatFloat;us[18]=(void *)us_GetUserStatInt;us[19]=(void *)us_GetUserAchievement;
    us[20]=(void *)us_GetUserAchievementAndUnlockTime;us[21]=(void *)us_ResetAllStats;
    us[22]=(void *)us_FindOrCreateLeaderboard;us[23]=(void *)us_FindLeaderboard;us[24]=(void *)us_GetLeaderboardName;
    us[25]=(void *)us_GetLeaderboardEntryCount;us[26]=(void *)us_GetLeaderboardSortMethod;
    us[27]=(void *)us_GetLeaderboardDisplayType;us[28]=(void *)us_DownloadLeaderboardEntries;
    us[29]=(void *)us_DownloadLeaderboardEntriesForUsers;us[31]=(void *)us_UploadLeaderboardScore;
    us[32]=(void *)us_AttachLeaderboardUGC;us[33]=(void *)us_GetNumberOfCurrentPlayers;
    us[34]=(void *)us_RequestGlobalAchievementPercentages;us[35]=(void *)us_GetMostAchievedAchievementInfo;
    us[36]=(void *)us_GetNextMostAchievedAchievementInfo;us[37]=(void *)us_GetAchievementAchievedPercent;
    us[38]=(void *)us_RequestGlobalStats;us[39]=(void *)us_GetGlobalStatDouble;us[40]=(void *)us_GetGlobalStatInt64;
    us[41]=(void *)us_GetGlobalStatHistoryDouble;us[42]=(void *)us_GetGlobalStatHistoryInt64;
    for(int k=0;k<KINDS;k++)for(int s=slot_counts[k];s<MAX_SLOTS;s++)vtables[k][s]=NULL;
}

static void *WIN w_CreateInterface(const char *version,int *code) {
    if(trace_enabled()) {
        log_msg("trace factory request %s caller=%p",version?version:"NULL",__builtin_return_address(0));
        void *bt[16];
        int n = backtrace(bt, 16);
        backtrace_symbols_fd(bt, n, 2);
    }
    if(ensure_open()) {if(code)*code=1;return NULL;}
    int kind=bridged_kind(version);
    if(kind<0) {
        log_msg("CreateInterface(%s): version not bridged",version?version:"NULL");
        if(code)*code=1;
        return NULL;
    }
    void *object=lin.create_interface(version,code);
    if(!object) {
        log_msg("CreateInterface(%s): Linux client returned NULL",version);
        return NULL;
    }
    void *windows=wrap(object,(Kind)kind);
    if(trace_enabled()) {
        if(code)log_msg("trace CreateInterface(%s): Linux=%p code=%d Windows=%p",version,object,*code,windows);
        else log_msg("trace CreateInterface(%s): Linux=%p code=not requested Windows=%p",version,object,windows);
    }
    return windows;
}

void *steam_bridge_export(const char *name) {
    static const struct {const char *name;void *address;} exports[]={
        {"CreateInterface",(void *)w_CreateInterface},
        {"Steam_BGetCallback",(void *)w_BGetCallback},
        {"Steam_FreeLastCallback",(void *)w_FreeLastCallback},
        {"Steam_GetAPICallResult",(void *)w_GetAPICallResult},
        {"Steam_ReleaseThreadLocalMemory",(void *)w_ReleaseThreadLocalMemory},
        {"Breakpad_SteamMiniDumpInit",(void *)w_MiniDumpInit},
        {"Breakpad_SteamSetAppID",(void *)w_SetAppID},
        {"Breakpad_SteamSetSteamID",(void *)w_SetSteamID},
        {"Breakpad_SteamWriteMiniDumpSetComment",(void *)w_SetComment},
        {"Breakpad_SteamWriteMiniDumpUsingExceptionInfoWithBuildId",(void *)w_WriteMiniDump},
    };
    for(size_t i=0;name && i<sizeof exports/sizeof *exports;i++)if(!strcmp(name,exports[i].name))return exports[i].address;
    return NULL;
}

int steam_bridge_contains_address(const void *addr) {
    if(!addr) return 0;
    static const struct {const char *name;void *address;} exports[]={
        {"CreateInterface",(void *)w_CreateInterface},
        {"Steam_BGetCallback",(void *)w_BGetCallback},
        {"Steam_FreeLastCallback",(void *)w_FreeLastCallback},
        {"Steam_GetAPICallResult",(void *)w_GetAPICallResult},
        {"Steam_ReleaseThreadLocalMemory",(void *)w_ReleaseThreadLocalMemory},
        {"Breakpad_SteamMiniDumpInit",(void *)w_MiniDumpInit},
        {"Breakpad_SteamSetAppID",(void *)w_SetAppID},
        {"Breakpad_SteamSetSteamID",(void *)w_SetSteamID},
        {"Breakpad_SteamWriteMiniDumpSetComment",(void *)w_SetComment},
        {"Breakpad_SteamWriteMiniDumpUsingExceptionInfoWithBuildId",(void *)w_WriteMiniDump},
    };
    for(size_t i=0; i<sizeof exports/sizeof *exports; i++) {
        if(exports[i].address == addr) return 1;
    }
    for(int k=0; k<KINDS; k++) {
        for(int s=0; s<MAX_SLOTS; s++) {
            if(vtables[k][s] == addr) return 1;
        }
    }
    if(addr >= (const void *)vtables && addr < (const void *)((const uint8_t *)vtables + sizeof vtables)) return 1;
    for(Wrapper *w = wrappers; w; w = w->next) {
        if((uintptr_t)addr >= (uintptr_t)w && (uintptr_t)addr < (uintptr_t)(w + 1)) return 1;
    }
    if(lin.library) {
        Dl_info info;
        if(dladdr(addr, &info) && info.dli_fbase == lin.library) return 1;
    }
    Dl_info info;
    if(dladdr(addr, &info) && info.dli_sname) {
        const char *s = info.dli_sname;
        if(!strncmp(s, "w_", 2) || !strncmp(s, "steam_bridge", 12) ||
           !strncmp(s, "cl_", 3) || !strncmp(s, "us_", 3) ||
           !strncmp(s, "ut_", 3) || !strncmp(s, "friends_", 8) ||
           !strncmp(s, "client_", 7) || !strncmp(s, "user_", 5) ||
           !strncmp(s, "utils_", 6) || !strncmp(s, "rs_", 3) ||
           !strncmp(s, "ct_", 3) || !strncmp(s, "ap_", 3) ||
           !strncmp(s, "appticket_", 10) || !strncmp(s, "mm_", 3) || !strcmp(s, "wrap")) {
            return 1;
        }
    }
    return 0;
}
