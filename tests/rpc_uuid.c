#include <windows.h>
#include <rpc.h>

void entry(void) {
    UUID uuid;
    RPC_STATUS status = UuidFromStringA((RPC_CSTR)"a40d1cbe-23ad-43c7-9277-8bfd0951030f", &uuid);
    if (status != RPC_S_OK || uuid.Data1 != 0xa40d1cbe || uuid.Data2 != 0x23ad ||
        uuid.Data3 != 0x43c7 || uuid.Data4[0] != 0x92 || uuid.Data4[1] != 0x77 ||
        uuid.Data4[7] != 0x0f) ExitProcess(1);
    uuid.Data1 = 0x12345678;
    status = UuidFromStringA((RPC_CSTR)"a40d1cbe-23ad-43c7-9277-8bfd0951030z", &uuid);
    if (status != RPC_S_INVALID_STRING_UUID || uuid.Data1 != 0x12345678) ExitProcess(2);
    status = UuidFromStringA(NULL, &uuid);
    if (status != RPC_S_OK || uuid.Data1 != 0 || uuid.Data2 != 0 || uuid.Data3 != 0 ||
        uuid.Data4[0] != 0 || uuid.Data4[7] != 0) ExitProcess(3);
    ExitProcess(0);
}
