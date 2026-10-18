# Palworld UE5 Engine & CEF Bringup Progress

**Date**: 2026-10-18  
**Component**: Unreal Engine 5 (`Palworld-Win64-Shipping.exe`), DXVK Presenter SwapChain, CEF3 (Chromium Embedded Framework), Shell & System APIs  
**Branch**: `doom-eternal-bringup`

---

## 1. Summary of Milestones Achieved

1. **DXVK Vulkan Presenter SwapChain Created**:
   - DXVK successfully created the native Vulkan presentation swapchain:
     - Format: `VK_FORMAT_B8G8R8A8_UNORM`
     - Resolution: `2560x1440`
     - Buffers: 5
     - Mode: `VK_PRESENT_MODE_IMMEDIATE_KHR`
2. **UE5 Third-Party Subsystems Attached**:
   - Wwise Audio Plugins loaded cleanly:
     - `AkPeakLimiter.dll`
     - `AkRoomVerb.dll`
     - `AkGuitarDistortion.dll`
     - `AkTimeStretch.dll`
   - D3DCompiler loaded cleanly: `d3dcompiler_47.dll`.
   - Chromium Embedded Framework booted: `chrome_elf.dll`, `libcef.dll`.
   - DirectWrite loaded system font collections (247 families via fontconfig).
3. **Critical SLIST Lock-Free Bug Fixed**:
   - Fixed `cmpxchg16b` inline assembly in `src/api/threading/slist.rs`.
   - Previously, LLVM mapped `ok` (`reg_byte`) to `%bl` and `mov rbx, {new0}` clobbered the status byte after `sete %bl`, trapping `slist::push` in an infinite loop.
   - Preserved `%rbx` via stack push/pop and explicitly isolated output registers; unit test verified in debug and release.
4. **Shell & Ordinal Resolution**:
   - Implemented `IsOS` in `src/api/shell/system.rs` supporting OS feature checks (Windows NT, Windows 10, desktop non-server).
   - Exported `IsOS` in `src/registry/shell.rs` for `shlwapi.dll` and `shcore.dll`.
   - Added ordinal 437 mapping in `src/registry/ordinals.rs` under `shlwapi`.
5. **Filesystem Change Notifications**:
   - Implemented `FindFirstChangeNotificationW`, `FindFirstChangeNotificationA`, `FindNextChangeNotification`, and `FindCloseChangeNotification` in `src/api/filesystem/files.rs`.
   - Backed by lightweight event handles compatible with `WaitForSingleObject` and `CloseHandle`.
   - Registered under `kernel32.dll` in `src/registry/filesystem.rs`.
6. **Computer Name Extensions**:
   - Implemented `GetComputerNameA`, `GetComputerNameExW`, and `GetComputerNameExA` in `src/api/system/info.rs`.
   - Supports `ComputerNameNetBIOS`, `ComputerNameDnsHostname`, `ComputerNameDnsDomain`, `ComputerNameDnsFullyQualified`.
   - Returns Win32 standard `ERROR_MORE_DATA` (234) on undersized buffer queries; registered in `src/registry/system/system_info.rs`.

---

## 2. Current Status & Next Execution
- All changes compile cleanly (`cargo check` & `cargo build --release`).
- Next execution point: Palworld UE5 CEF & network initialization continues with native Linux Steam client (PID 114075).

### 2026-10-18 CEF foreign worker stack probe

A live CEF crash at `libcef.dll+0x78230f7` touched one page below the TEB
StackLimit while the faulting thread's actual stack pointer was far away.
The diagnostic recorded different TIDs in the TEB and kernel. Disassembly
identified MSVC's stack probe reading `gs:0x10`. A narrowly matched fault
recovery path now installs a TEB with the current worker's stack bounds and
restarts the limit calculation. Live validation remains pending. Steam generic
interface requests for `SteamController008` and `SteamInput006` still return
NULL, but their warning is logged once per version to avoid flooding output.
