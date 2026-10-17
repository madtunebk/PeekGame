#include <windows.h>
#include <stdio.h>
#include <io.h>
#include <errno.h>
#define CHECK(x) do {if(!(x))ExitProcess(__LINE__);}while(0)
void entry(void) {
 FILE *f=fopen("large.dat","rb");CHECK(f);
 int fd=_fileno(f);
 CHECK(_lseeki64(fd,123,SEEK_SET)==123);
 CHECK(_filelengthi64(fd)==(((__int64)1<<32)+321));
 CHECK(_lseeki64(fd,0,SEEK_CUR)==123);
 CHECK(fclose(f)==0);
 CHECK(_filelengthi64(fd)==-1 && errno==EBADF);
 CHECK(_filelengthi64(-1)==-1 && errno==EBADF);
 f=fopen("empty.dat","rb");CHECK(f);
 CHECK(_filelengthi64(_fileno(f))==0);
 CHECK(fclose(f)==0);
 ExitProcess(0);
}
