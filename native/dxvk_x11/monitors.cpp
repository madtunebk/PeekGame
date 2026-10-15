#include "driver.h"
#include <X11/extensions/Xrandr.h>
#include <cstdio>
#include <cstring>
namespace dxvk::wsi {
bool PeekX11Driver::validMonitor(HMONITOR monitor) const {
  return uintptr_t(monitor)>0 && uintptr_t(monitor)<=uintptr_t(ScreenCount(display));
}
HMONITOR PeekX11Driver::enumMonitors(uint32_t index) {
  return index<uint32_t(ScreenCount(display)) ? reinterpret_cast<HMONITOR>(uintptr_t(index+1)) : nullptr;
}
HMONITOR PeekX11Driver::getDefaultMonitor() {return enumMonitors(DefaultScreen(display));}
HMONITOR PeekX11Driver::enumMonitors(const LUID*[],uint32_t count,uint32_t index) {
  // No adapter LUID association is available. DXGI's own monitor fallback handles this.
  return count ? nullptr : enumMonitors(index);
}
bool PeekX11Driver::getDisplayName(HMONITOR monitor,WCHAR(&name)[32]) {
  if (!validMonitor(monitor)) return false;
  char ascii[32];std::snprintf(ascii,sizeof(ascii),"\\\\.\\DISPLAY%u",unsigned(uintptr_t(monitor)));
  std::memset(name,0,sizeof(name));for (size_t i=0;ascii[i] && i<31;i++) name[i]=WCHAR(ascii[i]);
  return true;
}
bool PeekX11Driver::getDesktopCoordinates(HMONITOR monitor,RECT* rect) {
  if (!validMonitor(monitor) || !rect) return false;
  *rect={0,0,DisplayWidth(display,screen(monitor)),DisplayHeight(display,screen(monitor))};return true;
}
bool PeekX11Driver::getCurrentDisplayMode(HMONITOR monitor,WsiMode* mode) {
  if (!validMonitor(monitor) || !mode) return false;
  int s=screen(monitor),depth=DefaultDepth(display,s),count=0,bpp=0;
  auto formats=XListPixmapFormats(display,&count);
  for (int i=0;formats && i<count;i++) if (formats[i].depth==depth) bpp=formats[i].bits_per_pixel;
  if (formats) XFree(formats);
  if (!bpp) return false;
  *mode={uint32_t(DisplayWidth(display,s)),uint32_t(DisplayHeight(display,s)),{0,1},uint32_t(bpp),false};
  auto info=XRRGetScreenInfo(display,RootWindow(display,s));
  if (info) {short rate=XRRConfigCurrentRate(info);if(rate>0)mode->refreshRate.numerator=uint32_t(rate);XRRFreeScreenConfigInfo(info);}
  return true; // 0/1 refresh explicitly denotes unknown (e.g. Xvfb); no invented 60 Hz.
}
bool PeekX11Driver::getDesktopDisplayMode(HMONITOR m,WsiMode* mode) {return getCurrentDisplayMode(m,mode);}
bool PeekX11Driver::getDisplayMode(HMONITOR m,uint32_t index,WsiMode* mode) {return index==0 && getCurrentDisplayMode(m,mode);}
WsiEdidData PeekX11Driver::getMonitorEdid(HMONITOR) {return {}; /* unavailable; no synthetic EDID */}
}
