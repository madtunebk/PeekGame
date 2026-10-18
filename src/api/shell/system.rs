//! Shell system query APIs: IsOS.

/// Evaluates whether the operating system matches a specific version or feature.
pub unsafe extern "win64" fn is_os(feature: u32) -> i32 {
    let result = match feature {
        0x00 /* OS_WIN32SORGREATER */ => 1,
        0x01 /* OS_NT */ => 1,
        0x02 /* OS_WIN95ORGREATER */ => 1,
        0x03 /* OS_NT4ORGREATER */ => 1,
        0x04 /* OS_WIN2000ORGREATER_ALT */ => 1,
        0x05 /* OS_WIN98ORGREATER */ => 1,
        0x06 /* OS_WIN98_GOLD */ => 0,
        0x07 /* OS_WIN2000ORGREATER */ => 1,
        0x08 /* OS_WIN2000PRO */ => 1,
        0x09 /* OS_WIN2000SERVER */ => 0,
        0x0A /* OS_WIN2000ADVSERVER */ => 0,
        0x0B /* OS_WIN2000DATACENTER */ => 0,
        0x0C /* OS_WIN2000TERMINAL */ => 0,
        0x0D /* OS_EMBEDDED */ => 0,
        0x0E /* OS_TERMINALCLIENT */ => 0,
        0x0F /* OS_TERMINALREMOTEADMIN */ => 0,
        0x10 /* OS_WIN95_GOLD */ => 0,
        0x11 /* OS_MEORGREATER */ => 1,
        0x12 /* OS_XPORGREATER */ => 1,
        0x13 /* OS_HOME */ => 0,
        0x14 /* OS_PROFESSIONAL */ => 1,
        0x15 /* OS_DATACENTER */ => 0,
        0x16 /* OS_ADVSERVER */ => 0,
        0x17 /* OS_SERVER */ => 0,
        0x18 /* OS_TERMINALSERVER */ => 0,
        0x19 /* OS_PERSONALTERMINALSERVER */ => 0,
        0x1A /* OS_FASTUSERSWITCHING */ => 1,
        0x1B /* OS_WELCOMELOGONUI */ => 0,
        0x1C /* OS_DOMAINMEMBER */ => 0,
        0x1D /* OS_ANYSERVER */ => 0,
        0x1E /* OS_WOW6432 */ => 0, // 64-bit native process
        0x1F /* OS_WEBSERVER */ => 0,
        0x20 /* OS_SMALLBUSINESSSERVER */ => 0,
        0x21 /* OS_TABLETPC */ => 0,
        0x22 /* OS_SERVERADMINUI */ => 0,
        0x23 /* OS_MEDIACENTER */ => 0,
        0x24 /* OS_APPLIANCE */ => 0,
        // Windows Vista or greater (0x25), Windows 7, 8, 8.1, 10
        0x25..=0x30 => 1,
        _ => {
            eprintln!("IsOS: unhandled feature {feature:#x}, defaulting to 0");
            0
        }
    };
    result
}
