//! Process-level regression for the standalone badge synchronizer.

#![allow(clippy::expect_used, clippy::panic)]

use std::{fs, path::PathBuf, process::Command};

struct WorkspaceFixture(PathBuf);

impl WorkspaceFixture {
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("rullst-sync-badges-{}", rand::random::<u64>()));
        fs::create_dir_all(root.join("cargo-rullst/nested")).expect("fixture directories");
        fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"cargo-rullst\"]\n",
        )
        .expect("workspace manifest");
        fs::write(
            root.join("cargo-rullst/Cargo.toml"),
            "[package]\nname = \"cargo-rullst\"\nversion = \"12.0.0-rc.1\"\n",
        )
        .expect("package manifest");
        fs::write(
            root.join("README.md"),
            "![Status: v5.0.0](https://img.shields.io/badge/Status-v5.0.0-emerald)\n",
        )
        .expect("English README");
        Self(root)
    }

    fn run_from(&self, relative: &str) -> String {
        let output = Command::new(env!("CARGO_BIN_EXE_sync-badges"))
            .current_dir(self.0.join(relative))
            .output()
            .expect("badge synchronizer");
        assert!(
            output.status.success(),
            "badge synchronizer failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }
}

impl Drop for WorkspaceFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn synchronizer_discovers_workspace_package_and_nested_invocations() {
    let fixture = WorkspaceFixture::new();
    let root_output = fixture.run_from("");
    assert!(root_output.contains("12.0.0-rc.1"));
    assert!(root_output.contains("README.pt.md"));

    fs::write(
        fixture.0.join("README.pt.md"),
        "![Status: v6.0.0](https://img.shields.io/badge/Status-v6.0.0-emerald)\n",
    )
    .expect("Portuguese README");
    fixture.run_from("cargo-rullst");
    fixture.run_from("cargo-rullst/nested");

    for readme in ["README.md", "README.pt.md"] {
        let content = fs::read_to_string(fixture.0.join(readme)).expect("updated README");
        assert!(content.contains("Status-v12.0.0-rc.1-emerald"));
        assert!(!content.contains("Status-v5.0.0"));
        assert!(!content.contains("Status-v6.0.0"));
    }
}
