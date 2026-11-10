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
}
