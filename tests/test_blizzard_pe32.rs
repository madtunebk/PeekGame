
#[test]
fn test_blizzard_32bit_binaries() {
    let paths = [
        "/media/nobus/7CB22B82B22B3FCE/ProgramData/Battle.net/Agent/Agent.exe",
        "/media/nobus/7CB22B82B22B3FCE/Games/World of Warcraft/World of Warcraft Launcher.exe",
        "/media/nobus/7CB22B82B22B3FCE/Games/World of Warcraft/_classic_/BlizzardError.exe",
    ];
    for p in paths {
        if let Ok(bytes) = std::fs::read(p) {
            let img = game_imports::pe32::parse(&bytes).unwrap_or_else(|e| panic!("failed to parse {p}: {e}"));
            println!("SUCCESS: {} -> machine={:#x}", p, img.machine);
            println!("  image_base={:#x}, entry={:#x}, sections={}, imports={}", img.image_base, img.entry_rva, img.sections.len(), img.imports.len());
            assert_eq!(img.machine, 0x14c);
            assert!(img.imports.len() > 0);
        }
    }
}
