//! Export declarations for compatibility. Implementation stays in api/.
use crate::api::system::compat;
use crate::registry::Export;

exports! {
    implemented!("SetHandleCount", "system", "implemented", "Windows NT no-op; returns requested count", compat::set_handle_count),
    implemented!(
        "lstrlenW",
        "system",
        "partial",
        "complete",
        compat::lstrlen_w
    ),
    implemented!(
        "lstrlenA",
        "system",
        "partial",
        "complete",
        compat::lstrlen_a
    ),
    implemented!(
        "lstrcmpW",
        "system",
        "partial",
        "ordinal comparison",
        compat::lstrcmp_w
    ),
    implemented!(
        "lstrcmpiW",
        "system",
        "partial",
        "case-insensitive",
        compat::lstrcmpi_w
    ),
    implemented!(
        "lstrcmpA",
        "system",
        "partial",
        "byte comparison",
        compat::lstrcmp_a
    ),
    implemented!(
        "lstrcmpiA",
        "system",
        "partial",
        "ASCII case-insensitive",
        compat::lstrcmpi_a
    ),
    implemented!(
        "lstrcpyW",
        "system",
        "partial",
        "complete",
        compat::lstrcpy_w
    ),
    implemented!(
        "lstrcpynW",
        "system",
        "partial",
        "complete",
        compat::lstrcpyn_w
    ),
    implemented!(
        "lstrcatW",
        "system",
        "partial",
        "complete",
        compat::lstrcat_w
    ),
    implemented!(
        "lstrcpyA",
        "system",
        "partial",
        "complete",
        compat::lstrcpy_a
    ),
    implemented!(
        "CreateSemaphoreA",
        "threading",
        "partial",
        "as CreateSemaphoreW",
        compat::create_semaphore_a
    ),
    implemented!(
        "GetTempPathA",
        "filesystem",
        "partial",
        "as GetTempPathW",
        compat::temp_path_a
    ),
    implemented!(
        "GetTempFileNameA",
        "filesystem",
        "partial",
        "as GetTempFileNameW",
        compat::temp_file_name_a
    ),
    implemented!(
        "GetProcessAffinityMask",
        "system",
        "partial",
        "every host CPU",
        compat::process_affinity_mask
    ),
    implemented!(
        "SetProcessAffinityMask",
        "system",
        "partial",
        "accepted",
        compat::set_process_affinity_mask
    ),
    implemented!(
        "GetHandleInformation",
        "system",
        "partial",
        "no flags",
        compat::handle_information
    ),
    implemented!(
        "SetHandleInformation",
        "system",
        "partial",
        "accepted",
        compat::set_handle_information
    ),
    implemented!(
        "OpenProcess",
        "system",
        "partial",
        "this process only",
        compat::open_process
    ),
    implemented!(
        "DeviceIoControl",
        "system",
        "partial",
        "no device drivers",
        compat::device_io_control
    ),
    export!(
        "advapi32.dll",
        "RegNotifyChangeKeyValue",
        "system",
        "partial",
        "accepted; no notifications",
        compat::reg_notify_change_key_value
    ),
    export!(
        "gdi32.dll",
        "CreateCompatibleDC",
        "window",
        "partial",
        "token DC",
        compat::create_compatible_dc
    ),
    export!(
        "gdi32.dll",
        "CreateDCW",
        "window",
        "partial",
        "token DC",
        compat::create_dc
    ),
    export!(
        "gdi32.dll",
        "CreateDCA",
        "window",
        "partial",
        "token DC",
        compat::create_dc
    ),
    export!(
        "gdi32.dll",
        "DeleteDC",
        "window",
        "partial",
        "token DC",
        compat::delete_dc
    ),
    export!(
        "gdi32.dll",
        "GetICMProfileW",
        "window",
        "partial",
        "no custom ICM profile: returns FALSE for standard sRGB",
        compat::get_icm_profile
    ),
    export!(
        "gdi32.dll",
        "GetICMProfileA",
        "window",
        "partial",
        "no custom ICM profile: returns FALSE for standard sRGB",
        compat::get_icm_profile
    ),
    export!(
        "wtsapi32.dll",
        "WTSRegisterSessionNotification",
        "window",
        "partial",
        "accepted; no session change notifications",
        compat::wts_register_session_notification
    ),
    export!(
        "wtsapi32.dll",
        "WTSRegisterSessionNotificationEx",
        "window",
        "partial",
        "accepted; no session change notifications",
        compat::wts_register_session_notification_ex
    ),
    export!(
        "wtsapi32.dll",
        "WTSUnRegisterSessionNotification",
        "window",
        "partial",
        "accepted; no session change notifications",
        compat::wts_unregister_session_notification
    ),
    export!(
        "wtsapi32.dll",
        "WTSUnRegisterSessionNotificationEx",
        "window",
        "partial",
        "accepted; no session change notifications",
        compat::wts_unregister_session_notification_ex
    ),
    export!(
        "user32.dll",
        "ChangeDisplaySettingsExW",
        "window",
        "partial",
        "no mode switching: restore and the device's current size succeed, other sizes DISP_CHANGE_BADMODE",
        compat::change_display_settings_ex_w
    ),
    export!(
        "user32.dll",
        "ChangeDisplaySettingsExA",
        "window",
        "partial",
        "no mode switching: restore and the device's current size succeed, other sizes DISP_CHANGE_BADMODE",
        compat::change_display_settings_ex_a
    ),
    implemented!(
        "GetProfileIntA",
        "system",
        "partial",
        "no win.ini; returns caller default",
        compat::get_profile_int_a
    ),
    implemented!(
        "MulDiv",
        "system",
        "implemented",
        "64-bit intermediate multiply-divide with rounding",
        compat::mul_div
    ),
    export!(
        "netapi32.dll",
        "NetGetJoinInformation",
        "system",
        "partial",
        "reports NetSetupWorkgroupName (WORKGROUP)",
        compat::net_get_join_information
    ),
    export!(
        "netapi32.dll",
        "NetApiBufferFree",
        "system",
        "implemented",
        "no-op free for static NetGetJoinInformation buffer",
        compat::net_api_buffer_free
    ),
    export!(
        "netapi32.dll",
        "NetUserGetInfo",
        "system",
        "partial",
        "returns NERR_UserNotFound (2221)",
        compat::net_user_get_info
    ),
    export!(
        "wtsapi32.dll",
        "WTSQuerySessionInformationW",
        "window",
        "partial",
        "returns zeroed 4-byte session info buffer",
        compat::wts_query_session_information_w
    ),
    export!(
        "wtsapi32.dll",
        "WTSFreeMemory",
        "window",
        "implemented",
        "no-op free for static WTS session buffer",
        compat::wts_free_memory
    ),
    export!(
        "userenv.dll",
        "CreateEnvironmentBlock",
        "system",
        "partial",
        "returns empty double-null-terminated environment block",
        compat::create_environment_block
    ),
    export!(
        "userenv.dll",
        "DestroyEnvironmentBlock",
        "system",
        "implemented",
        "no-op destroy for static environment block",
        compat::destroy_environment_block
    ),
    export!(
        "secur32.dll",
        "GetUserNameExW",
        "system",
        "partial",
        "returns player username",
        compat::get_user_name_ex_w
    ),
    export!("secur32.dll", "CompleteAuthToken", "system", "partial", "returns SEC_E_OK", compat::complete_auth_token),
    export!("dsound.dll", "DirectSoundCreate", "multimedia", "partial", "returns DSERR_NODRIVER", compat::dsound_create),
    export!("dsound.dll", "DirectSoundCreate8", "multimedia", "partial", "returns DSERR_NODRIVER", compat::dsound_create),
    export!("dsound.dll", "DirectSoundCaptureCreate", "multimedia", "partial", "returns DSERR_NODRIVER", compat::dsound_create),
    export!("dsound.dll", "DirectSoundCaptureCreate8", "multimedia", "partial", "returns DSERR_NODRIVER", compat::dsound_create),
    export!("dsound.dll", "DirectSoundEnumerateA", "multimedia", "partial", "no DirectSound devices", compat::dsound_enum),
    export!("dsound.dll", "DirectSoundEnumerateW", "multimedia", "partial", "no DirectSound devices", compat::dsound_enum),
    export!("dsound.dll", "DirectSoundCaptureEnumerateA", "multimedia", "partial", "no DirectSound capture devices", compat::dsound_enum),
    export!("dsound.dll", "DirectSoundCaptureEnumerateW", "multimedia", "partial", "no DirectSound capture devices", compat::dsound_enum),
    export!("shell32.dll", "SHChangeNotifyRegister", "filesystem", "partial", "returns registration ID 1", compat::sh_change_notify_register),
    export!("shell32.dll", "SHChangeNotifyDeregister", "filesystem", "partial", "returns TRUE", compat::sh_change_notify_deregister),
    export!("shell32.dll", "IsUserAnAdmin", "system", "implemented", "returns FALSE (non-admin)", compat::is_user_an_admin),
    export!("shlwapi.dll", "SHAllocShared", "system", "partial", "returns NULL", compat::sh_alloc_shared),
    export!("shlwapi.dll", "QISearch", "com", "partial", "returns E_NOINTERFACE", compat::qi_search),
    export!("shlwapi.dll", "PathFindFileNameW", "filesystem", "implemented", "finds file name component in wide path", compat::path_find_file_name_w),
    export!("shlwapi.dll", "PathFindExtensionW", "filesystem", "implemented", "finds extension in wide path", compat::path_find_extension_w),
    export!("shlwapi.dll", "PathFileExistsW", "filesystem", "implemented", "checks file existence via GetFileAttributesW", compat::path_file_exists_w),
    export!("shlwapi.dll", "PathAppendW", "filesystem", "implemented", "appends wide path component", compat::path_append_w),
    export!("shlwapi.dll", "PathRemoveExtensionW", "filesystem", "implemented", "removes extension from wide path", compat::path_remove_extension_w),
    export!("shlwapi.dll", "PathMatchSpecW", "filesystem", "partial", "returns FALSE", compat::path_match_spec_w),
    export!("shlwapi.dll", "AssocQueryStringW", "filesystem", "partial", "returns E_FAIL", compat::assoc_query_string_w),
    export!("uxtheme.dll", "DrawThemeBackground", "window", "partial", "no-op theme background draw", compat::draw_theme_background),
    export!("uxtheme.dll", "OpenThemeData", "window", "partial", "returns NULL theme handle", compat::open_theme_data),
    export!("uxtheme.dll", "CloseThemeData", "window", "partial", "returns S_OK", compat::close_theme_data),
    export!("uxtheme.dll", "GetThemePartSize", "window", "partial", "returns E_FAIL", compat::get_theme_part_size),
    export!("uxtheme.dll", "GetThemeBackgroundContentRect", "window", "partial", "returns S_OK", compat::get_theme_background_content_rect),
    export!("winspool.drv", "ConnectToPrinterDlg", "window", "partial", "returns NULL", compat::connect_to_printer_dlg),
    implemented!("QueueUserWorkItem", "threading", "implemented", "spawns worker thread", compat::queue_user_work_item),
    implemented!("CreateFiber", "threading", "partial", "fiber handle stub", compat::create_fiber),
    implemented!("DeleteFiber", "threading", "partial", "no-op fiber delete", compat::delete_fiber),
    implemented!("SwitchToFiber", "threading", "partial", "no-op fiber switch", compat::switch_to_fiber),
    implemented!("IsThreadAFiber", "threading", "implemented", "returns FALSE", compat::is_thread_a_fiber),
    implemented!("CreateEventExA", "threading", "implemented", "ANSI CreateEventEx wrapper", compat::create_event_ex_a),
    implemented!("QueryFullProcessImageNameW", "system", "implemented", "queries process executable path", compat::query_full_process_image_name_w),
    implemented!("GetProcessHandleCount", "system", "partial", "returns 64 handles", compat::get_process_handle_count),
    implemented!("ReOpenFile", "filesystem", "partial", "returns INVALID_HANDLE_VALUE", compat::reopen_file),
    implemented!("MoveFileExA", "filesystem", "implemented", "ANSI MoveFileEx wrapper", compat::move_file_ex_a),
    implemented!("FreeConsole", "system", "partial", "returns TRUE", compat::console_noop_true),
    implemented!("SetConsoleScreenBufferSize", "system", "partial", "returns TRUE", compat::console_noop_true),
    implemented!("SetConsoleWindowInfo", "system", "partial", "returns TRUE", compat::console_noop_true),
    implemented!("SetConsoleTitleW", "system", "partial", "returns TRUE", compat::console_noop_true),
    implemented!("SetConsoleTitleA", "system", "partial", "returns TRUE", compat::console_noop_true),
    implemented!("FillConsoleOutputAttribute", "system", "partial", "returns TRUE", compat::console_noop_true),
    implemented!("FillConsoleOutputCharacterW", "system", "partial", "returns TRUE", compat::console_noop_true),
    implemented!("GetConsoleCursorInfo", "system", "partial", "returns TRUE", compat::console_noop_true),
    implemented!("SetConsoleCursorInfo", "system", "partial", "returns TRUE", compat::console_noop_true),
    implemented!("SetConsoleCursorPosition", "system", "partial", "returns TRUE", compat::console_noop_true),
    implemented!("ReadConsoleInputW", "system", "partial", "returns TRUE", compat::console_noop_true),
    implemented!("WriteConsoleInputW", "system", "partial", "returns TRUE", compat::console_noop_true),
    implemented!("WriteConsoleA", "system", "partial", "reports bytes written", compat::write_console_a),
    implemented!("ReadConsoleA", "system", "partial", "reports 0 bytes read", compat::read_console_a),
    implemented!("GetNumberOfConsoleInputEvents", "system", "partial", "reports 0 input events", compat::get_number_of_console_input_events),
    implemented!("GetNamedPipeHandleStateW", "filesystem", "partial", "returns FALSE", compat::get_named_pipe_handle_state_w),
    implemented!("ReadDirectoryChangesW", "filesystem", "partial", "returns FALSE", compat::read_directory_changes_w),
    export!("ntdll.dll", "NtCreateNamedPipeFile", "filesystem", "partial", "returns STATUS_OBJECT_NAME_NOT_FOUND", compat::nt_create_named_pipe_file),
    export!("qwave.dll", "QOSCreateHandle", "network", "partial", "returns FALSE", compat::qos_false),
    export!("qwave.dll", "QOSCloseHandle", "network", "partial", "returns FALSE", compat::qos_false),
    export!("qwave.dll", "QOSAddSocketToFlow", "network", "partial", "returns FALSE", compat::qos_false),
    export!("qwave.dll", "QOSRemoveSocketFromFlow", "network", "partial", "returns FALSE", compat::qos_false),
    export!("cfgmgr32.dll", "CM_Locate_DevNodeW", "system", "partial", "returns CR_NO_SUCH_DEVNODE", compat::cm_locate_devnode_w),
    export!("cfgmgr32.dll", "CM_Get_DevNode_PropertyW", "system", "partial", "returns CR_NO_SUCH_VALUE", compat::cm_get_devnode_property_w),
    export!("cfgmgr32.dll", "CM_MapCrToWin32Err", "system", "implemented", "maps CONFIGRET to Win32 error", compat::cm_map_cr_to_win32_err),
    export!("setupapi.dll", "CM_Locate_DevNodeW", "system", "partial", "returns CR_NO_SUCH_DEVNODE", compat::cm_locate_devnode_w),
    export!("setupapi.dll", "CM_Get_Device_IDW", "system", "partial", "returns CR_NO_SUCH_DEVNODE", compat::cm_locate_devnode_w),
    export!("setupapi.dll", "CM_Get_Device_ID_List_SizeW", "system", "partial", "returns empty list size", compat::cm_get_device_id_list_size_w),
    export!("setupapi.dll", "CM_Get_Device_ID_ListW", "system", "partial", "returns empty double-null list", compat::cm_get_device_id_list_w),
    export!("setupapi.dll", "SetupDiOpenDevRegKey", "system", "partial", "returns INVALID_HANDLE_VALUE", compat::setup_di_open_dev_reg_key),
    export!("setupapi.dll", "SetupGetInfDriverStoreLocationW", "system", "partial", "returns FALSE", compat::setup_get_inf_driver_store_location_w),
    export!("iphlpapi.dll", "IcmpCreateFile", "network", "partial", "returns INVALID_HANDLE_VALUE", compat::icmp_create_file),
    export!("iphlpapi.dll", "IcmpCloseHandle", "network", "partial", "returns TRUE", compat::icmp_close_handle),
    export!("iphlpapi.dll", "IcmpSendEcho", "network", "partial", "returns 0 (IP_REQ_TIMED_OUT)", compat::icmp_send_echo),
    export!("ws2_32.dll", "WSADuplicateSocketW", "network", "partial", "returns SOCKET_ERROR", compat::wsa_duplicate_socket_w),
    export!("ws2_32.dll", "WSAAddressToStringW", "network", "partial", "returns SOCKET_ERROR", compat::wsa_address_to_string_w),
    export!("ws2_32.dll", "WSAStringToAddressW", "network", "partial", "returns SOCKET_ERROR", compat::wsa_string_to_address_w),
    export!("advapi32.dll", "CryptAcquireContextW", "crypto", "partial", "returns dummy HCRYPTPROV", compat::crypt_acquire_context),
    export!("advapi32.dll", "CryptAcquireContextA", "crypto", "partial", "returns dummy HCRYPTPROV", compat::crypt_acquire_context),
    export!("advapi32.dll", "CryptReleaseContext", "crypto", "partial", "returns TRUE", compat::crypt_release_context),
    export!("advapi32.dll", "CryptCreateHash", "crypto", "partial", "returns dummy HCRYPTHASH", compat::crypt_create_hash),
    export!("advapi32.dll", "CryptHashData", "crypto", "partial", "returns TRUE", compat::crypt_hash_data),
    export!("advapi32.dll", "CryptGetHashParam", "crypto", "partial", "returns zeroed digest", compat::crypt_get_hash_param),
    export!("advapi32.dll", "CryptSetHashParam", "crypto", "partial", "returns TRUE", compat::crypt_release_context),
    export!("advapi32.dll", "CryptDestroyHash", "crypto", "partial", "returns TRUE", compat::crypt_destroy_handle),
    export!("advapi32.dll", "CryptDestroyKey", "crypto", "partial", "returns TRUE", compat::crypt_destroy_handle),
    export!("advapi32.dll", "CryptGetProvParam", "crypto", "partial", "returns FALSE", compat::crypt_stub_false),
    export!("advapi32.dll", "CryptGetUserKey", "crypto", "partial", "returns FALSE", compat::crypt_stub_false),
    export!("advapi32.dll", "CryptExportKey", "crypto", "partial", "returns FALSE", compat::crypt_stub_false),
    export!("advapi32.dll", "CryptImportKey", "crypto", "partial", "returns FALSE", compat::crypt_stub_false),
    export!("advapi32.dll", "CryptEncrypt", "crypto", "partial", "returns FALSE", compat::crypt_stub_false),
    export!("advapi32.dll", "CryptDecrypt", "crypto", "partial", "returns FALSE", compat::crypt_stub_false),
    export!("advapi32.dll", "CryptSignHashW", "crypto", "partial", "returns FALSE", compat::crypt_stub_false),
    export!("advapi32.dll", "CryptEnumProvidersW", "crypto", "partial", "returns FALSE", compat::crypt_stub_false),
    export!("advapi32.dll", "CredReadW", "crypto", "partial", "returns FALSE (ERROR_NOT_FOUND)", compat::crypt_stub_false),
    export!("advapi32.dll", "CredWriteW", "crypto", "partial", "returns FALSE", compat::crypt_stub_false),
    export!("advapi32.dll", "CredDeleteW", "crypto", "partial", "returns FALSE", compat::crypt_stub_false),
    export!("advapi32.dll", "CredFree", "crypto", "partial", "no-op", compat::delete_fiber),
    export!("advapi32.dll", "RegisterEventSourceW", "system", "partial", "returns dummy event log handle", compat::create_fiber),
    export!("advapi32.dll", "ReportEventW", "system", "partial", "returns TRUE", compat::console_noop_true),
    export!("advapi32.dll", "DeregisterEventSource", "system", "partial", "returns TRUE", compat::crypt_destroy_handle),
    export!("crypt32.dll", "CertGetNameStringW", "crypto", "partial", "returns 1 (empty string)", compat::console_noop_true),
    export!("crypt32.dll", "CertGetNameStringA", "crypto", "partial", "returns 1 (empty string)", compat::console_noop_true),
    export!("crypt32.dll", "CertGetEnhancedKeyUsage", "crypto", "partial", "returns FALSE", compat::crypt_stub_false),
    export!("crypt32.dll", "CertGetIntendedKeyUsage", "crypto", "partial", "returns FALSE", compat::crypt_stub_false),
    export!("crypt32.dll", "CryptStringToBinaryA", "crypto", "partial", "returns FALSE", compat::crypt_stub_false),
    export!("crypt32.dll", "CryptQueryObject", "crypto", "partial", "returns FALSE", compat::crypt_stub_false),
    export!("crypt32.dll", "CryptMsgGetParam", "crypto", "partial", "returns FALSE", compat::crypt_stub_false),
    export!("crypt32.dll", "PFXImportCertStore", "crypto", "partial", "returns NULL", compat::sh_alloc_shared),
    export!("crypt32.dll", "CryptDecodeObjectEx", "crypto", "partial", "returns FALSE", compat::crypt_stub_false),
    export!("crypt32.dll", "CertAddCertificateContextToStore", "crypto", "partial", "returns TRUE", compat::console_noop_true),
    export!("crypt32.dll", "CertFindExtension", "crypto", "partial", "returns NULL", compat::sh_alloc_shared),
    export!("crypt32.dll", "CertCreateCertificateChainEngine", "crypto", "partial", "returns FALSE", compat::crypt_stub_false),
    export!("crypt32.dll", "CertFreeCertificateChainEngine", "crypto", "partial", "no-op", compat::delete_fiber),
    export!("wintrust.dll", "WinVerifyTrust", "crypto", "partial", "returns ERROR_SUCCESS", compat::win_verify_trust),
    implemented!("GetCurrentThreadStackLimits", "threading", "implemented", "retrieves calling thread stack limits from TEB", compat::get_current_thread_stack_limits),
}


