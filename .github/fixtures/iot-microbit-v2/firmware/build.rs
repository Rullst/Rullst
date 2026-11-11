//! Places `memory.x` on the linker path, links with cortex-m-rt's `link.x`
//! (as a link argument, so a `RUSTFLAGS` override keeps it), writes the
//! synthetic OTA image and records the build profile.

use std::{env, fs, path::PathBuf};

include!("image.rs");

fn main() {
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    fs::copy("memory.x", out.join("memory.x")).expect("copy memory.x");
    fs::write(out.join("firmware.bin"), firmware_image()).expect("write firmware.bin");
    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rustc-link-arg-bins=-Tlink.x");
    // OUT_DIR = <target>/<triple>/<profile>/build/<package>-<hash>/out
    let profile = out
        .ancestors()
        .nth(3)
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_owned();
    println!("cargo:rustc-env=FW_PROFILE={profile}");
    println!("cargo:rerun-if-changed=memory.x");
    println!("cargo:rerun-if-changed=image.rs");
    cli_signed(&out);
}

/// With `RULLST_IOT_SIGNED_DIR` set to a folder holding `image.bin` and what
/// `cargo rullst iot:keygen --out ota.key` and `cargo rullst iot:sign --image
/// image.bin --target microbit-v2` wrote there, the firmware also verifies
/// that CLI-signed update (the `cli_signed` checks). CI builds without it.
fn cli_signed(out: &std::path::Path) {
    println!("cargo:rustc-check-cfg=cfg(cli_signed)");
    println!("cargo:rerun-if-env-changed=RULLST_IOT_SIGNED_DIR");
    let Some(dir) = env::var_os("RULLST_IOT_SIGNED_DIR").map(PathBuf::from) else {
        return;
    };
    for (from, to) in [
        ("image.bin", "cli-image.bin"),
        ("image.bin.manifest", "cli.manifest"),
        ("image.bin.manifest.sig", "cli.sig"),
        ("ota.key.pub.bin", "cli.pub"),
    ] {
        let source = dir.join(from);
        println!("cargo:rerun-if-changed={}", source.display());
        fs::copy(&source, out.join(to)).expect("copy a CLI-signed update file");
    }
    println!("cargo:rustc-cfg=cli_signed");
}
