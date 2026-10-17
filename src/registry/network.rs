//! WS2_32 (WSOCK32 resolves here too).
use super::*;
use crate::api::network::{self, events, resolve, sockets, sspi, winhttp};
use crate::api::threading::events as win_events;
exports! {
    export!("crypt32.dll", "CryptBinaryToStringA", "network", "partial", "Base64 and PEM certificate headers, CRLF/NOCR/NOCRLF, character-count buffers", network::crypt_strings::binary_to_string_a),
    export!("crypt32.dll", "CryptBinaryToStringW", "network", "partial", "UTF-16 Base64/PEM certificate output with character-count buffers", network::crypt_strings::binary_to_string_w),

    export!("crypt32.dll", "CertGetCertificateContextProperty", "network", "partial", "real SHA1/MD5/SHA256 certificate hashes; unattached extended properties: CRYPT_E_NOT_FOUND", network::certificates::properties::get),

    export!("crypt32.dll", "CertGetCertificateChain", "network", "partial", "OpenSSL verification against host trust, no online revocation", network::certificates::chain::get),
    export!("crypt32.dll", "CertFreeCertificateChain", "network", "partial", "owned Windows chain contexts", network::certificates::chain::free),
    export!("crypt32.dll", "CertDuplicateCertificateChain", "network", "partial", "reference-counted chain contexts", network::certificates::chain::duplicate),
    export!("crypt32.dll", "CertVerifyCertificateChainPolicy", "network", "partial", "BASE/SSL: trust, signatures, expiry, EKU and DNS/IP names", network::certificates::chain::policy),
    export!("crypt32.dll", "CertOpenStore", "network", "partial", "memory and read-only host ROOT stores", network::certificates::open),
    export!("crypt32.dll", "CertOpenSystemStoreA", "network", "partial", "read-only host ROOT", network::certificates::open_system_a),
    export!("crypt32.dll", "CertOpenSystemStoreW", "network", "partial", "read-only host ROOT", network::certificates::open_system_w),
    export!("crypt32.dll", "CertEnumCertificatesInStore", "network", "partial", "DER and CERT_INFO; consumes previous reference", network::certificates::enumerate),
    export!("crypt32.dll", "CertDuplicateCertificateContext", "network", "partial", "reference-counted contexts", network::certificates::duplicate),
    export!("crypt32.dll", "CertFreeCertificateContext", "network", "partial", "releases a context reference", network::certificates::free),
    export!("crypt32.dll", "CertCloseStore", "network", "partial", "normal/check close; external contexts survive", network::certificates::close),
    export!("crypt32.dll", "CertCreateCertificateContext", "network", "partial", "bounds-checked X.509 metadata", network::certificates::create_context),
    export!("crypt32.dll", "CertAddEncodedCertificateToStore", "network", "partial", "memory stores: new/use-existing/replace/always", network::certificates::add),
    export!(
        "winhttp.dll",
        "WinHttpGetIEProxyConfigForCurrentUser",
        "network",
        "partial",
        "no proxy",
        winhttp::ie_proxy_config
    ),
    export!("winhttp.dll", "WinHttpOpen", "network", "partial", "sync/async sessions; no network transport", winhttp::open),
    export!("winhttp.dll", "WinHttpCloseHandle", "network", "partial", "HANDLE_CLOSING notification", winhttp::close_handle),
    export!("winhttp.dll", "WinHttpConnect", "network", "partial", "connection handles; inherits mode and callback", winhttp::connect),
    export!("winhttp.dll", "WinHttpOpenRequest", "network", "partial", "request handles; HANDLE_CREATED notification", winhttp::open_request),
    export!("winhttp.dll", "WinHttpSetStatusCallback", "network", "partial", "per-handle callback and notification mask, inherited by new children", winhttp::set_status_callback),
    export!("winhttp.dll", "WinHttpSetOption", "network", "partial", "context value and connection options kept per handle; others ERROR_WINHTTP_INVALID_OPTION", winhttp::set_option),
    export!("winhttp.dll", "WinHttpSetTimeouts", "network", "partial", "kept per handle", winhttp::set_timeouts),
    export!("winhttp.dll", "WinHttpAddRequestHeaders", "network", "partial", "accepted onto the request", winhttp::add_request_headers),
    export!("winhttp.dll", "WinHttpSendRequest", "network", "partial", "no transport: ERROR_WINHTTP_CANNOT_CONNECT (async: REQUEST_ERROR callback)", winhttp::send_request),
    export!("winhttp.dll", "WinHttpReceiveResponse", "network", "partial", "no response ever exists: ERROR_WINHTTP_INCORRECT_HANDLE_STATE", winhttp::receive_response),
    export!("winhttp.dll", "WinHttpQueryHeaders", "network", "partial", "no response ever exists: ERROR_WINHTTP_INCORRECT_HANDLE_STATE", winhttp::query_headers),
    export!("winhttp.dll", "WinHttpQueryDataAvailable", "network", "partial", "no response ever exists: ERROR_WINHTTP_INCORRECT_HANDLE_STATE", winhttp::query_data_available),
    export!("winhttp.dll", "WinHttpReadData", "network", "partial", "no response ever exists: ERROR_WINHTTP_INCORRECT_HANDLE_STATE", winhttp::read_data),
    export!("winhttp.dll", "WinHttpWriteData", "network", "partial", "no sent request: ERROR_WINHTTP_INCORRECT_HANDLE_STATE", winhttp::write_data),
    export!(
        "winhttp.dll",
        "WinHttpGetProxyForUrl",
        "network",
        "partial",
        "no proxy auto-configuration",
        winhttp::proxy_for_url
    ),
    export!(
        "winhttp.dll",
        "WinHttpCheckPlatform",
        "network",
        "implemented",
        "returns TRUE",
        winhttp::check_platform
    ),
    export!(
        "secur32.dll",
        "InitSecurityInterfaceW",
        "network",
        "partial",
        "dispatch table without security packages",
        sspi::init_security_interface
    ),
    export!(
        "secur32.dll",
        "InitSecurityInterfaceA",
        "network",
        "partial",
        "dispatch table without security packages",
        sspi::init_security_interface
    ),
    export!(
        "secur32.dll",
        "EnumerateSecurityPackagesW",
        "network",
        "partial",
        "no security packages: SEC_E_UNSUPPORTED_FUNCTION",
        sspi::unsupported
    ),
    export!(
        "secur32.dll",
        "EnumerateSecurityPackagesA",
        "network",
        "partial",
        "no security packages: SEC_E_UNSUPPORTED_FUNCTION",
        sspi::unsupported
    ),
    export!(
        "secur32.dll",
        "LsaEnumerateLogonSessions",
        "network",
        "partial",
        "single interactive logon session LUID",
        sspi::lsa_enumerate_logon_sessions
    ),
    export!(
        "secur32.dll",
        "LsaFreeReturnBuffer",
        "network",
        "partial",
        "frees allocated LSA query buffer",
        sspi::lsa_free_return_buffer
    ),
    export!(
        "secur32.dll",
        "LsaGetLogonSessionData",
        "network",
        "partial",
        "interactive logon session data",
        sspi::lsa_get_logon_session_data
    ),
    export!(
        "secur32.dll",
        "AcquireCredentialsHandleW",
        "network",
        "partial",
        "no security packages: SEC_E_UNSUPPORTED_FUNCTION",
        sspi::unsupported
    ),
    export!(
        "secur32.dll",
        "AcquireCredentialsHandleA",
        "network",
        "partial",
        "no security packages: SEC_E_UNSUPPORTED_FUNCTION",
        sspi::unsupported
    ),
    export!(
        "secur32.dll",
        "InitializeSecurityContextW",
        "network",
        "partial",
        "no security packages: SEC_E_UNSUPPORTED_FUNCTION",
        sspi::unsupported
    ),
    export!(
        "secur32.dll",
        "InitializeSecurityContextA",
        "network",
        "partial",
        "no security packages: SEC_E_UNSUPPORTED_FUNCTION",
        sspi::unsupported
    ),
    export!(
        "secur32.dll",
        "QuerySecurityPackageInfoW",
        "network",
        "partial",
        "no security packages: SEC_E_UNSUPPORTED_FUNCTION",
        sspi::unsupported
    ),
    export!(
        "secur32.dll",
        "QuerySecurityPackageInfoA",
        "network",
        "partial",
        "no security packages: SEC_E_UNSUPPORTED_FUNCTION",
        sspi::unsupported
    ),
    export!(
        "secur32.dll",
        "FreeCredentialsHandle",
        "network",
        "partial",
        "no security packages: SEC_E_UNSUPPORTED_FUNCTION",
        sspi::unsupported
    ),
    export!(
        "secur32.dll",
        "DeleteSecurityContext",
        "network",
        "partial",
        "no security packages: SEC_E_UNSUPPORTED_FUNCTION",
        sspi::unsupported
    ),
    export!(
        "secur32.dll",
        "FreeContextBuffer",
        "network",
        "partial",
        "no security packages: SEC_E_UNSUPPORTED_FUNCTION",
        sspi::unsupported
    ),
    export!(
        "secur32.dll",
        "QueryContextAttributesW",
        "network",
        "partial",
        "no security packages: SEC_E_UNSUPPORTED_FUNCTION",
        sspi::unsupported
    ),
    export!(
        "secur32.dll",
        "EncryptMessage",
        "network",
        "partial",
        "no security packages: SEC_E_UNSUPPORTED_FUNCTION",
        sspi::unsupported
    ),
    export!(
        "secur32.dll",
        "DecryptMessage",
        "network",
        "partial",
        "no security packages: SEC_E_UNSUPPORTED_FUNCTION",
        sspi::unsupported
    ),
    export!(
        "secur32.dll",
        "ApplyControlToken",
        "network",
        "partial",
        "no security packages: SEC_E_UNSUPPORTED_FUNCTION",
        sspi::unsupported
    ),
    export!(
        "ws2_32.dll",
        "WSAStartup",
        "network",
        "partial",
        "versions 1.1 to 2.2",
        network::startup
    ),
    export!(
        "ws2_32.dll",
        "WSACleanup",
        "network",
        "partial",
        "startup count",
        network::cleanup
    ),
    export!(
        "ws2_32.dll",
        "WSAGetLastError",
        "network",
        "partial",
        "the thread's last error",
        network::get_last_error
    ),
    export!(
        "ws2_32.dll",
        "WSASetLastError",
        "network",
        "partial",
        "the thread's last error",
        network::set_last_error
    ),
    export!(
        "ws2_32.dll",
        "htonl",
        "network",
        "partial",
        "complete",
        network::htonl
    ),
    export!(
        "ws2_32.dll",
        "htons",
        "network",
        "partial",
        "complete",
        network::htons
    ),
    export!(
        "ws2_32.dll",
        "ntohl",
        "network",
        "partial",
        "complete",
        network::ntohl
    ),
    export!(
        "ws2_32.dll",
        "ntohs",
        "network",
        "partial",
        "complete",
        network::ntohs
    ),
    export!(
        "ws2_32.dll",
        "socket",
        "network",
        "partial",
        "Linux sockets; AF_INET6 translated",
        sockets::create
    ),
    export!(
        "ws2_32.dll",
        "WSASocketW",
        "network",
        "partial",
        "no protocol info or groups",
        sockets::wsa_socket
    ),
    export!(
        "ws2_32.dll",
        "WSASocketA",
        "network",
        "partial",
        "no protocol info or groups",
        sockets::wsa_socket
    ),
    export!(
        "ws2_32.dll",
        "closesocket",
        "network",
        "partial",
        "complete",
        sockets::close_socket
    ),
    export!(
        "ws2_32.dll",
        "bind",
        "network",
        "partial",
        "complete",
        sockets::bind_socket
    ),
    export!(
        "ws2_32.dll",
        "connect",
        "network",
        "partial",
        "non-blocking connects report WSAEWOULDBLOCK",
        sockets::connect_socket
    ),
    export!(
        "ws2_32.dll",
        "WSAConnect",
        "network",
        "partial",
        "no caller/callee data or QoS",
        sockets::wsa_connect
    ),
    export!(
        "ws2_32.dll",
        "listen",
        "network",
        "partial",
        "complete",
        sockets::listen_socket
    ),
    export!(
        "ws2_32.dll",
        "accept",
        "network",
        "partial",
        "complete",
        sockets::accept_socket
    ),
    export!(
        "ws2_32.dll",
        "getsockname",
        "network",
        "partial",
        "complete",
        sockets::socket_name
    ),
    export!(
        "ws2_32.dll",
        "getpeername",
        "network",
        "partial",
        "complete",
        sockets::peer_name
    ),
    export!(
        "ws2_32.dll",
        "send",
        "network",
        "partial",
        "no SIGPIPE",
        sockets::send_data
    ),
    export!(
        "ws2_32.dll",
        "recv",
        "network",
        "partial",
        "complete",
        sockets::recv_data
    ),
    export!(
        "ws2_32.dll",
        "sendto",
        "network",
        "partial",
        "no SIGPIPE",
        sockets::send_to
    ),
    export!(
        "ws2_32.dll",
        "recvfrom",
        "network",
        "partial",
        "complete",
        sockets::recv_from
    ),
    export!(
        "ws2_32.dll",
        "shutdown",
        "network",
        "partial",
        "complete",
        sockets::shutdown_socket
    ),
    export!(
        "ws2_32.dll",
        "WSASend",
        "network",
        "partial",
        "synchronous only",
        sockets::wsa_send
    ),
    export!(
        "ws2_32.dll",
        "WSASendTo",
        "network",
        "partial",
        "synchronous only",
        sockets::wsa_send_to
    ),
    export!(
        "ws2_32.dll",
        "WSARecv",
        "network",
        "partial",
        "synchronous only",
        sockets::wsa_recv
    ),
    export!(
        "ws2_32.dll",
        "WSARecvFrom",
        "network",
        "partial",
        "synchronous only",
        sockets::wsa_recv_from
    ),
    export!(
        "ws2_32.dll",
        "ioctlsocket",
        "network",
        "partial",
        "FIONBIO, FIONREAD",
        sockets::ioctl_socket
    ),
    export!(
        "ws2_32.dll",
        "setsockopt",
        "network",
        "partial",
        "SOL_SOCKET, IP, IPv6 and TCP options mapped to Linux",
        sockets::set_option
    ),
    export!(
        "ws2_32.dll",
        "getsockopt",
        "network",
        "partial",
        "SOL_SOCKET, IP, IPv6 and TCP options mapped to Linux",
        sockets::get_option
    ),
    export!(
        "ws2_32.dll",
        "select",
        "network",
        "partial",
        "poll over the three sets",
        sockets::select
    ),
    export!(
        "ws2_32.dll",
        "__WSAFDIsSet",
        "network",
        "partial",
        "complete",
        sockets::fd_is_set
    ),
    export!(
        "ws2_32.dll",
        "WSAIoctl",
        "network",
        "partial",
        "base and provider handle queries",
        sockets::wsa_ioctl
    ),
    export!(
        "ws2_32.dll",
        "gethostname",
        "network",
        "partial",
        "complete",
        sockets::get_host_name
    ),
    export!(
        "ws2_32.dll",
        "GetHostNameW",
        "network",
        "partial",
        "complete",
        sockets::get_host_name_w
    ),
    export!(
        "ws2_32.dll",
        "getaddrinfo",
        "network",
        "partial",
        "host resolver; Windows layout and flags",
        resolve::get_addr_info
    ),
    export!(
        "ws2_32.dll",
        "GetAddrInfoW",
        "network",
        "partial",
        "host resolver; Windows layout and flags",
        resolve::get_addr_info_w
    ),
    export!(
        "ws2_32.dll",
        "freeaddrinfo",
        "network",
        "partial",
        "lists from getaddrinfo",
        resolve::free_addr_info
    ),
    export!(
        "ws2_32.dll",
        "FreeAddrInfoW",
        "network",
        "partial",
        "lists from GetAddrInfoW",
        resolve::free_addr_info
    ),
    export!(
        "ws2_32.dll",
        "getnameinfo",
        "network",
        "partial",
        "host resolver",
        resolve::get_name_info
    ),
    export!(
        "ws2_32.dll",
        "GetNameInfoW",
        "network",
        "partial",
        "host resolver",
        resolve::get_name_info_w
    ),
    export!(
        "ws2_32.dll",
        "inet_addr",
        "network",
        "partial",
        "dotted IPv4",
        resolve::inet_addr
    ),
    export!(
        "ws2_32.dll",
        "inet_ntoa",
        "network",
        "partial",
        "per-thread buffer",
        resolve::inet_ntoa
    ),
    export!(
        "ws2_32.dll",
        "inet_pton",
        "network",
        "partial",
        "IPv4 and IPv6",
        resolve::inet_pton_a
    ),
    export!(
        "ws2_32.dll",
        "InetPtonW",
        "network",
        "partial",
        "IPv4 and IPv6",
        resolve::inet_pton_w
    ),
    export!(
        "ws2_32.dll",
        "inet_ntop",
        "network",
        "partial",
        "IPv4 and IPv6",
        resolve::inet_ntop_a
    ),
    export!(
        "ws2_32.dll",
        "InetNtopW",
        "network",
        "partial",
        "IPv4 and IPv6",
        resolve::inet_ntop_w
    ),
    export!(
        "ws2_32.dll",
        "gethostbyname",
        "network",
        "partial",
        "IPv4 through getaddrinfo",
        resolve::get_host_by_name
    ),
    export!(
        "ws2_32.dll",
        "getprotobyname",
        "network",
        "partial",
        "host protocol database",
        resolve::get_protocol_by_name
    ),
    export!(
        "ws2_32.dll",
        "getprotobynumber",
        "network",
        "partial",
        "host protocol database",
        resolve::get_protocol_by_number
    ),
    export!(
        "ws2_32.dll",
        "getservbyname",
        "network",
        "partial",
        "host services database",
        resolve::get_service_by_name
    ),
    export!(
        "ws2_32.dll",
        "getservbyport",
        "network",
        "partial",
        "host services database",
        resolve::get_service_by_port
    ),
    export!(
        "ws2_32.dll",
        "WSAEventSelect",
        "network",
        "partial",
        "FD_READ/WRITE/OOB/ACCEPT/CONNECT/CLOSE via a poller thread",
        events::event_select
    ),
    export!(
        "ws2_32.dll",
        "WSAEnumNetworkEvents",
        "network",
        "partial",
        "complete",
        events::enum_network_events
    ),
    export!(
        "ws2_32.dll",
        "WSACreateEvent",
        "network",
        "partial",
        "manual-reset event",
        events::create_event
    ),
    export!(
        "ws2_32.dll",
        "WSACloseEvent",
        "network",
        "partial",
        "complete",
        events::close_event
    ),
    export!(
        "ws2_32.dll",
        "WSASetEvent",
        "network",
        "partial",
        "complete",
        win_events::set
    ),
    export!(
        "ws2_32.dll",
        "WSAResetEvent",
        "network",
        "partial",
        "complete",
        win_events::reset
    ),
    export!(
        "ws2_32.dll",
        "WSAWaitForMultipleEvents",
        "network",
        "partial",
        "WaitForMultipleObjectsEx",
        events::wait_for_multiple_events
    ),
    export!(
        "iphlpapi.dll",
        "GetAdaptersInfo",
        "network",
        "partial",
        "single Ethernet IP_ADAPTER_INFO with host MAC address",
        network::get_adapters_info
    ),
    export!(
        "iphlpapi.dll",
        "GetBestInterface",
        "network",
        "partial",
        "returns adapter index 1 matching GetAdaptersInfo",
        network::get_best_interface
    ),
}
