#include <windows.h>
#include <stdio.h>
#include <errno.h>
#define CHECK(x) do {if(!(x)) ExitProcess(__LINE__);}while(0)
void entry(void) {
 CHECK(_getmaxstdio()==512);
 CHECK(_setmaxstdio(8192)==8192 && _getmaxstdio()==8192);
 CHECK(_setmaxstdio(19)==-1 && errno==EINVAL && _getmaxstdio()==8192);
 CHECK(_setmaxstdio(8193)==-1 && errno==EINVAL);
 CHECK(_setmaxstdio(20)==20);
 FILE *files[18];
 for(int i=0;i<17;i++){files[i]=fopen("stdio-limit.dat","a+b");CHECK(files[i]);}
 CHECK(!fopen("stdio-limit.dat","a+b") && errno==EMFILE);
 CHECK(_setmaxstdio(21)==21);
 files[17]=fopen("stdio-limit.dat","a+b");CHECK(files[17]);
 CHECK(_setmaxstdio(20)==-1 && errno==EINVAL && _getmaxstdio()==21);
 CHECK(fclose(files[17])==0);
 CHECK(_setmaxstdio(20)==20);
 CHECK(fclose(files[0])==0);
 files[0]=fopen("stdio-limit.dat","a+b");CHECK(files[0]);
 for(int i=0;i<17;i++)CHECK(fclose(files[i])==0);
 CHECK(_setmaxstdio(512)==512);
 ExitProcess(0);
}
