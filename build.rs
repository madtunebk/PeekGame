use std::{env, path::PathBuf, process::Command};
fn run(command: &mut Command) {
    let status = command
        .status()
        .expect("start native compiler/archive tool");
    assert!(status.success(), "native backend build failed: {command:?}");
}
fn main() {
    let source = "src/api/text/dwrite/backend.c";
    println!("cargo:rerun-if-changed={source}");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    println!("cargo:rerun-if-changed=native/steam_bridge/steam_bridge.c");
    println!("cargo:rerun-if-changed=native/steam_bridge/steam_bridge.h");
    run(Command::new("cc")
        .args(["-std=gnu11", "-O2", "-fPIC", "-Wall", "-Wextra", "-Werror", "-c",
               "native/steam_bridge/steam_bridge.c", "-o"])
        .arg(out.join("steam_bridge.o")));
    run(Command::new("ar").arg("crs").arg(out.join("libpeekgame_steam.a"))
        .arg(out.join("steam_bridge.o")));
    println!("cargo:rustc-link-lib=static=peekgame_steam");
    println!("cargo:rustc-link-lib=dl");
    println!("cargo:rustc-link-lib=pthread");
    let flags = Command::new("pkg-config")
        .args(["--cflags", "fontconfig"])
        .output()
        .expect("pkg-config fontconfig");
    assert!(
        flags.status.success(),
        "install the fontconfig development headers"
    );
    run(Command::new("cc")
        .args(["-std=gnu11", "-O2", "-fPIC", "-Wall", "-Wextra", "-Werror"])
        .args(String::from_utf8(flags.stdout).unwrap().split_whitespace())
        .arg("-c")
        .arg(source)
        .arg("-o")
        .arg(out.join("dwrite.o")));
    run(Command::new("ar")
        .arg("crs")
        .arg(out.join("libpeekgame_dwrite.a"))
        .arg(out.join("dwrite.o")));
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=peekgame_dwrite");
    println!("cargo:rustc-link-lib=fontconfig");
    println!("cargo:rerun-if-changed=tools/gen_gl_bridge.py");
    println!("cargo:rerun-if-changed=src/host/graphics/glx/backend.c");
    println!("cargo:rerun-if-changed=/usr/include/GL/gl.h");
    println!("cargo:rerun-if-changed=/usr/include/GL/glext.h");
    run(Command::new("python3")
        .arg("tools/gen_gl_bridge.py")
        .arg(&out));
    for (source, name) in [
        (out.join("gl_thunks.c"), "gl_thunks.o"),
        (PathBuf::from("src/host/graphics/glx/backend.c"), "glx.o"),
    ] {
        run(Command::new("cc")
            .args(["-std=gnu11", "-O2", "-fPIC", "-Wall", "-Wextra", "-Werror"])
            .arg("-c")
            .arg(source)
            .arg("-o")
            .arg(out.join(name)));
    }
    run(Command::new("ar")
        .arg("crs")
        .arg(out.join("libpeekgame_gl.a"))
        .arg(out.join("gl_thunks.o"))
        .arg(out.join("glx.o")));
    println!("cargo:rustc-link-lib=static=peekgame_gl");
    println!("cargo:rustc-link-lib=GL");
    println!("cargo:rerun-if-changed=src/host/x11/modes.c");
    run(Command::new("cc")
        .args([
            "-std=c11",
            "-O2",
            "-fPIC",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-c",
            "src/host/x11/modes.c",
            "-o",
        ])
        .arg(out.join("x11_modes.o")));
    run(Command::new("ar")
        .arg("crs")
        .arg(out.join("libpeekgame_x11.a"))
        .arg(out.join("x11_modes.o")));
    println!("cargo:rustc-link-lib=static=peekgame_x11");
    let nls = "src/host/nls/backend.c";
    println!("cargo:rerun-if-changed={nls}");
    let flags = Command::new("pkg-config")
        .args(["--cflags", "icu-i18n"])
        .output()
        .expect("pkg-config ICU");
    assert!(
        flags.status.success(),
        "install ICU development headers (libicu-dev)"
    );
    run(Command::new("cc")
        .args(["-std=c11", "-O2", "-fPIC", "-Wall", "-Wextra", "-Werror"])
        .args(String::from_utf8(flags.stdout).unwrap().split_whitespace())
        .args(["-c", nls, "-o"])
        .arg(out.join("nls.o")));
    println!("cargo:rerun-if-changed=src/host/nls/pattern.c");
    run(Command::new("cc")
        .args([
            "-std=c11",
            "-O2",
            "-fPIC",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-c",
            "src/host/nls/pattern.c",
            "-o",
        ])
        .arg(out.join("nls_pattern.o")));
    println!("cargo:rerun-if-changed=src/host/nls/calendar.c");
    run(Command::new("cc")
        .args([
            "-std=c11",
            "-O2",
            "-fPIC",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-c",
            "src/host/nls/calendar.c",
            "-o",
        ])
        .arg(out.join("nls_calendar.o")));
    println!("cargo:rerun-if-changed=src/host/nls/date_pattern.c");
    run(Command::new("cc")
        .args([
            "-std=c11",
            "-O2",
            "-fPIC",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-c",
            "src/host/nls/date_pattern.c",
            "-o",
        ])
        .arg(out.join("nls_date.o")));
    run(Command::new("ar")
        .arg("crs")
        .arg(out.join("libpeekgame_nls.a"))
        .arg(out.join("nls_date.o"))
        .arg(out.join("nls_calendar.o"))
        .arg(out.join("nls.o"))
        .arg(out.join("nls_pattern.o")));
    println!("cargo:rustc-link-lib=static=peekgame_nls");
    for library in ["icui18n", "icuuc", "icudata"] {
        println!("cargo:rustc-link-lib={library}");
    }
}
