/* C caller uses Windows COM ABI; DXVK Native behind Rust uses SysV ABI. */
#include <windows_base.h>
#undef STDMETHODCALLTYPE
#define STDMETHODCALLTYPE __attribute__((ms_abi))
#define COBJMACROS
#include <d3d11.h>
#include <dxgi.h>
#include "window_abi.h"
#include <dlfcn.h>
#include <stdio.h>
#include <string.h>
static intptr_t (WIN *default_call)(uintptr_t,uint32_t,uintptr_t,intptr_t);
static intptr_t WIN procedure(uintptr_t w,uint32_t m,uintptr_t p,intptr_t l){return default_call(w,m,p,l);}
static void frame(ID3D11Device *device,ID3D11DeviceContext *context,IDXGISwapChain *swap,const float color[4],UINT width,UINT height){
    ID3D11Texture2D *back=NULL,*again=NULL,*staging=NULL;
    ID3D11RenderTargetView *view=NULL;
    assert(SUCCEEDED(IDXGISwapChain_GetBuffer(swap,0,&IID_ID3D11Texture2D,(void **)&back)));
    assert(SUCCEEDED(IDXGISwapChain_GetBuffer(swap,0,&IID_ID3D11Texture2D,(void **)&again))&&again==back);
    ID3D11Texture2D_Release(again);
    IUnknown *unknown=NULL;
    assert(SUCCEEDED(ID3D11Texture2D_QueryInterface(back,&IID_IUnknown,(void **)&unknown))&&(void *)unknown==(void *)back);
    IUnknown_Release(unknown);
    GUID unsupported={0xdeadbeef,1,2,{3}};void *bad=(void *)1;
    assert(ID3D11Texture2D_QueryInterface(back,&unsupported,&bad)==E_NOINTERFACE&&bad==NULL);
    assert(SUCCEEDED(ID3D11Device_CreateRenderTargetView(device,(ID3D11Resource *)back,NULL,&view)));
    ID3D11DeviceContext_OMSetRenderTargets(context,1,&view,NULL);
    ID3D11DeviceContext_ClearRenderTargetView(context,view,color);
    D3D11_TEXTURE2D_DESC desc;ID3D11Texture2D_GetDesc(back,&desc);
    assert(desc.Width==width&&desc.Height==height);
    desc.Usage=D3D11_USAGE_STAGING;desc.BindFlags=0;desc.CPUAccessFlags=D3D11_CPU_ACCESS_READ;desc.MiscFlags=0;
    assert(SUCCEEDED(ID3D11Device_CreateTexture2D(device,&desc,NULL,&staging)));
    ID3D11DeviceContext_CopyResource(context,(ID3D11Resource *)staging,(ID3D11Resource *)back);
    D3D11_MAPPED_SUBRESOURCE mapped;
    assert(SUCCEEDED(ID3D11DeviceContext_Map(context,(ID3D11Resource *)staging,0,D3D11_MAP_READ,0,&mapped)));
    unsigned char *pixel=(unsigned char *)mapped.pData+(height/2)*mapped.RowPitch+(width/2)*4;
    for(int i=0;i<4;i++){int expected=(int)(color[i]*255+0.5f);assert((int)pixel[i]>=expected-1&&(int)pixel[i]<=expected+1);}
    printf("readback rgba=%02x%02x%02x%02x (%ux%u)\n",pixel[0],pixel[1],pixel[2],pixel[3],width,height);
    ID3D11DeviceContext_Unmap(context,(ID3D11Resource *)staging,0);
    ID3D11Texture2D_Release(staging);ID3D11RenderTargetView_Release(view);ID3D11Texture2D_Release(back);
    assert(SUCCEEDED(IDXGISwapChain_Present(swap,1,0)));
}
int main(int argc,char **argv){
    assert(argc>=2);void *lib=dlopen(argv[1],RTLD_NOW|RTLD_LOCAL);
    if(!lib){fprintf(stderr,"%s\n",dlerror());return 1;}
    const void *(*resolve)(const char *,const char *)=dlsym(lib,"peekgame_resolve");assert(resolve);
    uint16_t (WIN *register_fn)(const struct Class *)=(void *)resolve("USER32.dll","RegisterClassExW");
    uintptr_t (WIN *create_fn)(uint32_t,const uint16_t *,const uint16_t *,uint32_t,int32_t,int32_t,int32_t,int32_t,uintptr_t,uintptr_t,uintptr_t,void *)=(void *)resolve("USER32.dll","CreateWindowExW");
    int32_t (WIN *show_fn)(uintptr_t,int32_t)=(void *)resolve("USER32.dll","ShowWindow");
    int32_t (WIN *destroy_fn)(uintptr_t)=(void *)resolve("USER32.dll","DestroyWindow");
    int32_t (WIN *peek_fn)(struct Msg *,uintptr_t,uint32_t,uint32_t,uint32_t)=(void *)resolve("USER32.dll","PeekMessageW");
    intptr_t (WIN *dispatch_fn)(const struct Msg *)=(void *)resolve("USER32.dll","DispatchMessageW");
    default_call=(void *)resolve("USER32.dll","DefWindowProcW");
    uint32_t (*native_fn)(uintptr_t,struct Native *)=dlsym(lib,"peekgame_x11_window");
    typedef HRESULT (WIN *Create)(IDXGIAdapter *,D3D_DRIVER_TYPE,HMODULE,UINT,const D3D_FEATURE_LEVEL *,UINT,UINT,const DXGI_SWAP_CHAIN_DESC *,IDXGISwapChain **,ID3D11Device **,D3D_FEATURE_LEVEL *,ID3D11DeviceContext **);
    Create create=(void *)resolve("d3d11.dll","D3D11CreateDeviceAndSwapChain");
    assert(register_fn&&create_fn&&show_fn&&destroy_fn&&peek_fn&&dispatch_fn&&default_call&&native_fn&&create);
    _Static_assert(sizeof(DXGI_SWAP_CHAIN_DESC)==72,"DXGI swap descriptor");
    const uint16_t name[]={'G','P','U',0};struct Class cls={.size=sizeof cls,.proc=procedure,.name=name};
    assert(register_fn(&cls));uintptr_t hwnd=create_fn(0,name,name,0xcf0000,0,0,640,480,0,0,0,NULL);assert(hwnd);show_fn(hwnd,5);
    DXGI_SWAP_CHAIN_DESC desc={0};desc.BufferDesc.Width=640;desc.BufferDesc.Height=480;desc.BufferDesc.Format=DXGI_FORMAT_R8G8B8A8_UNORM;
    desc.SampleDesc.Count=1;desc.BufferUsage=DXGI_USAGE_RENDER_TARGET_OUTPUT;desc.BufferCount=2;desc.OutputWindow=(HWND)hwnd;desc.Windowed=TRUE;desc.SwapEffect=DXGI_SWAP_EFFECT_FLIP_DISCARD;
    IDXGISwapChain *swap=NULL;ID3D11Device *device=NULL;ID3D11DeviceContext *context=NULL;
    D3D_FEATURE_LEVEL want=D3D_FEATURE_LEVEL_11_0,got=0;
    HRESULT hr=create(NULL,D3D_DRIVER_TYPE_HARDWARE,NULL,0,&want,1,7,&desc,&swap,&device,&got,&context);
    if(argc==3){assert(hr==E_FAIL&&!swap&&!device&&!context);destroy_fn(hwnd);puts("PASS: missing DXVK fails without fake device");return 0;}
    printf("D3D11 create hr=%08x feature=%04x\n",(unsigned)hr,(unsigned)got);fflush(stdout);
    assert(SUCCEEDED(hr)&&swap&&device&&context&&got==want);
    const float colors[3][4]={{0.8f,0.1f,0.1f,1},{0.1f,0.7f,0.2f,1},{0.1f,0.3f,0.9f,1}};
    for(int i=0;i<3;i++)frame(device,context,swap,colors[i],640,480);
    ID3D11DeviceContext_OMSetRenderTargets(context,0,NULL,NULL);
    struct Native native;assert(native_fn(hwnd,&native)==0);
    XResizeWindow(native.display,native.window,321,123);XSync(native.display,False);
    struct Msg message;while(peek_fn(&message,0,0,0,1))dispatch_fn(&message);
    assert(SUCCEEDED(IDXGISwapChain_ResizeBuffers(swap,0,321,123,DXGI_FORMAT_UNKNOWN,0)));
    frame(device,context,swap,colors[0],321,123);
    destroy_fn(hwnd); /* test destruction before the final COM releases, like testgame */
    ID3D11DeviceContext_ClearState(context);ID3D11DeviceContext_Release(context);
    ID3D11Device_Release(device);IDXGISwapChain_Release(swap);
    dlclose(lib);puts("PASS: real DXVK/Vulkan clear, readback, present, resize and teardown without SDL");
}
