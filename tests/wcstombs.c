#include <windows.h>
#include <stdlib.h>
#include <errno.h>
#define CHECK(x) do{if(!(x))ExitProcess(__LINE__);}while(0)
void entry(void){
 const wchar_t text[]={0x41,0xe9,0xd83d,0xde00,0};
 char out[16];for(int i=0;i<16;i++)out[i]=0x55;
 CHECK(wcstombs(NULL,text,0)==7);
 CHECK(wcstombs(out,text,0)==0&&out[0]==0x55);
 CHECK(wcstombs(out,text,2)==1&&out[0]=='A'&&out[1]==0x55);
 CHECK(wcstombs(out,text,7)==7&&out[7]==0x55);
 CHECK((unsigned char)out[1]==0xc3&&(unsigned char)out[2]==0xa9&&(unsigned char)out[3]==0xf0);
 CHECK(wcstombs(out,text,8)==7&&out[7]==0);
 const wchar_t bad[]={0xd800,0},bad2[]={0xdc00,0};
 CHECK(wcstombs(out,bad,16)==(size_t)-1&&errno==EILSEQ);
 CHECK(wcstombs(NULL,bad2,0)==(size_t)-1&&errno==EILSEQ);
 CHECK(wcstombs(out,NULL,16)==(size_t)-1&&errno==EINVAL);
 CHECK(wcstombs(out,L"",1)==0&&out[0]==0);
 ExitProcess(0);
}
