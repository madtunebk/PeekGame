/* GLX context/format operations. Host-only types stay on this side of the ABI. */
#include <GL/gl.h>
#include <GL/glx.h>
#include <GL/glxext.h>
#include <X11/Xlib.h>
#include <limits.h>
#include <stdlib.h>
#include <string.h>
void *peekgame_glx_format(Display *d,Window window,int depth,int stencil,int alpha,int dbl,int *actual) {
 XWindowAttributes wa;if(!XGetWindowAttributes(d,window,&wa))return NULL;
 int count=0;GLXFBConfig *list=glXGetFBConfigs(d,XScreenNumberOfScreen(wa.screen),&count),best=NULL;
 long best_score=LONG_MAX;
 for(int i=0;list&&i<count;i++){
  XVisualInfo *visual=glXGetVisualFromFBConfig(d,list[i]);if(!visual)continue;
  int match=visual->visualid==XVisualIDFromVisual(wa.visual);XFree(visual);if(!match)continue;
  int attrs[]={GLX_RED_SIZE,GLX_GREEN_SIZE,GLX_BLUE_SIZE,GLX_ALPHA_SIZE,GLX_DEPTH_SIZE,GLX_STENCIL_SIZE,GLX_DOUBLEBUFFER};
  int values[7],drawable,render;
  glXGetFBConfigAttrib(d,list[i],GLX_DRAWABLE_TYPE,&drawable);
  glXGetFBConfigAttrib(d,list[i],GLX_RENDER_TYPE,&render);
  if(!(drawable&GLX_WINDOW_BIT)||!(render&GLX_RGBA_BIT))continue;
  for(int j=0;j<7;j++)glXGetFBConfigAttrib(d,list[i],attrs[j],&values[j]);
  if(values[6]!=dbl||values[0]<8||values[1]<8||values[2]<8)continue;
  long score=labs(values[4]-depth)*100+labs(values[5]-stencil)*10+labs(values[3]-alpha);
  if(score<best_score){best=list[i];best_score=score;memcpy(actual,values,sizeof values);}
 }
 if(list) XFree(list);
 return best;
}
void *peekgame_glx_create(Display *d,void *config,void *share,const int *attrs){
 if(attrs){
  PFNGLXCREATECONTEXTATTRIBSARBPROC create=(PFNGLXCREATECONTEXTATTRIBSARBPROC)glXGetProcAddressARB((const GLubyte*)"glXCreateContextAttribsARB");
  return create?create(d,config,share,True,attrs):NULL;
 }
 return glXCreateNewContext(d,config,GLX_RGBA_TYPE,share,True);
}
const char *peekgame_glx_extensions(Display *d){return glXQueryExtensionsString(d,DefaultScreen(d));}
int peekgame_glx_interval(Display *d,Window drawable,int interval){
 const char *ext=peekgame_glx_extensions(d);if(!ext||interval<0)return 0;
 if(strstr(ext,"GLX_MESA_swap_control")){
  PFNGLXSWAPINTERVALMESAPROC f=(PFNGLXSWAPINTERVALMESAPROC)glXGetProcAddressARB((const GLubyte*)"glXSwapIntervalMESA");
  return f&&f((unsigned)interval)==0;
 }
 if(strstr(ext,"GLX_EXT_swap_control")){
  PFNGLXSWAPINTERVALEXTPROC f=(PFNGLXSWAPINTERVALEXTPROC)glXGetProcAddressARB((const GLubyte*)"glXSwapIntervalEXT");
  if(!f) return 0;
  f(d,drawable,interval);
  unsigned actual=~0u;glXQueryDrawable(d,drawable,GLX_SWAP_INTERVAL_EXT,&actual);return actual==(unsigned)interval;
 }
 return 0;
}
