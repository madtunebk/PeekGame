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
        if (p->FirstUnicastAddress->Length != sizeof(IP_ADAPTER_UNICAST_ADDRESS)) ExitProcess(8);
        if (!p->FirstUnicastAddress->Address.lpSockaddr) ExitProcess(9);
        int family = p->FirstUnicastAddress->Address.lpSockaddr->sa_family;
        if (family != AF_INET && family != AF_INET6) ExitProcess(10);
    }
    ExitProcess(count > 0 ? 0 : 11);
}
