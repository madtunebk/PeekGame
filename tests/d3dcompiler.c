#define COBJMACROS
#include <windows.h>
#include <d3dcompiler.h>
#define CHECK(x) do{if(!(x))ExitProcess(__LINE__);}while(0)
void entry(void){
 HMODULE module=LoadLibraryA("d3dcompiler_47.dll");CHECK(module);
 pD3DCompile compile=(pD3DCompile)GetProcAddress(module,"D3DCompile");CHECK(compile);
 const char source[]="float4 main():SV_Target{return float4(1,0,0,1);}";
 ID3DBlob *code=NULL,*errors=NULL;
 HRESULT hr=compile(source,sizeof(source)-1,"fixture",NULL,NULL,"main","ps_5_0",0,0,&code,&errors);
 CHECK(SUCCEEDED(hr)&&code&&ID3D10Blob_GetBufferSize(code)>32);
 const char *bytes=ID3D10Blob_GetBufferPointer(code);CHECK(bytes[0]=='D'&&bytes[1]=='X'&&bytes[2]=='B'&&bytes[3]=='C');
 ID3D10Blob_Release(code);if(errors)ID3D10Blob_Release(errors);
 ExitProcess(0);
}
