#ifndef WINDOW_ABI_H
#define WINDOW_ABI_H
#include <assert.h>
#include <stddef.h>
#include <stdint.h>
#include <X11/Xlib.h>
#define WIN __attribute__((ms_abi))
typedef intptr_t (WIN *WndProc)(uintptr_t,uint32_t,uintptr_t,intptr_t);
struct Class {uint32_t size,style; WndProc proc; int32_t cls_extra,wnd_extra; uintptr_t instance,icon,cursor,background; const uint16_t *menu,*name; uintptr_t small_icon;};
struct Point {int32_t x,y;};
struct Msg {uintptr_t hwnd; uint32_t message; uintptr_t wp; intptr_t lp; uint32_t time; struct Point point; uint32_t reserved;};
struct Rect {int32_t left,top,right,bottom;};
struct Create {void *parameter;uintptr_t instance,menu,parent;int32_t height,width,y,x,style;const uint16_t *title,*class_name;uint32_t ex_style;};
struct Native {Display *display;Window window;};
_Static_assert(sizeof(struct Class)==80,"WNDCLASSEXW");
_Static_assert(sizeof(struct Msg)==48,"MSG");
_Static_assert(sizeof(struct Create)==80,"CREATESTRUCTW");
extern uint32_t (WIN *last_error)(void);
extern uint16_t (WIN *register_class)(const struct Class *);
extern uintptr_t (WIN *create_window)(uint32_t,const uint16_t *,const uint16_t *,uint32_t,int32_t,int32_t,int32_t,int32_t,uintptr_t,uintptr_t,uintptr_t,void *);
extern int32_t (WIN *show_window)(uintptr_t,int32_t);
extern int32_t (WIN *destroy_window)(uintptr_t);
extern intptr_t (WIN *default_proc)(uintptr_t,uint32_t,uintptr_t,intptr_t);
extern int32_t (WIN *post)(uintptr_t,uint32_t,uintptr_t,intptr_t);
extern void (WIN *quit)(int32_t);
extern int32_t (WIN *peek)(struct Msg *,uintptr_t,uint32_t,uint32_t,uint32_t);
extern intptr_t (WIN *dispatch)(const struct Msg *);
extern int32_t (WIN *translate)(const struct Msg *);
extern uint32_t observed_size_count;
extern intptr_t observed_size;
void test_native_input(uintptr_t hwnd,struct Native native,const void *(*resolve)(const char *,const char *));
void test_native_events(uintptr_t hwnd,struct Native native);
#endif
