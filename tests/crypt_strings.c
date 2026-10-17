#include <windows.h>
#include <wincrypt.h>
#define CHECK(x) do{if(!(x))ExitProcess(__LINE__);}while(0)
void entry(void){
 BYTE input[49];for(unsigned i=0;i<49;i++)input[i]=(BYTE)i;
 DWORD flags[]={CRYPT_STRING_BASE64,CRYPT_STRING_BASE64HEADER,CRYPT_STRING_BASE64|CRYPT_STRING_NOCR,CRYPT_STRING_BASE64|CRYPT_STRING_NOCRLF,CRYPT_STRING_BASE64HEADER|CRYPT_STRING_NOCRLF};
 DWORD lengths[]={0,1,2,3,48,49};char out[256];WCHAR wide[256];DWORD done;
 for(unsigned a=0;a<5;a++)for(unsigned b=0;b<6;b++){
  DWORD needed=0;CHECK(CryptBinaryToStringA(input,lengths[b],flags[a],NULL,&needed)&&needed<256);
  DWORD small=needed-1;out[0]='!';CHECK(!CryptBinaryToStringA(input,lengths[b],flags[a],out,&small)&&GetLastError()==ERROR_MORE_DATA&&small==needed&&out[0]=='!');
  DWORD n=needed;CHECK(CryptBinaryToStringA(input,lengths[b],flags[a],out,&n)&&n+1==needed&&out[n]==0);
  DWORD nw=256;CHECK(CryptBinaryToStringW(input,lengths[b],flags[a],wide,&nw)&&nw==n&&wide[nw]==0);
  for(DWORD i=0;i<n;i++)CHECK(wide[i]==(unsigned char)out[i]);
  CHECK(WriteFile(GetStdHandle(STD_OUTPUT_HANDLE),&n,4,&done,NULL)&&done==4);
  CHECK(WriteFile(GetStdHandle(STD_OUTPUT_HANDLE),out,n,&done,NULL)&&done==n);
 }
 DWORD n=256;CHECK(!CryptBinaryToStringA(NULL,1,1,out,&n)&&GetLastError()==ERROR_INVALID_PARAMETER);
 CHECK(!CryptBinaryToStringA(input,1,1,out,NULL)&&GetLastError()==ERROR_INVALID_PARAMETER);
 CHECK(!CryptBinaryToStringA(input,1,0x10000001,out,&n)&&GetLastError()==ERROR_INVALID_PARAMETER);
 ExitProcess(0);
}
