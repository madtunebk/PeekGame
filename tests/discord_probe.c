#include <windows.h>
#include "discord_game_sdk.h"
void *memset(void *out,int c,size_t n){unsigned char *p=out;while(n--)*p++=(unsigned char)c;return out;}
static void number(const char *label,DWORD value){char out[120];DWORD n=0,written;while(*label)out[n++]=*label++;char digits[16];DWORD i=0;do{digits[i++]=(char)('0'+value%10);value/=10;}while(value);while(i)out[n++]=digits[--i];out[n++]='\n';WriteFile(GetStdHandle(STD_OUTPUT_HANDLE),out,n,&written,0);}
void entry(void){
    HMODULE dll=LoadLibraryW(DISCORD_DLL);
    if(!dll){number("LoadLibrary error: ",GetLastError());ExitProcess(1);}
    enum EDiscordResult (DISCORD_API *create)(DiscordVersion,struct DiscordCreateParams*,struct IDiscordCore**)=(void*)GetProcAddress(dll,"DiscordCreate");
    if(!create)ExitProcess(2);
    struct DiscordCreateParams params;DiscordCreateParamsSetDefault(&params);
    params.client_id=1;params.flags=DiscordCreateFlags_NoRequireDiscord;
    struct IDiscordCore *core=0;
    enum EDiscordResult result=create(DISCORD_VERSION,&params,&core);
    number("DiscordCreate result: ",result);
    if(result==DiscordResult_Ok){if(!core)ExitProcess(3);for(int i=0;i<10;i++){number("run_callbacks: ",core->run_callbacks(core));Sleep(10);}core->destroy(core);}
    Sleep(100);FreeLibrary(dll);ExitProcess(0);
}
