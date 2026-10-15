#include "driver.h"
#include <vulkan/vulkan_xlib.h>
namespace dxvk::wsi {
bool PeekX11Driver::isWindow(HWND window) {return window && __atomic_load_n(&surface(window)->alive,__ATOMIC_ACQUIRE)!=0;}
bool PeekX11Driver::isMinimized(HWND window) {return !isWindow(window) || !__atomic_load_n(&surface(window)->visible,__ATOMIC_ACQUIRE);}
bool PeekX11Driver::isOccluded(HWND window) {return isMinimized(window);}
void PeekX11Driver::getWindowSize(HWND window,uint32_t* width,uint32_t* height) {
  auto s=surface(window);
  if(width)*width=isWindow(window)?__atomic_load_n(&s->width,__ATOMIC_ACQUIRE):0;
  if(height)*height=isWindow(window)?__atomic_load_n(&s->height,__ATOMIC_ACQUIRE):0;
}
void PeekX11Driver::resizeWindow(HWND window,DxvkWindowState*,uint32_t width,uint32_t height) {
  if(!isWindow(window)||!width||!height)throw DxvkError("PeekX11: invalid resize");
  auto s=surface(window);auto d=static_cast<Display*>(s->display);
  XResizeWindow(d,s->window,width,height);XFlush(d);
}
void PeekX11Driver::saveWindowState(HWND window,DxvkWindowState* state,bool) {
  getWindowSize(window,&state->peek.width,&state->peek.height);
}
void PeekX11Driver::restoreWindowState(HWND window,DxvkWindowState* state,bool) {
  resizeWindow(window,state,state->peek.width,state->peek.height);
}
bool PeekX11Driver::setWindowMode(HMONITOR,HWND,DxvkWindowState*,const WsiMode&) {return false;}
bool PeekX11Driver::enterFullscreenMode(HMONITOR,HWND,DxvkWindowState*,bool) {return false;}
bool PeekX11Driver::leaveFullscreenMode(HWND,DxvkWindowState*) {return true; /* never entered exclusive mode */}
bool PeekX11Driver::restoreDisplayMode() {return true; /* no display mode was changed */}
HMONITOR PeekX11Driver::getWindowMonitor(HWND window) {return isWindow(window)?getDefaultMonitor():nullptr;}
void PeekX11Driver::updateFullscreenWindow(HMONITOR,HWND,bool) {throw DxvkError("PeekX11: exclusive fullscreen unsupported");}
VkResult PeekX11Driver::createSurface(HWND window,PFN_vkGetInstanceProcAddr getProc,VkInstance instance,VkSurfaceKHR* output) {
  if(!output)return VK_ERROR_INITIALIZATION_FAILED;
  *output=VK_NULL_HANDLE;
  if(!isWindow(window))return VK_ERROR_SURFACE_LOST_KHR;
  auto create=reinterpret_cast<PFN_vkCreateXlibSurfaceKHR>(getProc(instance,"vkCreateXlibSurfaceKHR"));
  if(!create)return VK_ERROR_EXTENSION_NOT_PRESENT;
  auto s=surface(window);
  VkXlibSurfaceCreateInfoKHR info={VK_STRUCTURE_TYPE_XLIB_SURFACE_CREATE_INFO_KHR};
  info.dpy=static_cast<Display*>(s->display);info.window=s->window;
  return create(instance,&info,nullptr,output);
}
}
