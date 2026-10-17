/* Real X11 input -> Win64 USER32 messages, raw bytes and cursor conversion. */
#include "window_abi.h"
#include <string.h>
struct RawDevice { uint16_t page,usage; uint32_t flags; uintptr_t target; };
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
}
