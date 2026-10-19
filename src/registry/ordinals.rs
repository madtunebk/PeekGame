//! Exports programs import by ordinal from DLLs PeekGame implements. These numbers
//! are fixed parts of the Windows ABI (WS2_32 and WSOCK32 keep the Winsock 1.1
//! numbering, except that WSOCK32 has inet_addr/inet_ntoa/ioctlsocket at 10/11/12 where
//! WS2_32 has ioctlsocket/inet_addr/inet_ntoa; OLEAUT32's BSTR, VARIANT and SAFEARRAY
//! entry points), the same for every program.
const WS2_32: &[(u16, &str)] = &[
    (1, "accept"),
    (2, "bind"),
    (3, "closesocket"),
    (4, "connect"),
    (5, "getpeername"),
    (6, "getsockname"),
    (7, "getsockopt"),
    (8, "htonl"),
    (9, "htons"),
    (10, "ioctlsocket"),
    (11, "inet_addr"),
    (12, "inet_ntoa"),
    (13, "listen"),
    (14, "ntohl"),
    (15, "ntohs"),
    (16, "recv"),
    (17, "recvfrom"),
    (18, "select"),
    (19, "send"),
    (20, "sendto"),
    (21, "setsockopt"),
    (22, "shutdown"),
    (23, "socket"),
    (51, "gethostbyaddr"),
    (52, "gethostbyname"),
    (53, "getprotobyname"),
    (54, "getprotobynumber"),
    (55, "getservbyname"),
    (56, "getservbyport"),
    (57, "gethostname"),
    (101, "WSAAsyncSelect"),
    (102, "WSAAsyncGetHostByAddr"),
    (103, "WSAAsyncGetHostByName"),
    (104, "WSAAsyncGetProtoByNumber"),
    (105, "WSAAsyncGetProtoByName"),
    (106, "WSAAsyncGetServByPort"),
    (107, "WSAAsyncGetServByName"),
    (108, "WSACancelAsyncRequest"),
    (109, "WSASetBlockingHook"),
    (110, "WSAUnhookBlockingHook"),
    (111, "WSAGetLastError"),
    (112, "WSASetLastError"),
    (113, "WSACancelBlockingCall"),
    (114, "WSAIsBlocking"),
    (115, "WSAStartup"),
    (116, "WSACleanup"),
    (151, "__WSAFDIsSet"),
];
const OLEAUT32: &[(u16, &str)] = &[
    (2, "SysAllocString"),
    (3, "SysReAllocString"),
    (4, "SysAllocStringLen"),
    (5, "SysReAllocStringLen"),
    (6, "SysFreeString"),
    (7, "SysStringLen"),
    (8, "VariantInit"),
    (9, "VariantClear"),
    (10, "VariantCopy"),
    (11, "VariantCopyInd"),
    (12, "VariantChangeType"),
    (15, "SafeArrayCreate"),
    (16, "SafeArrayDestroy"),
    (17, "SafeArrayGetDim"),
    (18, "SafeArrayGetElemsize"),
    (19, "SafeArrayGetUBound"),
    (20, "SafeArrayGetLBound"),
    (21, "SafeArrayLock"),
    (22, "SafeArrayUnlock"),
    (23, "SafeArrayAccessData"),
    (24, "SafeArrayUnaccessData"),
    (25, "SafeArrayGetElement"),
    (26, "SafeArrayPutElement"),
    (27, "SafeArrayCopy"),
    (77, "SafeArrayGetVartype"),
    (149, "SysStringByteLen"),
    (150, "SysAllocStringByteLen"),
    (159, "LoadTypeLib"),
    (160, "LoadRegTypeLib"),
    (161, "RegisterTypeLib"),
    (162, "QueryPathOfRegTypeLib"),
    (181, "LoadTypeLibEx"),
    (184, "SystemTimeToVariantTime"),
    (185, "VariantTimeToSystemTime"),
    (186, "UnRegisterTypeLib"),
    (200, "GetErrorInfo"),
    (201, "SetErrorInfo"),
    (202, "CreateErrorInfo"),
    (277, "VarCmp"),
    (314, "VarBstrCmp"),
    (411, "SafeArrayCreateVector"),
    (440, "RegisterTypeLibForUser"),
    (441, "UnRegisterTypeLibForUser"),
];
const D3D12: &[(u16, &str)] = &[
    (101, "D3D12CreateDevice"),
    (102, "D3D12GetDebugInterface"),
    (103, "D3D12CreateRootSignatureDeserializer"),
    (104, "D3D12SerializeRootSignature"),
    (105, "D3D12EnableExperimentalFeatures"),
];
/// XINPUT1_3/1_4: 1 is DllMain; 100 is the unnamed XInputGetStateEx.
const XINPUT: &[(u16, &str)] = &[
    (2, "XInputGetState"),
    (3, "XInputSetState"),
    (4, "XInputGetCapabilities"),
    (5, "XInputEnable"),
    (6, "XInputGetDSoundAudioDeviceGuids"),
    (7, "XInputGetBatteryInformation"),
    (8, "XInputGetKeystroke"),
    (10, "XInputGetAudioDeviceIds"),
    (100, "XInputGetStateEx"),
];
const DSOUND: &[(u16, &str)] = &[
    (1, "DirectSoundCreate"),
    (2, "DirectSoundEnumerateA"),
    (3, "DirectSoundEnumerateW"),
    (6, "DirectSoundCaptureCreate"),
    (7, "DirectSoundCaptureEnumerateA"),
    (8, "DirectSoundCaptureEnumerateW"),
    (11, "DirectSoundCreate8"),
    (12, "DirectSoundCaptureCreate8"),
];
const SHELL32: &[(u16, &str)] = &[
    (2, "SHChangeNotifyRegister"),
    (4, "SHChangeNotifyDeregister"),
    (680, "IsUserAnAdmin"),
];
const SHLWAPI: &[(u16, &str)] = &[
    (12, "SHAllocShared"),
    (219, "QISearch"),
    (437, "IsOS"),
];
const UXTHEME: &[(u16, &str)] = &[
    (47, "DrawThemeBackground"),
];
const WINSPOOL: &[(u16, &str)] = &[
    (203, "ConnectToPrinterDlg"),
];
const COMCTL32: &[(u16, &str)] = &[
    (345, "TaskDialogIndirect"),
    (410, "SetWindowSubclass"),
    (412, "RemoveWindowSubclass"),
    (413, "DefSubclassProc"),
];
/// The name of export `ordinal` of `dll` (canonical, without ".dll").
pub(super) fn name(dll: &str, ordinal: u16) -> Option<&'static str> {
    let table = match dll {
        "ws2_32" => WS2_32,
        "wsock32" => {
            // Same numbering as WS2_32 except 10..=12 (wsock32.spec/ws2_32.spec).
            return match ordinal {
                10 => Some("inet_addr"),
                11 => Some("inet_ntoa"),
                12 => Some("ioctlsocket"),
                _ => WS2_32.iter().find(|(o, _)| *o == ordinal).map(|(_, n)| *n),
            };
        }
        "oleaut32" => OLEAUT32,
        "d3d12" => D3D12,
        "xinput1_3" | "xinput1_4" => XINPUT,
        "dsound" => DSOUND,
        "shell32" => SHELL32,
        "shlwapi" => SHLWAPI,
        "uxtheme" => UXTHEME,
        "winspool.drv" | "winspool" => WINSPOOL,
        "comctl32" => COMCTL32,
        _ => return None,
    };
    table.iter().find(|(o, _)| *o == ordinal).map(|(_, n)| *n)
}

#[cfg(test)]
mod tests {
    #[test]
    fn winsock_ordinals_differ_at_10_to_12() {
        assert_eq!(super::name("ws2_32", 10), Some("ioctlsocket"));
        assert_eq!(super::name("ws2_32", 11), Some("inet_addr"));
        assert_eq!(super::name("ws2_32", 12), Some("inet_ntoa"));
        assert_eq!(super::name("wsock32", 10), Some("inet_addr"));
        assert_eq!(super::name("wsock32", 11), Some("inet_ntoa"));
        assert_eq!(super::name("wsock32", 12), Some("ioctlsocket"));
        assert_eq!(super::name("wsock32", 9), Some("htons"));
        assert_eq!(super::name("wsock32", 23), Some("socket"));
    }
    #[test]
    fn import_by_ordinal_uses_the_dll_s_own_numbering() {
        let name = |dll, ordinal| super::super::find_import(dll, None, Some(ordinal)).map(|e| e.name);
        assert_eq!(name("WSOCK32.dll", 10), Some("inet_addr"));
        assert_eq!(name("WSOCK32.dll", 12), Some("ioctlsocket"));
        assert_eq!(name("WS2_32.dll", 10), Some("ioctlsocket"));
        assert_eq!(name("WS2_32.dll", 11), Some("inet_addr"));
    }
}
