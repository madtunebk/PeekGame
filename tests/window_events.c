#include "window_abi.h"
#include <X11/keysym.h>
#include <string.h>
void test_native_events(uintptr_t hwnd,struct Native native) {
    struct Msg message;
    while(peek(&message,0,0,0,1)) { dispatch(&message); }
    XResizeWindow(native.display,native.window,321,123);
    XSync(native.display,False);
    assert(peek(&message,hwnd,5,5,1));
    assert(message.message==5 && (message.lp&0xffff)==321 && ((message.lp>>16)&0xffff)==123);
    assert(post(hwnd,0x8002,11,22));
    assert(peek(&message,hwnd,0x8002,0x8002,0));
    assert(message.wp==11 && message.lp==22);
    assert(peek(&message,hwnd,0x8002,0x8002,0)); /* no-remove */
    assert(peek(&message,hwnd,0x8002,0x8002,1));
    assert(!peek(&message,hwnd,0x8002,0x8002,1));
    assert(post(0,0x8003,33,44));
    assert(!peek(&message,hwnd,0x8003,0x8003,1));
    assert(peek(&message,UINTPTR_MAX,0x8003,0x8003,1));
    XEvent event;memset(&event,0,sizeof event);
    event.xkey.type=KeyPress;event.xkey.display=native.display;
    event.xkey.window=native.window;event.xkey.root=DefaultRootWindow(native.display);
    event.xkey.same_screen=True;event.xkey.keycode=XKeysymToKeycode(native.display,XK_a);
    assert(XSendEvent(native.display,native.window,False,KeyPressMask,&event));
    XSync(native.display,False);
    assert(peek(&message,hwnd,0x100,0x100,1));
    assert(message.wp=='A' && message.message==0x100 && (message.lp&0xffff)==1);
    assert(translate(&message));
    assert(peek(&message,hwnd,0x102,0x102,1) && message.wp=='a');
    event.xkey.type=KeyRelease;
    assert(XSendEvent(native.display,native.window,False,KeyReleaseMask,&event));
    XSync(native.display,False);
    assert(peek(&message,hwnd,0x101,0x101,1) && message.wp=='A');
    assert(((uint32_t)message.lp>>30)==3);
    memset(&event,0,sizeof event);
    event.xclient.type=ClientMessage;event.xclient.display=native.display;
    event.xclient.window=native.window;event.xclient.format=32;
    event.xclient.message_type=XInternAtom(native.display,"WM_PROTOCOLS",False);
    event.xclient.data.l[0]=(long)XInternAtom(native.display,"WM_DELETE_WINDOW",False);
    assert(XSendEvent(native.display,native.window,False,NoEventMask,&event));
    XSync(native.display,False);
    assert(peek(&message,hwnd,0x10,0x10,1));dispatch(&message);
    /* Quit must pass message-range filters and remains a thread notification. */
    assert(peek(&message,0,0x8000,0x9000,1));
    assert(message.hwnd==0 && message.message==0x12 && message.wp==17);
}
