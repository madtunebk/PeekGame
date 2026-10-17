# IP Helper adapter enumeration

DOOM Eternal passed the WinHTTP runtime config response and then stopped on
`IPHLPAPI.DLL!GetAdaptersAddresses` at EXE RVA `0x1ac4632`.

PeekGame now enumerates host interfaces through `getifaddrs` and `/sys/class/net`,
returning real interface names, indices, MAC addresses, MTU, state and the first
IPv4 or IPv6 unicast address. The Windows x64 layout was checked against
mingw-w64 `iptypes.h`: `IP_ADAPTER_ADDRESSES` is 448 bytes and
`IP_ADAPTER_UNICAST_ADDRESS` is 64 bytes. The usual size query returns
`ERROR_BUFFER_OVERFLOW` with the required buffer length. The implementation
currently reports the first address of the requested family per interface.

`python3 tests/test_iphlpapi_adapters.py` passes: a Windows PE program loaded
under PeekGame checks buffer sizing, adapter pointers and unicast structure.
The owner has not yet rerun DOOM with this build.
