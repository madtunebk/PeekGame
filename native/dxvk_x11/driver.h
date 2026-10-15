#pragma once
#include "../wsi_platform.h"
#include "../../util/util_error.h"
#include "surface.h"
#include <X11/Xlib.h>
#undef None
#undef Success
namespace dxvk::wsi {
class PeekX11Driver final : public WsiDriver {
  Display* display;
  PeekSurface* surface(HWND window) const {return reinterpret_cast<PeekSurface*>(window);}
  bool validMonitor(HMONITOR monitor) const;
  int screen(HMONITOR monitor) const {return int(uintptr_t(monitor))-1;}
public:
  PeekX11Driver();
  ~PeekX11Driver();
  std::vector<const char*> getInstanceExtensions() override;
  HMONITOR getDefaultMonitor() override;
  HMONITOR enumMonitors(uint32_t) override;
  HMONITOR enumMonitors(const LUID*[],uint32_t,uint32_t) override;
  bool getDisplayName(HMONITOR,WCHAR(&)[32]) override;
  bool getDesktopCoordinates(HMONITOR,RECT*) override;
  bool getDisplayMode(HMONITOR,uint32_t,WsiMode*) override;
  bool getCurrentDisplayMode(HMONITOR,WsiMode*) override;
  bool getDesktopDisplayMode(HMONITOR,WsiMode*) override;
  WsiEdidData getMonitorEdid(HMONITOR) override;
  void getWindowSize(HWND,uint32_t*,uint32_t*) override;
  void resizeWindow(HWND,DxvkWindowState*,uint32_t,uint32_t) override;
  void saveWindowState(HWND,DxvkWindowState*,bool) override;
  void restoreWindowState(HWND,DxvkWindowState*,bool) override;
  bool setWindowMode(HMONITOR,HWND,DxvkWindowState*,const WsiMode&) override;
  bool enterFullscreenMode(HMONITOR,HWND,DxvkWindowState*,bool) override;
  bool leaveFullscreenMode(HWND,DxvkWindowState*) override;
  bool restoreDisplayMode() override;
  HMONITOR getWindowMonitor(HWND) override;
  bool isWindow(HWND) override;
  bool isMinimized(HWND) override;
  bool isOccluded(HWND) override;
  void updateFullscreenWindow(HMONITOR,HWND,bool) override;
  VkResult createSurface(HWND,PFN_vkGetInstanceProcAddr,VkInstance,VkSurfaceKHR*) override;
};
}
