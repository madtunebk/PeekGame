#include <windows.h>
#include <GL/gl.h>
#include <GL/glext.h>
static void check(int ok, UINT code) {if(!ok) ExitProcess(code);}
static LRESULT CALLBACK wndproc(HWND w,UINT m,WPARAM a,LPARAM b){return DefWindowProcW(w,m,a,b);}
typedef HGLRC (WINAPI *CreateAttrs)(HDC,HGLRC,const int*);
#define LOAD(type,name) type name=(type)wglGetProcAddress(#name);check(name!=0,__LINE__)
static HDC shared_dc;static HGLRC shared_rc;
static DWORD WINAPI worker(void *unused){(void)unused;check(wglMakeCurrent(shared_dc,shared_rc),__LINE__);return 0;}
static DWORD WINAPI busy_worker(void *unused){(void)unused;check(!wglMakeCurrent(shared_dc,shared_rc)&&GetLastError()==ERROR_BUSY,__LINE__);return 0;}
void entry(void){
 WNDCLASSW wc={0};wc.lpfnWndProc=wndproc;wc.lpszClassName=L"PeekGL";
 check(RegisterClassW(&wc)!=0,__LINE__);
 HWND w=CreateWindowExW(0,wc.lpszClassName,L"PeekGame native OpenGL test",WS_OVERLAPPEDWINDOW|WS_VISIBLE,0,0,160,160,0,0,0,0);
 check(w!=0,__LINE__);HDC dc=GetDC(w);
 PIXELFORMATDESCRIPTOR p={0};p.nSize=sizeof p;p.nVersion=1;p.dwFlags=PFD_DRAW_TO_WINDOW|PFD_SUPPORT_OPENGL|PFD_DOUBLEBUFFER;p.iPixelType=PFD_TYPE_RGBA;p.cColorBits=24;p.cDepthBits=24;p.cStencilBits=8;
 int format=ChoosePixelFormat(dc,&p);check(format!=0,__LINE__);
 PIXELFORMATDESCRIPTOR actual={0};check(DescribePixelFormat(dc,format,sizeof actual,&actual)!=0,__LINE__);
 check(actual.nSize==40&&actual.cRedBits>=8&&actual.cDepthBits>=24,__LINE__);
 check(SetPixelFormat(dc,format,&p),__LINE__);check(GetPixelFormat(dc)==format,__LINE__);
 HGLRC legacy=wglCreateContext(dc);check(legacy!=0,__LINE__);check(wglMakeCurrent(dc,legacy),__LINE__);
 CreateAttrs create=(CreateAttrs)wglGetProcAddress("wglCreateContextAttribsARB");check(create!=0,__LINE__);
 int attrs[]={0x2091,3,0x2092,3,0x9126,1,0};HGLRC rc=create(dc,0,attrs);check(rc!=0,__LINE__);
 check(wglMakeCurrent(dc,rc),__LINE__);check(wglDeleteContext(legacy),__LINE__);
 check(wglGetCurrentContext()==rc&&wglGetCurrentDC()==dc,__LINE__);
 check(glGetString(GL_VERSION)!=0,__LINE__);
 glViewport(0,0,64,64);glClearColor(.25f,.5f,.75f,1.f);glClearDepth(.125);glClear(GL_COLOR_BUFFER_BIT|GL_DEPTH_BUFFER_BIT);
 unsigned char pixel[4]={0};glReadPixels(3,3,1,1,GL_RGBA,GL_UNSIGNED_BYTE,pixel);
 check(pixel[0]>=63&&pixel[0]<=65&&pixel[1]>=127&&pixel[1]<=129&&pixel[2]>=190&&pixel[2]<=192,__LINE__);
 LOAD(PFNGLCREATESHADERPROC,glCreateShader);LOAD(PFNGLSHADERSOURCEPROC,glShaderSource);LOAD(PFNGLCOMPILESHADERPROC,glCompileShader);LOAD(PFNGLGETSHADERIVPROC,glGetShaderiv);
 LOAD(PFNGLCREATEPROGRAMPROC,glCreateProgram);LOAD(PFNGLATTACHSHADERPROC,glAttachShader);LOAD(PFNGLLINKPROGRAMPROC,glLinkProgram);LOAD(PFNGLGETPROGRAMIVPROC,glGetProgramiv);LOAD(PFNGLUSEPROGRAMPROC,glUseProgram);
 LOAD(PFNGLGETUNIFORMLOCATIONPROC,glGetUniformLocation);LOAD(PFNGLUNIFORM4FPROC,glUniform4f);LOAD(PFNGLGENVERTEXARRAYSPROC,glGenVertexArrays);LOAD(PFNGLBINDVERTEXARRAYPROC,glBindVertexArray);
 const char *vs="#version 330 core\nvoid main(){vec2 p[3]=vec2[3](vec2(-1,-1),vec2(3,-1),vec2(-1,3));gl_Position=vec4(p[gl_VertexID],0,1);}";
 const char *fs="#version 330 core\nuniform vec4 tint;out vec4 color;void main(){color=tint;}";
 GLuint v=glCreateShader(GL_VERTEX_SHADER),f=glCreateShader(GL_FRAGMENT_SHADER);GLint ok=0;
 glShaderSource(v,1,&vs,0);glCompileShader(v);glGetShaderiv(v,GL_COMPILE_STATUS,&ok);check(ok,__LINE__);
 glShaderSource(f,1,&fs,0);glCompileShader(f);glGetShaderiv(f,GL_COMPILE_STATUS,&ok);check(ok,__LINE__);
 GLuint program=glCreateProgram();glAttachShader(program,v);glAttachShader(program,f);glLinkProgram(program);glGetProgramiv(program,GL_LINK_STATUS,&ok);check(ok,__LINE__);
 glUseProgram(program);glUniform4f(glGetUniformLocation(program,"tint"),1.f,.25f,0.f,1.f);
 GLuint vao=0;glGenVertexArrays(1,&vao);glBindVertexArray(vao);glDrawArrays(GL_TRIANGLES,0,3);
 glReadPixels(3,3,1,1,GL_RGBA,GL_UNSIGNED_BYTE,pixel);check(pixel[0]==255&&pixel[1]>=63&&pixel[1]<=65&&pixel[2]==0,__LINE__);
 LOAD(PFNGLGETSTRINGIPROC,glGetStringi);check(glGetStringi(GL_EXTENSIONS,0)!=0,__LINE__);
 LOAD(PFNGLGENBUFFERSPROC,glGenBuffers);LOAD(PFNGLBINDBUFFERPROC,glBindBuffer);LOAD(PFNGLBUFFERDATAPROC,glBufferData);LOAD(PFNGLMAPBUFFERRANGEPROC,glMapBufferRange);LOAD(PFNGLUNMAPBUFFERPROC,glUnmapBuffer);LOAD(PFNGLISBUFFERPROC,glIsBuffer);
 GLuint buffer=0;glGenBuffers(1,&buffer);glBindBuffer(GL_ARRAY_BUFFER,buffer);glBufferData(GL_ARRAY_BUFFER,16,0,GL_DYNAMIC_DRAW);
 unsigned *mapped=(unsigned*)glMapBufferRange(GL_ARRAY_BUFFER,0,16,GL_MAP_WRITE_BIT);check(mapped!=0,__LINE__);mapped[0]=0x12345678;check(glUnmapBuffer(GL_ARRAY_BUFFER),__LINE__);
 HGLRC shared=create(dc,rc,attrs);check(shared!=0,__LINE__);check(wglMakeCurrent(dc,shared),__LINE__);check(glIsBuffer(buffer),__LINE__);check(wglMakeCurrent(dc,rc),__LINE__);check(wglDeleteContext(shared),__LINE__);
 shared_dc=dc;shared_rc=rc;
 HANDLE thread=CreateThread(0,0,busy_worker,0,0,0);check(thread!=0,__LINE__);check(WaitForSingleObject(thread,5000)==WAIT_OBJECT_0,__LINE__);CloseHandle(thread);
 check(wglMakeCurrent(0,0),__LINE__);thread=CreateThread(0,0,worker,0,0,0);check(thread!=0,__LINE__);check(WaitForSingleObject(thread,5000)==WAIT_OBJECT_0,__LINE__);CloseHandle(thread);
 check(wglMakeCurrent(dc,rc),__LINE__);
 check(glGetError()==GL_NO_ERROR,__LINE__);check(SwapBuffers(dc),__LINE__);
 check(SetWindowPos(w,0,0,0,240,200,SWP_NOMOVE|SWP_NOZORDER),__LINE__);
 glClearColor(0.f,1.f,0.f,1.f);glClear(GL_COLOR_BUFFER_BIT);glReadPixels(3,3,1,1,GL_RGBA,GL_UNSIGNED_BYTE,pixel);check(pixel[0]==0&&pixel[1]==255&&pixel[2]==0,__LINE__);
 check(wglGetProcAddress("glPeekGameNotAFunction")==0,__LINE__);
 check(wglMakeCurrent(0,0),__LINE__);check(wglGetCurrentContext()==0,__LINE__);
 check(wglDeleteContext(rc),__LINE__);check(!wglMakeCurrent(dc,rc),__LINE__);
 ReleaseDC(w,dc);DestroyWindow(w);
 const char text[]="OPENGL_BRIDGE_PASS\n";DWORD written=0;WriteFile(GetStdHandle(STD_OUTPUT_HANDLE),text,sizeof(text)-1,&written,0);ExitProcess(0);
}
