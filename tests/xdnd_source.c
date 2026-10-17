/* Controlled real X11 selection owner, never linked into PeekGame. */
#include <X11/Xlib.h>
#include <X11/Xatom.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
static Display*d;static Window source,target;static Atom enter,position,leave,drop,status,finished,selection,uri,copy;
static void require(int ok,const char*what){if(!ok){fprintf(stderr,"Xdnd source failed: %s\n",what);exit(1);}}
static Atom atom(const char*n){return XInternAtom(d,n,False);}
static void send(Atom type,long a,long b,long c,long e){XEvent event={0};event.xclient.type=ClientMessage;event.xclient.display=d;event.xclient.window=target;event.xclient.message_type=type;event.xclient.format=32;event.xclient.data.l[0]=source;event.xclient.data.l[1]=a;event.xclient.data.l[2]=b;event.xclient.data.l[3]=c;event.xclient.data.l[4]=e;XSendEvent(d,target,False,0,&event);XFlush(d);}
static Window find(void){Window root,parent,*children=0;unsigned n=0;Window result=0;if(XQueryTree(d,DefaultRootWindow(d),&root,&parent,&children,&n)){for(unsigned i=0;i<n;i++){char*title=0;if(XFetchName(d,children[i],&title)&&title){if(!strcmp(title,"PeekGame Xdnd OLE test"))result=children[i];XFree(title);}}}if(children)XFree(children);return result;}
static void answer(XSelectionRequestEvent*r,int phase){
 const char *files=phase==3?"file://remote.invalid/not-local\r\n":"# file list\r\nfile:///drag-%C3%A9.txt\r\nfile://localhost/space%20file.txt\r\n";
 Atom prop=r->property?r->property:r->target;XChangeProperty(d,r->requestor,prop,uri,8,PropModeReplace,(const unsigned char*)files,(int)strlen(files));
 XEvent reply={0};reply.xselection.type=SelectionNotify;reply.xselection.display=d;reply.xselection.requestor=r->requestor;reply.xselection.selection=r->selection;reply.xselection.target=r->target;reply.xselection.property=prop;reply.xselection.time=r->time;XSendEvent(d,r->requestor,False,0,&reply);XFlush(d);
}
int main(int argc,char **argv){
 (void)argv;int reentrant=argc>1;
 alarm(18);d=XOpenDisplay(0);require(d!=0,"open display");source=XCreateSimpleWindow(d,DefaultRootWindow(d),0,0,1,1,0,0,0);
 enter=atom("XdndEnter");position=atom("XdndPosition");leave=atom("XdndLeave");drop=atom("XdndDrop");status=atom("XdndStatus");finished=atom("XdndFinished");selection=atom("XdndSelection");uri=atom("text/uri-list");copy=atom("XdndActionCopy");
 for(int i=0;i<100&&!target;i++){target=find();usleep(20000);}require(target!=0,"find target");
 Atom kind;int format;unsigned long n,left;unsigned char*value=0;
 for(int i=0;i<100;i++){XGetWindowProperty(d,target,atom("XdndAware"),0,1,False,XA_ATOM,&kind,&format,&n,&left,&value);int ready=value&&n==1&&format==32&&*(unsigned long*)value==5;if(value)XFree(value);value=0;if(ready)break;usleep(20000);require(i!=99,"XdndAware property");}
 for(int phase=1;phase<=(reentrant?1:3);phase++){
  XSetSelectionOwner(d,selection,source,CurrentTime);if(phase==2){Atom types[]={atom("text/plain"),uri,atom("application/octet-stream"),atom("UTF8_STRING")};XChangeProperty(d,source,atom("XdndTypeList"),XA_ATOM,32,PropModeReplace,(unsigned char*)types,4);}
  send(enter,(5L<<24)|(phase==2),phase==2?0:uri,0,0);send(position,0,(115L<<16)|126,CurrentTime,copy);
  int selected=0,statuses=0,moved=0,sentdrop=0,complete=0;
  while(!complete){XEvent event;XNextEvent(d,&event);
   if(event.type==SelectionRequest){answer(&event.xselectionrequest,phase);selected=1;}
   else if(event.type==ClientMessage&&event.xclient.message_type==status){statuses++;int accepted=(event.xclient.data.l[1]&1)!=0;
    if(phase==1&&selected&&accepted){if(!moved){send(position,0,(119L<<16)|130,CurrentTime,copy);moved=1;}else if(!sentdrop){send(drop,0,CurrentTime,0,0);sentdrop=1;}}
    if(phase!=1&&selected&&statuses>=2){require(!accepted,"rejected transfer accepted");if(phase==2){send(leave,0,0,0,0);complete=1;}else if(!sentdrop){send(drop,0,CurrentTime,0,0);sentdrop=1;}}
   }else if(event.type==ClientMessage&&event.xclient.message_type==finished){require(sentdrop,"unexpected finished");require((event.xclient.data.l[1]&1)==(phase==1),"completion result");complete=1;}
  }
 }
 if(reentrant){XDestroyWindow(d,source);XCloseDisplay(d);puts("XDND_REENTRANT_PASS");return 0;}
 XEvent close={0};close.xclient.type=ClientMessage;close.xclient.display=d;close.xclient.window=target;close.xclient.message_type=atom("WM_PROTOCOLS");close.xclient.format=32;close.xclient.data.l[0]=atom("WM_DELETE_WINDOW");XSendEvent(d,target,False,0,&close);XFlush(d);XDestroyWindow(d,source);XCloseDisplay(d);puts("XDND_SOURCE_PASS");return 0;
}
