/* Host XRandR mode discovery. No fabricated resolutions or display changes. */
#include <X11/Xlib.h>
#include <X11/extensions/Xrandr.h>
#include <stdint.h>
struct PeekMode {uint32_t width,height,hz,flags;};
static struct PeekMode mode(const XRRModeInfo *m,Rotation rotation){
 double hz=m->hTotal&&m->vTotal?(double)m->dotClock/((double)m->hTotal*m->vTotal):0;
 if(m->modeFlags&RR_Interlace)hz*=2;
 if(m->modeFlags&RR_DoubleScan)hz/=2;
 struct PeekMode result={m->width,m->height,(uint32_t)(hz+0.5),(m->modeFlags&RR_Interlace)?2:0};
 if(rotation&(RR_Rotate_90|RR_Rotate_270)){result.width=m->height;result.height=m->width;}return result;
}
uint32_t peekgame_x11_modes(Display*d,Atom monitor,struct PeekMode*out,uint32_t cap,struct PeekMode*current){
 int count=0;XRRMonitorInfo*list=XRRGetMonitors(d,DefaultRootWindow(d),True,&count);
 if(!list)return 0;
 XRRScreenResources*resources=XRRGetScreenResourcesCurrent(d,DefaultRootWindow(d));
 uint32_t found=0;
 for(int i=0;resources&&i<count;i++)if(list[i].name==monitor&&list[i].noutput==1){
  XRROutputInfo*output=XRRGetOutputInfo(d,resources,list[i].outputs[0]);if(!output)break;
  XRRCrtcInfo*crtc=output->crtc?XRRGetCrtcInfo(d,resources,output->crtc):NULL;
  Rotation rotation=crtc?crtc->rotation:RR_Rotate_0;
  for(int j=0;j<output->nmode;j++)for(int k=0;k<resources->nmode;k++)if(resources->modes[k].id==output->modes[j]){
   struct PeekMode value=mode(&resources->modes[k],rotation);
   if(found<cap)out[found]=value;
   found++;
   if(crtc&&crtc->mode==resources->modes[k].id){*current=value;current->width=list[i].width;current->height=list[i].height;}
   break;
  }
  if(crtc)XRRFreeCrtcInfo(crtc);
  XRRFreeOutputInfo(output);break;
 }
 if(resources)XRRFreeScreenResources(resources);
 XRRFreeMonitors(list);return found;
}
