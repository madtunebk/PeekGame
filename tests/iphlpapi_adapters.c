#include <winsock2.h>
#include <iphlpapi.h>
#include <windows.h>
static unsigned char storage[32768];
void entry(void) {
    ULONG size = 0;
    if (GetAdaptersAddresses(AF_UNSPEC, 0, NULL, NULL, &size) != ERROR_BUFFER_OVERFLOW) ExitProcess(1);
    if (size == 0 || size > sizeof(storage)) ExitProcess(2);
    ULONG capacity = sizeof(storage);
    PIP_ADAPTER_ADDRESSES first = (PIP_ADAPTER_ADDRESSES)storage;
    if (GetAdaptersAddresses(AF_UNSPEC, 0, NULL, first, &capacity) != NO_ERROR) ExitProcess(3);
    if (capacity != size) ExitProcess(4);
    int count = 0;
    for (PIP_ADAPTER_ADDRESSES p = first; p; p = p->Next) {
        if (++count > 32) ExitProcess(5);
        if (p->Length != sizeof(IP_ADAPTER_ADDRESSES) || !p->IfIndex || !p->AdapterName) ExitProcess(6);
        if (!p->FriendlyName || !p->FirstUnicastAddress) ExitProcess(7);
        if (!p->DnsSuffix || !p->FirstDnsServerAddress) ExitProcess(18);
        if (p->FirstDnsServerAddress->Length != sizeof(IP_ADAPTER_DNS_SERVER_ADDRESS_XP) ||
            !p->FirstDnsServerAddress->Address.lpSockaddr ||
            p->FirstDnsServerAddress->Address.lpSockaddr->sa_family != AF_INET) ExitProcess(19);
        if (p->FirstUnicastAddress->Length != sizeof(IP_ADAPTER_UNICAST_ADDRESS)) ExitProcess(8);
        if (!p->FirstUnicastAddress->Address.lpSockaddr) ExitProcess(9);
        int family = p->FirstUnicastAddress->Address.lpSockaddr->sa_family;
        if (family != AF_INET && family != AF_INET6) ExitProcess(10);
        /* PlayFab Party only uses addresses whose duplicate-address detection is done. */
        for (PIP_ADAPTER_UNICAST_ADDRESS u = p->FirstUnicastAddress; u; u = u->Next) {
            if (u->Length != sizeof(IP_ADAPTER_UNICAST_ADDRESS) || !u->Address.lpSockaddr) ExitProcess(12);
            if (u->DadState != IpDadStatePreferred || u->ValidLifetime != 0xffffffff || u->PreferredLifetime != 0xffffffff) ExitProcess(13);
            int f = u->Address.lpSockaddr->sa_family;
            if (u->Address.iSockaddrLength != (f == AF_INET ? (int)sizeof(struct sockaddr_in) : 28)) ExitProcess(14);
            if (f == AF_INET && !(p->Ipv4Enabled)) ExitProcess(15);
            if (f == AF_INET6 && !(p->Ipv6Enabled)) ExitProcess(16);
        }
    }
    /* The flags PartyWin passes: skip anycast and multicast. */
    size = 0;
    if (GetAdaptersAddresses(AF_UNSPEC, GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST, NULL, NULL, &size) != ERROR_BUFFER_OVERFLOW || !size) ExitProcess(17);
    ExitProcess(count > 0 ? 0 : 11);
}
