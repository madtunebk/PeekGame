# Evidența Importurilor cu Valori Hardcodate (Fixed / Stub Values)

Acest document conține inventarul complet al funcțiilor API Windows implementate în **PeekGame** folosind valori fixe (hardcodate), stubs sau mock-uri. 

Aceste valori fixe au rolul de a permite jocurilor (precum *Palworld*, *DOOM Eternal*, jocuri Godot 4, Unity etc.) și subsistemelor conexe (Unreal Engine 5, CEF / EpicWebHelper, Steamworks, PlayFab) să ruleze fără a se bloca la verificări de compatibilitate, fără a accesa dispozitive hardware inexistente pe Linux sau fără a declanșa mecanisme Windows incompatibile.

---

## 1. Importuri Adăugate sau Modificate Recent (Sesiunile de Bringup Palworld & CEF)

| DLL | Funcție / Simbol | Valoare Hardcodată | Fișier Sursă | Motiv / Comportament |
| :--- | :--- | :--- | :--- | :--- |
| `ws2_32.dll` | `WSAIoctl` (`SIO_IDEAL_SEND_BACKLOG_QUERY` = `0x4004747b`) | `65536` bytes, `*returned = 4`, return `0` | [`src/api/network/sockets.rs`](file:///home/nobus/projects/PeekGame/src/api/network/sockets.rs) | Solicitat de stack-ul de rețea libcurl/HTTP al Epic Online Services (EOS). Returnează o dimensiune optimă de buffer TCP (64 KB). |
| `d3d9.dll` | `D3DPERF_BeginEvent` | return `0` | [`src/api/graphics/directx/d3d9.rs`](file:///home/nobus/projects/PeekGame/src/api/graphics/directx/d3d9.rs) | Profiling marker Direct3D 9; Palworld/UE5 verifică dacă profilerul D3D9 este activ. |
| `d3d9.dll` | `D3DPERF_EndEvent` | return `0` | [`src/api/graphics/directx/d3d9.rs`](file:///home/nobus/projects/PeekGame/src/api/graphics/directx/d3d9.rs) | Închide un eveniment de profiling (marker no-op). |
| `d3d9.dll` | `D3DPERF_SetMarker` | no-op | [`src/api/graphics/directx/d3d9.rs`](file:///home/nobus/projects/PeekGame/src/api/graphics/directx/d3d9.rs) | Inserează un marker de frame în profiler (no-op). |
| `d3d9.dll` | `D3DPERF_SetOptions` | no-op | [`src/api/graphics/directx/d3d9.rs`](file:///home/nobus/projects/PeekGame/src/api/graphics/directx/d3d9.rs) | Setează opțiuni de profiling D3D9 (no-op). |
| `d3d9.dll` | `D3DPERF_GetStatus` | return `0` | [`src/api/graphics/directx/d3d9.rs`](file:///home/nobus/projects/PeekGame/src/api/graphics/directx/d3d9.rs) | Indică faptul că profilerul nu capturează cadre (`0` = inactiv). |
| `d3d9.dll` | `D3DPERF_QueryRepeatFrame` | return `0` | [`src/api/graphics/directx/d3d9.rs`](file:///home/nobus/projects/PeekGame/src/api/graphics/directx/d3d9.rs) | Returnează `FALSE` (nu se repetă cadrul). |
| `d3d9.dll` | `Direct3DCreate9` | return `0` (NULL) | [`src/api/graphics/directx/d3d9.rs`](file:///home/nobus/projects/PeekGame/src/api/graphics/directx/d3d9.rs) | UE5 probează prezența runtime-ului D3D9. Returnează NULL pentru a forța D3D11/D3D12/Vulkan. |
| **TEB** | `NtTib.StackLimit` (`gs:[0x10]` / `teb[2]`) | `0` | [`src/host/teb.rs`](file:///home/nobus/projects/PeekGame/src/host/teb.rs) | Dezactivează bucla `__chkstk` din MSVC în `libcef.dll` (Chromium). Pe Linux, stiva este pre-alocată de pthread; setarea la `0` previne accesarea paginii guard `PROT_NONE` la adrese mici. |
| `kernel32.dll` | `FormatMessageW` | `0x80090302 => "The function requested is not supported."` | [`src/api/system/message.rs`](file:///home/nobus/projects/PeekGame/src/api/system/message.rs) | Mesaj standard pentru `SEC_E_UNSUPPORTED_FUNCTION` cerut de log-ul EOS când Schannel SSPI eșuează. |
| `steam_bridge` | `ISteamNetworkingSockets012::InitAuthentication` | return `1` | [`native/steam_bridge/steam_bridge.c`](file:///home/nobus/projects/PeekGame/native/steam_bridge/steam_bridge.c) | Raportează stare validă de autentificare pentru networking sockets. |
| `steam_bridge` | `ISteamNetworkingSockets012::GetAuthenticationStatus` | return `1` | [`native/steam_bridge/steam_bridge.c`](file:///home/nobus/projects/PeekGame/native/steam_bridge/steam_bridge.c) | Raportează stare ok de autentificare. |

---

## 2. Rețea, Securitate & SSPI (Schannel, WinHTTP, Crypt32, Advapi32)

| DLL | Funcție | Valoare Hardcodată | Fișier Sursă | Explicație |
| :--- | :--- | :--- | :--- | :--- |
| `secur32.dll` | `InitSecurityInterfaceW` / `InitSecurityInterfaceA` | Returnează `SecurityFunctionTable` (v4) cu pointeri la `sspi::unsupported` | [`src/api/network/sspi.rs`](file:///home/nobus/projects/PeekGame/src/api/network/sspi.rs) | Toate funcțiile din tabelă returnează `SEC_E_UNSUPPORTED_FUNCTION` (`0x80090302`), semnalând că pachetele Schannel/NTLM/Kerberos nu sunt disponibile în subsistem. |
| `secur32.dll` | `LsaEnumerateLogonSessions` | `1` sesiune, LUID `999` (High=0, Low=999) | [`src/api/network/sspi.rs`](file:///home/nobus/projects/PeekGame/src/api/network/sspi.rs) | Simulează o singură sesiune interactivă de utilizator local. |
| `winhttp.dll` | `WinHttpCheckPlatform` | return `1` (`TRUE`) | [`src/api/network/winhttp.rs`](file:///home/nobus/projects/PeekGame/src/api/network/winhttp.rs) | Confirmă că platforma suportă WinHTTP. |
| `iphlpapi.dll` | `GetAdaptersInfo` | 1 adapter Ethernet, MAC fix `02:00:4c:4f:43:01`, IP `192.168.1.100`, Gateway `192.168.1.1` | [`src/api/network/adapters.rs`](file:///home/nobus/projects/PeekGame/src/api/network/adapters.rs) | Topologie statică de rețea dacă interfețele Linux reale nu sunt enumerate. |
| `iphlpapi.dll` | `GetBestInterface` | return adapter index `1` | [`src/api/network/adapters.rs`](file:///home/nobus/projects/PeekGame/src/api/network/adapters.rs) | Direcționează rutele către primul adaptor de rețea. |
| `advapi32.dll` | `CryptGetUserKey`, `CryptExportKey`, `CryptImportKey`, `CryptEncrypt`, `CryptDecrypt`, `CryptSignHashW`, `CryptEnumProvidersW` | return `0` (`FALSE`) | [`src/registry/system/compatibility.rs`](file:///home/nobus/projects/PeekGame/src/registry/system/compatibility.rs) | CryptoAPI 1.0 legacy stubs; jocurile moderne folosesc BCrypt/OpenSSL, iar apelurile legacy raportează lipsa cheilor. |
| `advapi32.dll` | `CredReadW`, `CredWriteW`, `CredDeleteW` | return `0` (`FALSE`, `ERROR_NOT_FOUND`) | [`src/registry/system/compatibility.rs`](file:///home/nobus/projects/PeekGame/src/registry/system/compatibility.rs) | Windows Credential Manager; raportează absența credențialelor salvate. |
| `crypt32.dll` | `CertGetNameStringW`, `CertGetNameStringA` | return `1` (șir vid `""`) | [`src/registry/system/compatibility.rs`](file:///home/nobus/projects/PeekGame/src/registry/system/compatibility.rs) | Returnează un nume de certificat gol când contextul nu este complet parsabil. |
| `wintrust.dll` | `WinVerifyTrust` | return `0` (`ERROR_SUCCESS`) | [`src/registry/system/compatibility.rs`](file:///home/nobus/projects/PeekGame/src/registry/system/compatibility.rs) | Verificarea semnăturilor de drivere și DLL-uri; returnează succes permanent pentru a preveni blocajele DRM. |

---

## 3. Sistem & Kernel (Hardware, Discuri, Memorie, Procese)

| DLL | Funcție | Valoare Hardcodată | Fișier Sursă | Explicație |
| :--- | :--- | :--- | :--- | :--- |
| `kernel32.dll` | `GetSystemPowerStatus` | `ACLineStatus = 1` (Online), `BatteryFlag = 128` (No battery), `BatteryLifePercent = 255` | [`src/api/system/power.rs`](file:///home/nobus/projects/PeekGame/src/api/system/power.rs) | Simulează un sistem desktop conectat la priză, fără baterie. |
| `kernel32.dll` | `GetLogicalDrives` | return `0x4` (doar `C:`) | [`src/api/filesystem/files.rs`](file:///home/nobus/projects/PeekGame/src/api/filesystem/files.rs) | Raportează masca de unități Windows compatibile. |
| `kernel32.dll` | `GetDiskFreeSpaceW` | 8 sectoare/cluster, 512 bytes/sector, 100 GB liber hardcodat | [`src/api/filesystem/files.rs`](file:///home/nobus/projects/PeekGame/src/api/filesystem/files.rs) | Valori de fallback când `statvfs` nu poate fi interogat pe cale. |
| `kernel32.dll` | `GetLogicalProcessorInformation` | 1 socket, 1 nod NUMA (Node 0), mască de afinitate core | [`src/api/system/info.rs`](file:///home/nobus/projects/PeekGame/src/api/system/info.rs) | Topologie multiprocesor standardizată pentru planificatorul de thread-uri al motoarelor grafice. |
| `kernel32.dll` | `GetLargePageMinimum` | return `2 * 1024 * 1024` (2 MB) | [`src/api/memory/virtual_memory.rs`](file:///home/nobus/projects/PeekGame/src/api/memory/virtual_memory.rs) | Dimensiunea minimă a paginilor transparente mari (Transparent Huge Pages). |
| `kernel32.dll` | `DeviceIoControl` | `IOCTL_STORAGE_GET_DEVICE_NUMBER` (DeviceType=7, Number=0, Partition=1) | [`src/api/system/devices.rs`](file:///home/nobus/projects/PeekGame/src/api/system/devices.rs) | Răspunde interogărilor jocurilor despre discurile fizice (raportează disk 0, part 1). |
| `advapi32.dll` | `RegisterEventSourceW` | return dummy handle `0x1000` | [`src/registry/system/compatibility.rs`](file:///home/nobus/projects/PeekGame/src/registry/system/compatibility.rs) | Handle fictiv pentru log-ul de evenimente Windows (Event Log). |
| `advapi32.dll` | `ReportEventW`, `DeregisterEventSource` | return `1` (`TRUE`) | [`src/registry/system/compatibility.rs`](file:///home/nobus/projects/PeekGame/src/registry/system/compatibility.rs) | Ignoră scrierea în Event Log și returnează succes. |
| `ntdll.dll` | `NtQueryInformationProcess` | `ProcessCookie = 0x5a5a5a5a`, `ProcessExecuteFlags = 0x2` | [`src/api/system/process.rs`](file:///home/nobus/projects/PeekGame/src/api/system/process.rs) | Cookie de proces și flag-uri DEP constante cerute de CRT. |

---

## 4. Grafică, GDI & Interfață Fereastră (User32, GDI32, UXTheme, IMM32)

| DLL | Funcție | Valoare Hardcodată | Fișier Sursă | Explicație |
| :--- | :--- | :--- | :--- | :--- |
| `user32.dll` | `SetParent` | return desktop HWND (`0x10010`) | [`src/api/window/misc.rs`](file:///home/nobus/projects/PeekGame/src/api/window/misc.rs) | Evită ierarhii imbricate de ferestre nesuportate de serverul X11. |
| `user32.dll` | `IsChild` | return `0` (`FALSE`) | [`src/api/window/misc.rs`](file:///home/nobus/projects/PeekGame/src/api/window/misc.rs) | Toate ferestrele de joc sunt considerate top-level. |
| `user32.dll` | `GetWindowDisplayAffinity` | return `0` (`WDA_NONE`) | [`src/api/window/misc.rs`](file:///home/nobus/projects/PeekGame/src/api/window/misc.rs) | Fără protecție de captură video / screen recording. |
| `user32.dll` | `IsDialogMessageW` | return `0` (`FALSE`) | [`src/api/window/misc.rs`](file:///home/nobus/projects/PeekGame/src/api/window/misc.rs) | Mesajele nu aparțin unei casete de dialog Windows clasice. |
| `user32.dll` | `GetClassLongPtrW` / `GetClassLongW` | return `0` | [`src/api/window/misc.rs`](file:///home/nobus/projects/PeekGame/src/api/window/misc.rs) | Câmpuri de clasă extra neutilizate. |
| `user32.dll` | `GetDlgItem` | return `0` (NULL) | [`src/api/window/misc.rs`](file:///home/nobus/projects/PeekGame/src/api/window/misc.rs) | Fără elemente de dialog copil. |
| `user32.dll` | `GetDlgItemTextW` | return `0` (șir vid) | [`src/api/window/misc.rs`](file:///home/nobus/projects/PeekGame/src/api/window/misc.rs) | Text element dialog inexistent. |
| `user32.dll` | `DrawTextW` | return înălțime `16` | [`src/api/window/misc.rs`](file:///home/nobus/projects/PeekGame/src/api/window/misc.rs) | Măsurătoare de text minimală fallback. |
| `user32.dll` | `TrackPopupMenu` | return `0` | [`src/api/window/misc.rs`](file:///home/nobus/projects/PeekGame/src/api/window/misc.rs) | Meniuri popup neutilizate în randare full-screen. |
| `user32.dll` | `SetWinEventHook` | return handle fictiv `0x5001` | [`src/api/window/hooks.rs`](file:///home/nobus/projects/PeekGame/src/api/window/hooks.rs) | Stub pentru hook-uri de accesibilitate/evenimente UI. |
| `user32.dll` | `UnhookWinEvent` | return `1` (`TRUE`) | [`src/api/window/hooks.rs`](file:///home/nobus/projects/PeekGame/src/api/window/hooks.rs) | Eliberează hook-ul de evenimente UI. |
| `gdi32.dll` | `GetTextExtentPoint32W` | celulă fixă de `8x16` pixeli (`cx = 8 * len`, `cy = 16`) | [`src/api/window/regions.rs`](file:///home/nobus/projects/PeekGame/src/api/window/regions.rs) | Metrică de font consolă/raster pentru UI preliminar. |
| `imm32.dll` | `ImmGetDescriptionW` | return `0` | [`src/api/window/misc.rs`](file:///home/nobus/projects/PeekGame/src/api/window/misc.rs) | Fără IME (Input Method Editor) instalat. |
| `uxtheme.dll` | `SetWindowTheme` | return `0` (`S_OK`) | [`src/api/window/misc.rs`](file:///home/nobus/projects/PeekGame/src/api/window/misc.rs) | Teme vizuale clasice dezactivate. |

---

## 5. Multimedia, Audio & XInput (WinMM, DirectSound, XInput)

| DLL | Funcție | Valoare Hardcodată | Fișier Sursă | Explicație |
| :--- | :--- | :--- | :--- | :--- |
| `winmm.dll` | `timeBeginPeriod`, `timeEndPeriod` | return `0` (`TIMERR_NOERROR`) | [`src/registry/system/multimedia.rs`](file:///home/nobus/projects/PeekGame/src/registry/system/multimedia.rs) | Rezoluția timer-ului este gestionată nativ de Linux kernel (`nanosleep`). |
| `winmm.dll` | `waveOutGetNumDevs`, `waveInGetNumDevs`, `midiOutGetNumDevs`, `midiInGetNumDevs` | return `0` | [`src/registry/system/multimedia.rs`](file:///home/nobus/projects/PeekGame/src/registry/system/multimedia.rs) | Raportează 0 dispozitive legacy; forțează folosirea WASAPI/PulseAudio. |
| `xinput1_3.dll` / `xinput1_4.dll` | `XInputGetDSoundAudioDeviceGuids`, `XInputGetAudioDeviceIds` | return `1167` (`ERROR_DEVICE_NOT_CONNECTED`) | [`src/api/window/input/xinput.rs`](file:///home/nobus/projects/PeekGame/src/api/window/input/xinput.rs) | Semnalează lipsa căștilor/dispozitivelor audio conectate prin controller gamepad. |
| `dsound.dll` | `DirectSoundCreate`, `DirectSoundCreate8`, `DirectSoundEnumerateA/W` | return `0x8878000A` (`DSERR_NODRIVER`) | [`src/registry/system/compatibility.rs`](file:///home/nobus/projects/PeekGame/src/registry/system/compatibility.rs) | Forțează motoarele de sunet să utilizeze WASAPI în loc de DirectSound 8 legacy. |

---

## 6. Sumar și Recomandări pentru Viitor

1. **Rețea & Conectivitate EOS (`secur32.dll` / SSPI)**:
   * În prezent, `InitSecurityInterfaceW` returnează `SEC_E_UNSUPPORTED_FUNCTION` pentru a împiedica blocajele legate de autentificări domeniale Windows (Kerberos/NTLM).
   * Pentru modulul de autentificare online completă EOS prin HTTPS/TLS, tabelul de funcții Schannel poate fi conectat la backend-ul OpenSSL deja existent în [`src/api/network/schannel.rs`](file:///home/nobus/projects/PeekGame/src/api/network/schannel.rs).

2. **Topologie Rețea (`iphlpapi.dll`)**:
   * Adresa IP și MAC-ul sunt momentan statice (`192.168.1.100`). Dacă jocurile verifică subrețele reale pentru LAN co-op, se pot citi adresele reale din `/sys/class/net` sau prin `getifaddrs()`.

3. **Stabilitate Stivă (`teb[2] = 0`)**:
   * Setarea `StackLimit = 0` în TEB rezolvă definitiv problema crash-urilor `SIGSEGV` cauzate de `__chkstk` în biblioteci compilate cu MSVC (cum e `libcef.dll`), fără niciun impact negativ asupra restului sistemului.
