/* Event-based overlapped UDP as PlayFab Party uses it: a dual-stack IPv6 socket,
   WSARecvFrom/WSASendTo with OVERLAPPED.hEvent, WSAGetOverlappedResult, immediate
   completion, datagram truncation and cancellation by closesocket. */
#define _WIN32_WINNT 0x0A00
#include <winsock2.h>
#include <ws2tcpip.h>
#include <windows.h>
/* Missing from the mingw-w64 headers. */
INT WSAAPI GetAddrInfoExOverlappedResult(LPOVERLAPPED overlapped);
#define CHECK(x) do{if(!(x))ExitProcess(__LINE__);}while(0)
static int same(const char*a,const char*b,int n){for(int i=0;i<n;i++)if(a[i]!=b[i])return 0;return 1;}
void entry(void){
    WSADATA wsa;CHECK(WSAStartup(0x202,&wsa)==0);
    SOCKET party=socket(AF_INET6,SOCK_DGRAM,IPPROTO_UDP);CHECK(party!=INVALID_SOCKET);
    u_long nonblocking=1;CHECK(ioctlsocket(party,FIONBIO,&nonblocking)==0);
    int off=0;CHECK(setsockopt(party,IPPROTO_IPV6,IPV6_V6ONLY,(char*)&off,4)==0);
    struct sockaddr_in6 any={0};any.sin6_family=AF_INET6;CHECK(bind(party,(struct sockaddr*)&any,sizeof(any))==0);
    struct sockaddr_in6 local;int local_size=sizeof(local);CHECK(getsockname(party,(struct sockaddr*)&local,&local_size)==0&&local.sin6_port);
    SOCKET peer=socket(AF_INET,SOCK_DGRAM,IPPROTO_UDP);CHECK(peer!=INVALID_SOCKET);
    struct sockaddr_in target={0};target.sin_family=AF_INET;target.sin_port=local.sin6_port;target.sin_addr.s_addr=htonl(INADDR_LOOPBACK);

    /* 1. pending receive, completed later by a send */
    char buffer[64];WSABUF in={sizeof(buffer),buffer};
    struct sockaddr_storage from;int from_size=sizeof(from);DWORD got=0,flags=0;
    WSAOVERLAPPED ov={0};ov.hEvent=WSACreateEvent();CHECK(ov.hEvent!=WSA_INVALID_EVENT);
    CHECK(WSARecvFrom(party,&in,1,&got,&flags,(struct sockaddr*)&from,&from_size,&ov,NULL)==SOCKET_ERROR&&WSAGetLastError()==WSA_IO_PENDING);
    CHECK(!WSAGetOverlappedResult(party,&ov,&got,FALSE,&flags)&&WSAGetLastError()==WSA_IO_INCOMPLETE);
    CHECK(WaitForSingleObject(ov.hEvent,0)==WAIT_TIMEOUT);
    static char probe[]="qos-probe-1";WSABUF out={sizeof(probe),probe};
    WSAOVERLAPPED sov={0};sov.hEvent=WSACreateEvent();DWORD sent=0;
    CHECK(WSASendTo(peer,&out,1,&sent,0,(struct sockaddr*)&target,sizeof(target),&sov,NULL)==0&&sent==sizeof(probe));
    CHECK(WaitForSingleObject(sov.hEvent,0)==WAIT_OBJECT_0);
    CHECK(WSAGetOverlappedResult(peer,&sov,&sent,FALSE,&flags)&&sent==sizeof(probe));
    CHECK(WaitForSingleObject(ov.hEvent,5000)==WAIT_OBJECT_0);
    CHECK(WSAGetOverlappedResult(party,&ov,&got,FALSE,&flags)&&got==sizeof(probe)&&flags==0);
    CHECK(same(buffer,probe,sizeof(probe)));
    struct sockaddr_in6 *mapped=(struct sockaddr_in6*)&from;
    CHECK(from_size==sizeof(struct sockaddr_in6)&&mapped->sin6_family==AF_INET6&&IN6_IS_ADDR_V4MAPPED(&mapped->sin6_addr));

    /* 2. data already waiting: completes at once, event signalled */
    CHECK(sendto(peer,"now",3,0,(struct sockaddr*)&target,sizeof(target))==3);
    Sleep(50);WSAResetEvent(ov.hEvent);from_size=sizeof(from);got=0;
    CHECK(WSARecvFrom(party,&in,1,&got,&flags,(struct sockaddr*)&from,&from_size,&ov,NULL)==0&&got==3&&same(buffer,"now",3));
    CHECK(WaitForSingleObject(ov.hEvent,0)==WAIT_OBJECT_0);

    /* 3. truncated datagram: WSAEMSGSIZE through the overlapped result */
    char tiny[4];WSABUF small={sizeof(tiny),tiny};WSAResetEvent(ov.hEvent);
    CHECK(WSARecvFrom(party,&small,1,&got,&flags,NULL,NULL,&ov,NULL)==SOCKET_ERROR&&WSAGetLastError()==WSA_IO_PENDING);
    CHECK(sendto(peer,"0123456789",10,0,(struct sockaddr*)&target,sizeof(target))==10);
    CHECK(WaitForSingleObject(ov.hEvent,5000)==WAIT_OBJECT_0);
    CHECK(!WSAGetOverlappedResult(party,&ov,&got,TRUE,&flags)&&WSAGetLastError()==WSAEMSGSIZE&&got==4&&same(tiny,"0123",4));

    /* 4. closesocket cancels a pending receive */
    WSAResetEvent(ov.hEvent);
    CHECK(WSARecvFrom(party,&in,1,&got,&flags,NULL,NULL,&ov,NULL)==SOCKET_ERROR&&WSAGetLastError()==WSA_IO_PENDING);
    CHECK(closesocket(party)==0);
    CHECK(WaitForSingleObject(ov.hEvent,5000)==WAIT_OBJECT_0);
    CHECK(!WSAGetOverlappedResult(party,&ov,&got,FALSE,&flags)&&WSAGetLastError()==WSA_OPERATION_ABORTED);

    /* completion routines stay unsupported */
    SOCKET other=socket(AF_INET,SOCK_DGRAM,IPPROTO_UDP);
    CHECK(WSARecv(other,&in,1,&got,&flags,&ov,(LPWSAOVERLAPPED_COMPLETION_ROUTINE)1)==SOCKET_ERROR&&WSAGetLastError()==WSAEOPNOTSUPP);
    closesocket(other);closesocket(peer);

    /* GetAddrInfoExW: synchronous, asynchronous with an event, failure status */
    ADDRINFOEXW hints={0};hints.ai_family=AF_INET;hints.ai_socktype=SOCK_DGRAM;hints.ai_protocol=IPPROTO_UDP;
    ADDRINFOEXW *list=NULL;
    CHECK(GetAddrInfoExW(L"localhost",NULL,NS_ALL,NULL,&hints,&list,NULL,NULL,NULL,NULL)==0&&list);
    CHECK(list->ai_family==AF_INET&&list->ai_addrlen==sizeof(struct sockaddr_in)&&((struct sockaddr_in*)list->ai_addr)->sin_addr.s_addr==htonl(INADDR_LOOPBACK));
    FreeAddrInfoExW(list);list=NULL;
    OVERLAPPED dns={0};dns.hEvent=CreateEventW(NULL,TRUE,FALSE,NULL);HANDLE cancel=NULL;
    CHECK(GetAddrInfoExW(L"localhost",NULL,NS_ALL,NULL,&hints,&list,NULL,&dns,NULL,&cancel)==WSA_IO_PENDING&&cancel);
    CHECK(WaitForSingleObject(dns.hEvent,10000)==WAIT_OBJECT_0);
    CHECK(GetAddrInfoExOverlappedResult(&dns)==0&&list&&list->ai_family==AF_INET);
    FreeAddrInfoExW(list);list=NULL;ResetEvent(dns.hEvent);
    CHECK(GetAddrInfoExW(L"peekgame-no-such-host.invalid",NULL,NS_ALL,NULL,&hints,&list,NULL,&dns,NULL,&cancel)==WSA_IO_PENDING);
    CHECK(WaitForSingleObject(dns.hEvent,20000)==WAIT_OBJECT_0);
    CHECK(GetAddrInfoExOverlappedResult(&dns)==WSAHOST_NOT_FOUND&&!list);
    CHECK(GetAddrInfoExW(L"localhost",NULL,NS_ALL,NULL,&hints,&list,NULL,&dns,(LPLOOKUPSERVICE_COMPLETION_ROUTINE)1,&cancel)==WSAEOPNOTSUPP);
    CloseHandle(dns.hEvent);
    WSACloseEvent(ov.hEvent);WSACloseEvent(sov.hEvent);WSACleanup();
    ExitProcess(0);
}
