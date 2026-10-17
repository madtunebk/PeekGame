#include <assert.h>
#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <X11/Xlib.h>
#include <X11/extensions/Xrandr.h>
#define WIN __attribute__((ms_abi))
static int32_t(WIN *info)(uintptr_t,void*);
static int32_t(WIN *settings)(const uint16_t*,uint32_t,void*);
static XRRMonitorInfo *native;static int total,visited,ox,oy;
static int32_t WIN monitor(uintptr_t handle,uintptr_t dc,int32_t *rect,intptr_t data){
    (void)data;assert(dc==0);unsigned char value[104]={0};*(uint32_t*)value=104;
    assert(info(handle,value));assert(!memcmp(rect,value+4,16));
    int matched=0;
    for(int i=0;i<total;i++)if(rect[0]==native[i].x-ox&&rect[1]==native[i].y-oy&&rect[2]-rect[0]==native[i].width&&rect[3]-rect[1]==native[i].height){matched=1;break;}
    assert(matched);
    unsigned char mode[220]={0};*(uint16_t*)(mode+68)=220;
    assert(settings((uint16_t*)(value+40),UINT32_MAX,mode));
    assert(*(int32_t*)(mode+76)==rect[0]&&*(int32_t*)(mode+80)==rect[1]);
    assert(*(uint32_t*)(mode+172)==(uint32_t)(rect[2]-rect[0]));
    assert(*(uint32_t*)(mode+176)==(uint32_t)(rect[3]-rect[1]));visited++;return 1;
}
int main(int argc,char **argv){
    assert(argc==2);Display*d=XOpenDisplay(NULL);assert(d);
    native=XRRGetMonitors(d,DefaultRootWindow(d),True,&total);assert(native&&total>0);
    int primary=0;for(int i=0;i<total;i++)if(native[i].primary)primary=i;
    ox=native[primary].x;oy=native[primary].y;
    void *lib=dlopen(argv[1],RTLD_NOW|RTLD_LOCAL);if(!lib){puts(dlerror());return 1;}
    const void *(*resolve)(const char*,const char*)=dlsym(lib,"peekgame_resolve");assert(resolve);
    int32_t(WIN *metrics)(int32_t)=(void*)resolve("user32.dll","GetSystemMetrics");
    int32_t(WIN *enumerate)(uintptr_t,const void*,void*,intptr_t)=(void*)resolve("user32.dll","EnumDisplayMonitors");
    info=(void*)resolve("user32.dll","GetMonitorInfoW");settings=(void*)resolve("user32.dll","EnumDisplaySettingsW");assert(metrics&&enumerate&&info&&settings);
    assert(metrics(0)==native[primary].width&&metrics(1)==native[primary].height);
    assert(metrics(80)==total);assert(enumerate(0,NULL,monitor,0));assert(visited==total);
    int left=native[0].x,top=native[0].y,right=left+native[0].width,bottom=top+native[0].height;
    for(int i=1;i<total;i++){if(native[i].x<left)left=native[i].x;if(native[i].y<top)top=native[i].y;if(native[i].x+native[i].width>right)right=native[i].x+native[i].width;if(native[i].y+native[i].height>bottom)bottom=native[i].y+native[i].height;}
    assert(metrics(76)==left-ox&&metrics(77)==top-oy&&metrics(78)==right-left&&metrics(79)==bottom-top);
    unsigned char mode[220]={0};*(uint16_t*)(mode+68)=220;assert(settings(NULL,UINT32_MAX,mode));
    assert(*(uint32_t*)(mode+172)==(uint32_t)native[primary].width);
    assert(*(uint32_t*)(mode+176)==(uint32_t)native[primary].height);
    printf("PASS: %d monitor(s), primary %dx%d, virtual %dx%d, Windows origin (%d,%d)\n",total,metrics(0),metrics(1),metrics(78),metrics(79),metrics(76),metrics(77));
    XRRFreeMonitors(native);XCloseDisplay(d);return 0;
}
