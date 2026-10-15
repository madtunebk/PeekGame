#include "window_abi.h"
#include <dlfcn.h>
#include <stdio.h>
#include <string.h>
uint32_t (WIN *last_error)(void);
uint16_t (WIN *register_class)(const struct Class *);
uintptr_t (WIN *create_window)(uint32_t,const uint16_t *,const uint16_t *,uint32_t,int32_t,int32_t,int32_t,int32_t,uintptr_t,uintptr_t,uintptr_t,void *);
int32_t (WIN *show_window)(uintptr_t,int32_t);
int32_t (WIN *destroy_window)(uintptr_t);
intptr_t (WIN *default_proc)(uintptr_t,uint32_t,uintptr_t,intptr_t);
int32_t (WIN *post)(uintptr_t,uint32_t,uintptr_t,intptr_t);
void (WIN *quit)(int32_t);
int32_t (WIN *peek)(struct Msg *,uintptr_t,uint32_t,uint32_t,uint32_t);
intptr_t (WIN *dispatch)(const struct Msg *);
int32_t (WIN *translate)(const struct Msg *);
static int created,destroyed,ncdestroyed;
static int parameter;
static intptr_t WIN procedure(uintptr_t hwnd,uint32_t message,uintptr_t wp,intptr_t lp) {
    if(message==0x81 || message==1){
        const struct Create *create=(void *)lp;
        assert(create->width==640 && create->height==480 && create->parameter==&parameter);
    }
    if(message==1){created++;assert(post(hwnd,0x8001,1,2));}
    if(message==2){destroyed++;quit(17);}
    if(message==0x82){ncdestroyed++;}
    return default_proc(hwnd,message,wp,lp);
}
int main(int argc,char **argv) {
    assert(argc>=2);
    void *lib=dlopen(argv[1],RTLD_NOW|RTLD_LOCAL);
    if(!lib){fprintf(stderr,"%s\n",dlerror());return 1;}
    const void *(*resolve)(const char *,const char *)=dlsym(lib,"peekgame_resolve");assert(resolve);
    #define API(var,name) var=(void *)resolve("USER32.dll",name);assert(var)
    API(register_class,"RegisterClassExW");API(create_window,"CreateWindowExW");
    API(show_window,"ShowWindow");API(destroy_window,"DestroyWindow");
    API(default_proc,"DefWindowProcW");API(post,"PostMessageW");API(quit,"PostQuitMessage");
    API(peek,"PeekMessageW");API(dispatch,"DispatchMessageW");API(translate,"TranslateMessage");
    uintptr_t (WIN *cursor)(uintptr_t,const uint16_t *)=(void *)resolve("USER32.dll","LoadCursorW");
    int32_t (WIN *adjust)(struct Rect *,uint32_t,int32_t,uint32_t)=(void *)resolve("USER32.dll","AdjustWindowRectEx");
    last_error=(void *)resolve("kernel32.dll","GetLastError");assert(cursor&&adjust&&last_error);
    uint32_t (*native_window)(uintptr_t,struct Native *)=dlsym(lib,"peekgame_x11_window");assert(native_window);
    uintptr_t arrow=cursor(0,(const uint16_t *)32512);
    if(argc==3){assert(arrow==0&&last_error()==50);puts("PASS: unavailable X11 display fails explicitly");return 0;}
    assert(arrow && cursor(0,(const uint16_t *)32512)==arrow);
    const uint16_t name[]={'T','e','s','t',0},title[]={'N','a','t','i','v','e',0};
    struct Class cls={.size=sizeof cls,.proc=procedure,.cursor=arrow,.name=name};
    uint16_t atom=register_class(&cls);assert(atom);
    assert(!register_class(&cls)&&last_error()==1410);
    struct Rect rect={0,0,640,480};assert(adjust(&rect,0xcf0000,0,0));
    assert(rect.right==640&&rect.bottom==480);
    assert(!adjust(&rect,0xcf0000,1,0)&&last_error()==50);
    uintptr_t hwnd=create_window(0,(const uint16_t *)(uintptr_t)atom,title,0xcf0000,10,20,640,480,0,0,0,&parameter);
    assert(hwnd&&created==1);
    struct Native native;assert(native_window(hwnd,&native)==0);
    assert(native.window!=hwnd&&native.display);
    XWindowAttributes attrs;assert(XGetWindowAttributes(native.display,native.window,&attrs));
    assert(attrs.width==640&&attrs.height==480&&attrs.map_state==IsUnmapped);
    assert(!show_window(hwnd,5));XSync(native.display,False);
    assert(XGetWindowAttributes(native.display,native.window,&attrs)&&attrs.map_state==IsViewable);
    assert(show_window(hwnd,0)==1);assert(!show_window(hwnd,5));
    struct Msg message;assert(peek(&message,hwnd,0x8001,0x8001,1));
    assert(message.wp==1&&message.lp==2); /* reentrant post from WM_CREATE */
    test_native_events(hwnd,native);
    assert(destroyed==1&&ncdestroyed==1);
    assert(native_window(hwnd,&native)==1400);
    assert(!destroy_window(hwnd)&&last_error()==1400);
    assert(!post(hwnd,0x8000,0,0)&&last_error()==1400);
    dlclose(lib);
    puts("PASS: native X11 Windows ABI, lifecycle, reentry, queue filters, resize, keyboard, close");
}
