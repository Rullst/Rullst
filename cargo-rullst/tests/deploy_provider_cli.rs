//! `deploy` reports the provider CLI's real outcome through its exit status.
#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic)]

use std::{
    fs,
    os::unix::fs::PermissionsExt as _,
    path::{Path, PathBuf},
    process::{Command, Output},
};

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("rullst-cli-deploy-{}", rand::random::<u64>()));
        fs::create_dir_all(root.join("tools")).expect("tool directory");
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"deploy-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst = \"12\"\n",
        )
        .expect("fixture manifest");
        fs::write(root.join("Dockerfile"), "FROM scratch\n").expect("existing Dockerfile");
        Self { root }
    }

    fn provider(&self, name: &str, status: u8) {
        let path = self.root.join("tools").join(name);
        fs::write(&path, format!("#!/bin/sh\nexit {status}\n")).expect("provider fixture");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
            .expect("provider permissions");
    }

    fn deploy(&self, platform: &str) -> Output {
        Command::new(env!("CARGO_BIN_EXE_rullst"))
            .current_dir(&self.root)
            .args(["deploy", "--platform", platform])
            .env("RULLST_DISABLE_UPDATE_CHECK", "1")
            .env("NO_COLOR", "1")
            .env("PATH", self.root.join("tools"))
            .output()
            .unwrap_or_else(|error| panic!("run deploy {platform}: {error}"))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn failed_provider_deployments_exit_nonzero_and_missing_tools_stay_advisory() {
    let fixture = Fixture::new();

    let missing = fixture.deploy("fly");
    assert!(missing.status.success(), "{}", text(&missing));
    assert!(text(&missing).contains("not found"));
    assert!(Path::new(&fixture.root.join("fly.toml")).is_file());

    for (platform, program, command) in [
        ("fly", "flyctl", "flyctl deploy"),
        ("railway", "railway", "railway up"),
    ] {
        fixture.provider(program, 3);
        let failed = fixture.deploy(platform);
        assert!(!failed.status.success(), "{}", text(&failed));
        assert!(text(&failed).contains(command), "{}", text(&failed));
        assert!(!text(&failed).contains("successfully deployed"));

        fixture.provider(program, 0);
        let deployed = fixture.deploy(platform);
        assert!(deployed.status.success(), "{}", text(&deployed));
        assert!(text(&deployed).contains("successfully deployed"));
    }
}
