use super::{Fixture, text};
use std::{fs, os::unix::fs::PermissionsExt as _, process::Command};

#[test]
fn preparation_ignores_a_project_supplied_rustup_toolchain() {
    let fixture = Fixture::current();
    let marker = fixture.base.join("project-toolchain-executed");
    for tool in ["cargo", "rustc"] {
        let path = fixture.app.join("tc/bin").join(tool);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            format!("#!/bin/sh\ntouch '{}'\nexit 1\n", marker.display()),
        )
        .unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    fs::create_dir_all(fixture.app.join("tc/lib")).unwrap();
    fs::write(fixture.app.join("tc/lib/.keep"), "").unwrap();
    // rustup resolves this path inside whichever copy it runs in.
    fs::write(
        fixture.app.join("rust-toolchain.toml"),
        "[toolchain]\npath = \"/proc/self/cwd/tc\"\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_cargo-rullst"))
        .args(["update", "project", "prepare", "--project"])
        .arg(&fixture.app)
        .arg("--json")
        .env("XDG_CACHE_HOME", &fixture.base)
        .env("LOCALAPPDATA", &fixture.base)
        .env("CARGO_NET_OFFLINE", "true")
        .env("RULLST_DISABLE_UPDATE_CHECK", "true")
        // Exercise a direct invocation that no rustup proxy pinned.
        .env_remove("RUSTUP_TOOLCHAIN")
        .output()
        .unwrap();
    assert!(
        !marker.exists(),
        "preparation executed the project toolchain: {}",
        text(&output)
    );
    assert!(
        output.status.success() || text(&output).contains("RUSTUP_TOOLCHAIN"),
        "{}",
        text(&output)
    );
}
