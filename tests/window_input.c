/* Real X11 input -> Win64 USER32 messages, raw bytes and cursor conversion. */
#include "window_abi.h"
#include <string.h>
#include <stdio.h>
extern int XTestFakeRelativeMotionEvent(Display *, int, int, unsigned long);
extern int XTestFakeButtonEvent(Display *, unsigned int, int, unsigned long);
struct RawDevice { uint16_t page,usage; uint32_t flags; uintptr_t target; };

/* Exercise the real server's implicit button grab, not XSendEvent: keep each
   button held while pressing the next, moving, then releasing independently. */
static void check_mouse_chord(uintptr_t hwnd, struct Native native, const void *(*resolve)(const char *,const char *)) {
    int16_t (WIN *key_state)(int32_t)=(void *)resolve("user32.dll","GetAsyncKeyState");
    uint32_t (WIN *raw_data)(uintptr_t,uint32_t,void *,uint32_t *,uint32_t)=(void *)resolve("user32.dll","GetRawInputData");
    const unsigned buttons[]={3,1,2,8,9};
    const int vks[]={2,1,4,5,6};
    const uintptr_t flags[]={2,1,0x10,0x20,0x40};
    const uint32_t messages[]={0x204,0x201,0x207,0x20b,0x20b};
    const uint16_t raw_flags[]={4,1,0x10,0x40,0x100};
    uintptr_t held=0;
    for(int step=0;step<10;step++){
        int b=step%5,down=step<5;
        held=down?held|flags[b]:held&~flags[b];
        assert(XTestFakeButtonEvent(native.display,buttons[b],down,0));XSync(native.display,False);
        struct Msg msg;int legacy=0,raw_button=0;
        while(peek(&msg,0,0,0,1)){
            if(msg.message==messages[b]+!down){
                assert(msg.hwnd==hwnd);
                uintptr_t expected=held|(b>=3?(uintptr_t)(b-2)<<16:0);
                assert(msg.wp==expected);legacy++;
            }
            if(msg.message==0xff){
                unsigned char raw[48];uint32_t bytes=sizeof raw;uint16_t changed;
                assert(raw_data((uintptr_t)msg.lp,0x10000003,raw,&bytes,24)==48);
                memcpy(&changed,raw+28,2);
                if(changed){assert(changed==(down?raw_flags[b]:raw_flags[b]<<1));raw_button++;}
            }
            dispatch(&msg);
        }
        assert(legacy==1&&raw_button==1);
        for(int i=0;i<5;i++)assert((key_state(vks[i])<0)==((held&flags[i])!=0));
        assert(XTestFakeRelativeMotionEvent(native.display,3,2,0));XSync(native.display,False);
        int dx_sum=0,dy_sum=0;
        while(peek(&msg,0,0,0,1)){
            if(msg.message==0xff){
                unsigned char raw[48];uint32_t bytes=sizeof raw;int32_t dx,dy;
                assert(raw_data((uintptr_t)msg.lp,0x10000003,raw,&bytes,24)==48);
                memcpy(&dx,raw+36,4);memcpy(&dy,raw+40,4);dx_sum+=dx;dy_sum+=dy;
            }
            if(msg.message==0x200)assert(msg.wp==held);
            dispatch(&msg);
        }
        if(dx_sum!=3||dy_sum!=2)fprintf(stderr,"mouse chord step=%d held=%#lx motion=%d,%d expected=3,2\n",step,(unsigned long)held,dx_sum,dy_sum);
        assert(dx_sum==3&&dy_sum==2);
    }
}

/* Camera-look capture: a focused game that hides the cursor must read raw mouse
   motion straight from the device through XInput2, unaccelerated and 1:1, not as
   deltas of the pointer-accelerated cursor. Inject real relative motion with
   XTest and confirm the WM_INPUT delta matches it exactly. */
static void check_raw_capture(uintptr_t hwnd, struct Native native, const void *(*resolve)(const char *,const char *)) {
    int32_t (WIN *show_cursor)(int32_t)=(void *)resolve("user32.dll","ShowCursor");
    int32_t (WIN *register_raw)(const struct RawDevice *,uint32_t,uint32_t)=(void *)resolve("user32.dll","RegisterRawInputDevices");
    uint32_t (WIN *raw_data)(uintptr_t,uint32_t,void *,uint32_t *,uint32_t)=(void *)resolve("user32.dll","GetRawInputData");
    assert(show_cursor&&register_raw&&raw_data);
    int32_t (WIN *to_screen)(uintptr_t,struct Point *)=(void *)resolve("user32.dll","ClientToScreen");
    int32_t (WIN *set_position)(int32_t,int32_t)=(void *)resolve("user32.dll","SetCursorPos");
    struct Point center={320,240};assert(to_screen(hwnd,&center));
    assert(set_position(center.x,center.y));
    struct Msg msg;
    /* Hold the input focus the runtime keys relative capture on. */
    XSetInputFocus(native.display,native.window,RevertToParent,CurrentTime);XSync(native.display,False);
    while(peek(&msg,0,0,0,1))dispatch(&msg);
    /* Raw mouse with no target follows the focus window, as real games register it. */
    struct RawDevice device={1,2,0,0};assert(register_raw(&device,1,sizeof device));
    assert(show_cursor(0)==-1); /* hide: enter relative capture */
    while(peek(&msg,0,0,0,1))dispatch(&msg);
    assert(XTestFakeRelativeMotionEvent(native.display,10,12,0));XSync(native.display,False);
    int found=0;
    for(int i=0;i<200&&!found;i++){
        while(peek(&msg,hwnd,0xff,0xff,1)){
            unsigned char raw[48];uint32_t bytes=sizeof raw;
            if(raw_data((uintptr_t)msg.lp,0x10000003,raw,&bytes,24)==48){
                int32_t dx,dy;memcpy(&dx,raw+36,4);memcpy(&dy,raw+40,4);
                if(dx||dy){assert(dx==10&&dy==12);found=1;break;}
            }
        }
        while(peek(&msg,0,0,0,1))dispatch(&msg);
    }
    assert(found);
    check_mouse_chord(hwnd,native,resolve);
    int32_t (WIN *clip)(const struct Rect *)=(void *)resolve("user32.dll","ClipCursor");
    uintptr_t (WIN *capture)(uintptr_t)=(void *)resolve("user32.dll","SetCapture");
    int32_t (WIN *release)(void)=(void *)resolve("user32.dll","ReleaseCapture");
    struct Point origin={0,0};assert(to_screen(hwnd,&origin));
    struct Rect clip_rect={origin.x,origin.y,origin.x+640,origin.y+480};
    assert(clip(&clip_rect));capture(hwnd);
    check_mouse_chord(hwnd,native,resolve);
    assert(release());assert(clip(NULL));
    puts("PASS: five overlapping mouse buttons, independent releases and camera motion, clipped and unclipped");
    assert(show_cursor(1)==0); /* show again */
    device.flags=1;device.target=0;assert(register_raw(&device,1,sizeof device));
    while(peek(&msg,0,0,0,1))dispatch(&msg);
}
static void check_native_input(uintptr_t hwnd, struct Native native, const void *(*resolve)(const char *,const char *)) {
    uintptr_t (WIN *system_menu)(uintptr_t,int32_t)=(void *)resolve("user32.dll","GetSystemMenu");
    uintptr_t (WIN *get_menu)(uintptr_t)=(void *)resolve("user32.dll","GetMenu");
    uint32_t (WIN *enable_menu)(uintptr_t,uint32_t,uint32_t)=(void *)resolve("user32.dll","EnableMenuItem");
    assert(system_menu&&get_menu&&enable_menu);
    assert(!system_menu(hwnd,0)&&last_error()==50);
    assert(!system_menu(0,0)&&last_error()==1400);
    assert(!get_menu(hwnd));assert(enable_menu(0,0,0)==UINT32_MAX&&last_error()==1401);
    int32_t (WIN *client_to_screen)(uintptr_t,struct Point *)=(void *)resolve("user32.dll","ClientToScreen");
    int32_t (WIN *screen_to_client)(uintptr_t,struct Point *)=(void *)resolve("user32.dll","ScreenToClient");
    int32_t (WIN *set_position)(int32_t,int32_t)=(void *)resolve("user32.dll","SetCursorPos");
    int32_t (WIN *get_position)(struct Point *)=(void *)resolve("user32.dll","GetCursorPos");
    int32_t (WIN *register_raw)(const struct RawDevice *,uint32_t,uint32_t)=(void *)resolve("user32.dll","RegisterRawInputDevices");
    uint32_t (WIN *raw_data)(uintptr_t,uint32_t,void *,uint32_t *,uint32_t)=(void *)resolve("user32.dll","GetRawInputData");
    int16_t (WIN *key_state)(int32_t)=(void *)resolve("user32.dll","GetAsyncKeyState");
    uintptr_t (WIN *capture)(uintptr_t)=(void *)resolve("user32.dll","SetCapture");
    uintptr_t (WIN *get_capture)(void)=(void *)resolve("user32.dll","GetCapture");
    int32_t (WIN *release_capture)(void)=(void *)resolve("user32.dll","ReleaseCapture");
    assert(client_to_screen&&screen_to_client&&set_position&&get_position&&register_raw&&raw_data&&key_state&&capture&&get_capture&&release_capture);
    int32_t (WIN *client_rect)(uintptr_t,struct Rect *)=(void *)resolve("user32.dll","GetClientRect");
    XWindowAttributes attrs;struct Rect rect;assert(XGetWindowAttributes(native.display,native.window,&attrs));
    assert(client_rect(hwnd,&rect));assert(rect.left==0&&rect.top==0&&rect.right==attrs.width&&rect.bottom==attrs.height);
    struct Point point={37,41},actual;
    assert(client_to_screen(hwnd,&point)); assert(set_position(point.x,point.y));
    XSync(native.display,False); assert(get_position(&actual)); assert(actual.x==point.x&&actual.y==point.y);
    assert(screen_to_client(hwnd,&actual)&&actual.x==37&&actual.y==41);
    struct Msg msg;while(peek(&msg,0,0,0,1))dispatch(&msg);
    struct RawDevice device={1,2,0,hwnd};assert(register_raw(&device,1,sizeof device));
    struct Point origin={0,0};assert(client_to_screen(hwnd,&origin));
    XEvent event;memset(&event,0,sizeof event);
    event.xmotion.type=MotionNotify;event.xmotion.display=native.display;event.xmotion.window=native.window;
    event.xmotion.root=DefaultRootWindow(native.display);event.xmotion.same_screen=True;
    event.xmotion.x=47;event.xmotion.y=53;event.xmotion.x_root=origin.x+47;event.xmotion.y_root=origin.y+53;
    assert(XSendEvent(native.display,native.window,False,PointerMotionMask,&event));XSync(native.display,False);
    assert(peek(&msg,hwnd,0x200,0x200,1));assert((int16_t)msg.lp==47&&(int16_t)(msg.lp>>16)==53);
    assert(msg.point.x==origin.x+47&&msg.point.y==origin.y+53);
    assert(peek(&msg,hwnd,0xff,0xff,1));unsigned char raw[48];uint32_t bytes=sizeof raw;
    assert(raw_data((uintptr_t)msg.lp,0x10000003,raw,&bytes,24)==48);
    int32_t dx,dy;memcpy(&dx,raw+36,4);memcpy(&dy,raw+40,4);assert(dx==10&&dy==12);
    memset(&event,0,sizeof event);event.xbutton.type=ButtonPress;event.xbutton.display=native.display;event.xbutton.window=native.window;
    event.xbutton.root=DefaultRootWindow(native.display);event.xbutton.same_screen=True;
    event.xbutton.x=47;event.xbutton.y=53;event.xbutton.x_root=origin.x+47;event.xbutton.y_root=origin.y+53;event.xbutton.button=1;
    assert(XSendEvent(native.display,native.window,False,ButtonPressMask,&event));XSync(native.display,False);
    assert(peek(&msg,hwnd,0x201,0x201,1)&&msg.wp==1);assert(key_state(1)<0);
    assert(peek(&msg,hwnd,0xff,0xff,1));bytes=sizeof raw;assert(raw_data((uintptr_t)msg.lp,0x10000003,raw,&bytes,24)==48);assert(raw[28]==1);
    event.xbutton.type=ButtonRelease;event.xbutton.state=Button1Mask;
    assert(XSendEvent(native.display,native.window,False,ButtonReleaseMask,&event));XSync(native.display,False);
    assert(peek(&msg,hwnd,0x202,0x202,1)&&msg.wp==0);assert(key_state(1)>=0);
    assert(!capture(hwnd));assert(get_capture()==hwnd);assert(release_capture()&&get_capture()==0);
    device.flags=1;device.target=0;assert(register_raw(&device,1,sizeof device));
}

/* Simulate WM reparenting: decoration offset belongs to the frame, never client
   size. Move the frame without moving the child, then repeat real cursor/raw
   checks so cached parent-relative coordinates cannot accidentally pass. */
void test_native_input(uintptr_t hwnd, struct Native native, const void *(*resolve)(const char *,const char *)) {
    check_native_input(hwnd,native,resolve);
    Window root=DefaultRootWindow(native.display);
    Window frame=XCreateSimpleWindow(native.display,root,133,77,824,644,0,0,0);
    XMapWindow(native.display,frame);
    XReparentWindow(native.display,native.window,frame,9,31);
    XResizeWindow(native.display,native.window,800,600);
    XMapWindow(native.display,native.window);XSync(native.display,False);
    struct Msg msg;while(peek(&msg,0,0,0,1))dispatch(&msg);
    check_native_input(hwnd,native,resolve);
    XMoveWindow(native.display,frame,21,113);XSync(native.display,False);
    while(peek(&msg,0,0,0,1))dispatch(&msg);
    check_native_input(hwnd,native,resolve);
    XReparentWindow(native.display,native.window,root,10,20);
    XResizeWindow(native.display,native.window,640,480);XMapWindow(native.display,native.window);
    XDestroyWindow(native.display,frame);XSync(native.display,False);
    while(peek(&msg,0,0,0,1))dispatch(&msg);
    check_raw_capture(hwnd,native,resolve);
}
