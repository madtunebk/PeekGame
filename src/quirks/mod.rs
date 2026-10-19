//! Per-game behaviour quirks: the one place game-specific workarounds live.
//!
//! A hook point in the main code asks [`active`] (or a purpose-built helper such as
//! [`suppresses_builtin`]) instead of hard-coding a game's name inline. Each quirk is
//! a named behaviour change; [`PROFILES`] maps a game — by its EXE file stem,
//! case-insensitive — to the quirks it needs. Keeping them here, data-driven and
//! greppable, stops workarounds from scattering through the code and being forgotten.
//!
//! A quirk is the exception, not the default: the main code's normal path is what runs
//! for every game not listed. When a workaround turns out to be needed for a whole
//! engine rather than one title, prefer an engine predicate (see [`is_godot`]) over
//! listing every game by name. Record why each quirk exists in the engine ledger under
//! `docs/engines/`.
use crate::images;
use std::path::Path;

/// A named behaviour change requested for specific games.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Quirk {
    /// Hide `dinput8.dll` from `LoadLibrary`/`GetModuleHandle`. Godot resolves a
    /// freshly loaded DLL's exports by reading its module image directly; PeekGame's
    /// builtin module handle is not a mapped PE image, so letting the load succeed
    /// makes the engine fault dereferencing the handle (read at base+0x2dc). Godot
    /// games don't need DirectInput, so hiding the DLL lets them fall back cleanly.
    /// Also applied to any Godot game via [`is_godot`], not only those listed below.
    SuppressDinput8,
    /// Force every WinHTTP request to fail (as if offline), so the game does not gate
    /// menus on online services it cannot finish. For DOOM Eternal this also stops the
    /// PlayFab Party (PartyWin.dll) system from initializing, so the Settings menu's
    /// `idShellDataManager::RefreshChatSpeechProfiles` (text-to-speech voice list, from
    /// the game's own Party SDK) never runs — it otherwise waits for Party to connect,
    /// which cannot happen under PeekGame, timing out ~15-30s and triggering a later DC.
    ForceHttpOffline,
    /// Do not start DOOM Eternal's PlayFab Party text-to-speech profile lookup.
    /// The game itself skips enumeration when net_enableTextToSpeech is false.
    DisableDoomTextToSpeech,
}

/// Game EXE stem (lowercased, without extension) -> the quirks it needs.
const PROFILES: &[(&str, &[Quirk])] = &[
    // Godot titles that crash on a builtin dinput8 (see Quirk::SuppressDinput8). These
    // also ship a .pck, so is_godot() would catch them; listing keeps the case
    // explicit and gives a worked example of the table.
    ("gundam survivors", &[Quirk::SuppressDinput8]),
    ("doometernalx64vk", &[Quirk::DisableDoomTextToSpeech]),
    // (No default offline for DOOM Eternal: it runs online by default. Set
    // PEEKGAME_HTTP_OFFLINE=1 to opt into offline — which also sidesteps the Settings
    // menu's RefreshChatSpeechProfiles (PlayFab Party TTS) hang and the ~2-min DC.)
];

/// Add startup cvars for game-specific behavior before the guest sees its command line.
/// An explicit user-provided value wins.
pub fn game_args(exe: &Path, args: &[String]) -> Vec<String> {
    let stem = exe.file_stem().map(|s| s.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    let mut out = args.to_vec();
    if listed(&stem, Quirk::DisableDoomTextToSpeech)
        && !args.iter().any(|arg| arg.eq_ignore_ascii_case("+net_enableTextToSpeech")
            || arg.eq_ignore_ascii_case("net_enableTextToSpeech"))
    {
        out.extend(["+net_enableTextToSpeech".into(), "0".into()]);
        eprintln!("PeekGame: requesting DOOM to skip text-to-speech profile lookup (network settings unchanged)");
    }
    out
}

/// The running game's EXE file stem, lowercased (e.g. "gundam survivors").
fn current_stem() -> Option<String> {
    images::exe().map(|m| {
        Path::new(&m.name)
            .file_stem()
            .map(|s| s.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default()
    })
}

/// A Godot game: its EXE sits beside a Godot pack (`<stem>.pck`). This catches Godot
/// titles not listed in [`PROFILES`] for engine-wide quirks. (Single-file exports that
/// embed the pack are not detected; list those by name.)
fn is_godot() -> bool {
    images::exe().is_some_and(|m| m.path.with_extension("pck").exists())
}
/// An Unreal Engine shipping build: the EXE follows the UE4/UE5 `*-Win64-Shipping.exe`
/// naming. These default to an uncapped IMMEDIATE swapchain when vsync is off, which
/// pins the GPU at 100% and tears; [`prefers_vsync`] paces them to the refresh.
fn is_unreal() -> bool {
    current_stem().is_some_and(|s| unreal_stem(&s))
}
/// An Unreal shipping-build EXE stem (lowercased). Split out so it is unit-testable
/// without a registered EXE.
fn unreal_stem(stem: &str) -> bool {
    stem.ends_with("-win64-shipping")
}
/// Whether to default this game's swapchain to FIFO (vsync) when it would otherwise
/// present uncapped, so the GPU paces to the display refresh instead of rendering
/// flat out. Consulted by `create_swapchain`; `PEEKGAME_VULKAN_PRESENT` overrides it.
pub(crate) fn prefers_vsync() -> bool {
    is_unreal()
}

/// Whether a game (by lowercased EXE stem) has `quirk`, from [`PROFILES`] alone.
fn listed(stem: &str, quirk: Quirk) -> bool {
    PROFILES
        .iter()
        .any(|(game, quirks)| *game == stem && quirks.contains(&quirk))
}

/// Is `quirk` requested for the game now running?
pub(crate) fn active(quirk: Quirk) -> bool {
    current_stem().is_some_and(|stem| listed(&stem, quirk))
}

/// Whether `dll` (any spelling) is dinput8, given the running game's quirk state.
/// Split from [`suppresses_builtin`] so the name/quirk logic is unit-testable without
/// a registered EXE.
fn suppresses_builtin_impl(dll: &str, suppress_dinput8: bool) -> bool {
    let name = images::normalize(dll)
        .unwrap_or_else(|| dll.into())
        .to_ascii_lowercase();
    name == "dinput8.dll" && suppress_dinput8
}

/// Should `dll` be hidden from `LoadLibrary`/`GetModuleHandle` for this game, so the
/// game falls back as if PeekGame did not implement it? Consulted wherever a builtin
/// module handle would otherwise be returned.
pub(crate) fn suppresses_builtin(dll: &str) -> bool {
    suppresses_builtin_impl(dll, active(Quirk::SuppressDinput8) || is_godot())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listed_games_get_their_quirks() {
        assert!(listed("gundam survivors", Quirk::SuppressDinput8));
        assert!(!listed("doometernalx64vk", Quirk::ForceHttpOffline)); // online by default
        assert!(listed("doometernalx64vk", Quirk::DisableDoomTextToSpeech));
        assert!(!listed("doometernalx64vk", Quirk::SuppressDinput8));
        assert!(!listed("some other game", Quirk::SuppressDinput8));
    }

    #[test]
    fn doom_speech_override_keeps_other_arguments_and_respects_explicit_value() {
        let exe = Path::new("/games/DOOMEternalx64vk.exe");
        assert_eq!(game_args(exe, &["-windowed".into()]),
            ["-windowed", "+net_enableTextToSpeech", "0"]);
        assert_eq!(game_args(exe, &["+net_enableTextToSpeech".into(), "1".into()]),
            ["+net_enableTextToSpeech", "1"]);
        assert!(game_args(Path::new("other.exe"), &[]).is_empty());
    }

    #[test]
    fn suppresses_only_dinput8_and_only_when_requested() {
        // dinput8, by several spellings, suppressed when the quirk (or Godot) applies.
        assert!(suppresses_builtin_impl("dinput8.dll", true));
        assert!(suppresses_builtin_impl("DINPUT8.DLL", true));
        assert!(suppresses_builtin_impl("dinput8", true));
        // Never suppressed for games without the quirk.
        assert!(!suppresses_builtin_impl("dinput8.dll", false));
        // Other builtins are never touched by this quirk.
        assert!(!suppresses_builtin_impl("xinput1_3.dll", true));
        assert!(!suppresses_builtin_impl("user32.dll", true));
    }

    #[test]
    fn unreal_shipping_builds_are_recognised() {
        assert!(unreal_stem("palworld-win64-shipping"));
        assert!(unreal_stem("fortniteclient-win64-shipping"));
        assert!(!unreal_stem("doometernalx64vk"));
        assert!(!unreal_stem("shipping")); // no hyphen prefix
    }
}
