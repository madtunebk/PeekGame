#include "driver.h"
namespace dxvk::wsi {
PeekX11Driver::PeekX11Driver() {
  if (!XInitThreads() || !(display=XOpenDisplay(nullptr)))
    throw DxvkError("PeekX11: cannot open X11 display");
}
PeekX11Driver::~PeekX11Driver() {XCloseDisplay(display);}
std::vector<const char*> PeekX11Driver::getInstanceExtensions() {
  return {"VK_KHR_surface","VK_KHR_xlib_surface"};
}
static bool createPeekDriver(WsiDriver** driver) {
  try {*driver=new PeekX11Driver();return true;}
  catch (const DxvkError&) {return false;}
}
WsiBootstrap PeekX11WSI={"PeekX11",createPeekDriver};
}
