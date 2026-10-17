#define COBJMACROS
#include <windows.h>
#include <initguid.h>
#include <mmdeviceapi.h>
#include <audioclient.h>
#include <propsys.h>
#include <functiondiscoverykeys_devpkey.h>
#define CHECK(condition,code) do {if (!(condition)) ExitProcess(code);} while (0)
static int contains(const char *s,const char *needle) {
    for (;*s;s++) {const char *a=s,*b=needle;while(*a&&*a==*b){a++;b++;}if(!*b)return 1;}return 0;
}
void entry(void) {
    CHECK(CoInitializeEx(NULL,COINIT_MULTITHREADED)==S_OK,1);
    IMMDeviceEnumerator *enumerator=NULL;
    CHECK(CoCreateInstance(&CLSID_MMDeviceEnumerator,NULL,CLSCTX_ALL,&IID_IMMDeviceEnumerator,(void**)&enumerator)==S_OK,2);
    IMMDeviceCollection *list=NULL; UINT count=0;
    CHECK(IMMDeviceEnumerator_EnumAudioEndpoints(enumerator,eRender,DEVICE_STATE_ACTIVE,&list)==S_OK,3);
    CHECK(IMMDeviceCollection_GetCount(list,&count)==S_OK&&count==1,4);
    IMMDeviceCollection_Release(list);
    IMMDevice *device=NULL;
    CHECK(IMMDeviceEnumerator_GetDefaultAudioEndpoint(enumerator,eRender,eMultimedia,&device)==S_OK,5);
    IMMEndpoint *endpoint=NULL; IUnknown *identity=NULL; EDataFlow flow=eCapture;
    CHECK(IMMDevice_QueryInterface(device,&IID_IMMEndpoint,(void**)&endpoint)==S_OK,6);
    CHECK(IMMEndpoint_GetDataFlow(endpoint,&flow)==S_OK&&flow==eRender,7);
    CHECK(IMMEndpoint_QueryInterface(endpoint,&IID_IUnknown,(void**)&identity)==S_OK&&(void*)identity==(void*)device,8);
    IUnknown_Release(identity); IMMEndpoint_Release(endpoint);
    IPropertyStore *properties=NULL; PROPVARIANT name;
    CHECK(IMMDevice_OpenPropertyStore(device,STGM_READ,&properties)==S_OK,9);
    CHECK(IPropertyStore_GetValue(properties,&PKEY_Device_FriendlyName,&name)==S_OK&&name.vt==VT_LPWSTR,10);
    CHECK(PropVariantClear(&name)==S_OK&&name.vt==VT_EMPTY,11);
    IPropertyStore_Release(properties);
    IAudioClient *client=NULL;
    CHECK(IMMDevice_Activate(device,&IID_IAudioClient,CLSCTX_ALL,NULL,(void**)&client)==S_OK,12);
    UINT32 size=0;
    CHECK(IAudioClient_GetBufferSize(client,&size)==AUDCLNT_E_NOT_INITIALIZED,13);
    WAVEFORMATEX *format=NULL;
    CHECK(IAudioClient_GetMixFormat(client,&format)==S_OK&&format->wFormatTag==WAVE_FORMAT_IEEE_FLOAT,14);
    int pcm=contains(GetCommandLineA(),"--pcm16");
    if(pcm){format->wFormatTag=WAVE_FORMAT_PCM;format->wBitsPerSample=16;format->nBlockAlign=4;format->nAvgBytesPerSec=format->nSamplesPerSec*4;}
    CHECK(IAudioClient_IsFormatSupported(client,AUDCLNT_SHAREMODE_SHARED,format,NULL)==S_OK,15);
    int adjust=contains(GetCommandLineA(),"--rate-adjust");
    CHECK(IAudioClient_Initialize(client,0,AUDCLNT_STREAMFLAGS_LOOPBACK,500000,0,format,NULL)==AUDCLNT_E_INVALID_STREAM_FLAG,40);
    if(adjust){format->nSamplesPerSec=44100;format->nAvgBytesPerSec=44100*format->nBlockAlign;}
    CHECK(IAudioClient_Initialize(client,AUDCLNT_SHAREMODE_SHARED,AUDCLNT_STREAMFLAGS_EVENTCALLBACK|(adjust?AUDCLNT_STREAMFLAGS_RATEADJUST:0),500000,0,format,NULL)==S_OK,16);
    IAudioClockAdjustment *rate=NULL;
    HRESULT rate_hr=IAudioClient_GetService(client,&IID_IAudioClockAdjustment,(void**)&rate);
    if(adjust){CHECK(rate_hr==S_OK&&rate,41);CHECK(IAudioClockAdjustment_SetSampleRate(rate,0)==E_INVALIDARG,42);CHECK(IAudioClockAdjustment_SetSampleRate(rate,48000.0f)==S_OK,43);IAudioClockAdjustment_Release(rate);}
    else{CHECK(rate_hr==E_NOINTERFACE&&rate==NULL,44);}
    CHECK(IAudioClient_Initialize(client,0,0,500000,0,format,NULL)==AUDCLNT_E_ALREADY_INITIALIZED,17);
    CHECK(IAudioClient_Start(client)==AUDCLNT_E_EVENTHANDLE_NOT_SET,18);
    HANDLE event=CreateEventW(NULL,FALSE,FALSE,NULL),wait=NULL;
    CHECK(event&&DuplicateHandle(GetCurrentProcess(),event,GetCurrentProcess(),&wait,0,FALSE,DUPLICATE_SAME_ACCESS),19);
    CHECK(IAudioClient_SetEventHandle(client,event)==S_OK,20);CloseHandle(event);
    CHECK(IAudioClient_GetBufferSize(client,&size)==S_OK&&size>0,21);
    IAudioRenderClient *render=NULL;IAudioClock *clock=NULL;
    CHECK(IAudioClient_GetService(client,&IID_IAudioRenderClient,(void**)&render)==S_OK,22);
    CHECK(IAudioClient_GetService(client,&IID_IAudioClock,(void**)&clock)==S_OK,23);
    BYTE *buffer=NULL;
    CHECK(IAudioRenderClient_GetBuffer(render,size+1,&buffer)==AUDCLNT_E_BUFFER_TOO_LARGE,24);
    CHECK(IAudioRenderClient_ReleaseBuffer(render,1,0)==AUDCLNT_E_OUT_OF_ORDER,25);
    CHECK(IAudioRenderClient_GetBuffer(render,0,&buffer)==S_OK,26);
    CHECK(IAudioRenderClient_ReleaseBuffer(render,0,0)==S_OK,27);
    CHECK(IAudioClient_Start(client)==S_OK,28);
    UINT64 written=0,frequency=0,position=0,qpc=0;
    CHECK(IAudioClock_GetFrequency(clock,&frequency)==S_OK&&frequency==format->nSamplesPerSec,29);
    while(written<48000) {
        CHECK(WaitForSingleObject(wait,2000)==WAIT_OBJECT_0,30);
        UINT32 padding=0;CHECK(IAudioClient_GetCurrentPadding(client,&padding)==S_OK&&padding<=size,31);
        UINT32 frames=size-padding;if(!frames)continue;
        if(frames>48000-written)frames=(UINT32)(48000-written);
        CHECK(IAudioRenderClient_GetBuffer(render,frames,&buffer)==S_OK,32);
        BYTE *other=NULL;CHECK(IAudioRenderClient_GetBuffer(render,1,&other)==AUDCLNT_E_OUT_OF_ORDER,33);
        for(UINT32 i=0;i<frames;i++) {
            int sign=((written+i)*440/48000)%2 ? 1 : -1;
            if(pcm){((short*)buffer)[i*2]=(short)(sign*4000);((short*)buffer)[i*2+1]=(short)(sign*4000);}
            else {((float*)buffer)[i*2]=sign*0.125f;((float*)buffer)[i*2+1]=sign*0.125f;}
        }
        CHECK(IAudioRenderClient_ReleaseBuffer(render,frames,0)==S_OK,34);written+=frames;
    }
    Sleep(150);
    CHECK(IAudioClock_GetPosition(clock,&position,&qpc)==S_OK&&position>0&&position<=written&&qpc>0,35);
    CHECK(IAudioClient_Reset(client)==AUDCLNT_E_NOT_STOPPED,36);
    CHECK(IAudioClient_Stop(client)==S_OK,37);
    CHECK(IAudioClient_Reset(client)==S_OK,38);
    CHECK(IAudioClient_GetCurrentPadding(client,&size)==S_OK&&size==0,39);
    CoTaskMemFree(format);CloseHandle(wait);
    IAudioClient_Release(client);IAudioRenderClient_Release(render);IAudioClock_Release(clock);
    IMMDevice_Release(device);IMMDeviceEnumerator_Release(enumerator);CoUninitialize();ExitProcess(0);
}
