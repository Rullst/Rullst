//! Process contracts for packaging order, failure propagation and signing consent.
use std::process::{Command, Output};

const SIGNING_ENV: [&str; 4] = [
    "RULLST_ANDROID_KEYSTORE",
    "RULLST_ANDROID_KEY_ALIAS",
    "RULLST_ANDROID_STORE_PASSWORD",
    "RULLST_ANDROID_KEY_PASSWORD",
];

fn command(root: &std::path::Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_cargo-rullst"));
    command
        .current_dir(root)
        .env("RULLST_DISABLE_UPDATE_CHECK", "1");
    for name in SIGNING_ENV {
        command.env_remove(name);
    }
    command
}

fn output_text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn release_requires_android_and_never_prints_a_password() {
    let temp = tempfile::tempdir().unwrap();
    let output = command(temp.path())
        .args(["omni", "desktop", "--release"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output_text(&output).contains("requires the android"));
    let output = command(temp.path())
        .args(["omni", "android", "--release"])
        .env("RULLST_ANDROID_STORE_PASSWORD", "never-print-this-secret")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let text = output_text(&output);
    assert!(text.contains("RULLST_ANDROID_KEYSTORE"));
    assert!(!text.contains("never-print-this-secret"));
}

#[cfg(unix)]
mod unix {
    use super::*;
    use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};

    struct Fixture {
        temp: tempfile::TempDir,
        tools: PathBuf,
    }
    impl Fixture {
        fn new() -> Self {
            let temp = tempfile::tempdir().unwrap();
            fs::write(temp.path().join("Cargo.toml"), "[package]\nname='packaging-fixture'\nversion='1.0.0'\n[dependencies]\nrullst='12'\n").unwrap();
            let tools = temp.path().join("tools");
            fs::create_dir(&tools).unwrap();
            let npm = tools.join("npm");
            fs::write(
                &npm,
                r#"#!/bin/sh
echo "$*" >> ../commands.log
if [ "$1" = install ]; then
  /bin/mkdir -p node_modules/@tauri-apps/cli
  printf '{}' > node_modules/@tauri-apps/cli/package.json
fi
if [ "$5 $6" = 'android init' ]; then
  /bin/mkdir -p gen/android/app/src/main/res
  printf '// generated project\n' > gen/android/app/build.gradle.kts
  printf 'tauri default' > gen/android/app/src/main/res/icon-fixture
fi
if [ "$5" = icon ] && [ -d gen/android ]; then
  if [ "$FAIL_FINAL_ICON" = 1 ]; then exit 9; fi
  printf 'application icon' > gen/android/app/src/main/res/icon-fixture
fi
if [ "$5 $6" = 'android build' ]; then exit "${BUILD_EXIT:-0}"; fi
exit 0
"#,
            )
            .unwrap();
            fs::set_permissions(npm, fs::Permissions::from_mode(0o755)).unwrap();
            Self { temp, tools }
        }
        fn command(&self) -> Command {
            let mut result = super::command(self.temp.path());
            result.env("PATH", &self.tools);
            result
        }
        fn scaffold(&self) -> Command {
            let mut result = self.command();
            result.args([
                "make:omni",
                "--platform",
                "android",
                "--backend-url",
                "https://example.invalid",
                "--identifier",
                "dev.rullst.fixture",
            ]);
            result
        }
    }

    #[test]
    fn mobile_icons_are_applied_after_init_and_existing_shells_are_not_overwritten() {
        let fixture = Fixture::new();
        let output = fixture.scaffold().output().unwrap();
        assert!(output.status.success(), "{}", output_text(&output));
        let root = fixture.temp.path();
        let log = fs::read_to_string(root.join("commands.log")).unwrap();
        assert_eq!(
            log.lines().collect::<Vec<_>>(),
            [
                "install",
                "exec --offline -- tauri icon icons/icon.svg",
                "exec --offline -- tauri android init --ci",
                "exec --offline -- tauri icon icons/icon.svg"
            ]
        );
        assert_eq!(
            fs::read_to_string(root.join("omni-app/gen/android/app/src/main/res/icon-fixture"))
                .unwrap(),
            "application icon"
        );
        let svg = fs::read_to_string(root.join("omni-app/icons/icon.svg")).unwrap();
        use base64::Engine as _;
        let encoded = svg
            .split("base64,")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap();
        assert_eq!(
            base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .unwrap(),
            include_bytes!("../src/blueprints/blank/rullst.png")
        );
        assert!(!fixture.scaffold().output().unwrap().status.success());
        assert_eq!(log, fs::read_to_string(root.join("commands.log")).unwrap());
    }

    #[test]
    fn failed_final_icon_pass_is_not_reported_as_success() {
        let fixture = Fixture::new();
        let output = fixture
            .scaffold()
            .env("FAIL_FINAL_ICON", "1")
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output_text(&output).contains("failed to generate platform icon"));
        assert!(!output_text(&output).contains("successfully generated"));
    }

    #[test]
    fn release_build_is_explicit_and_propagates_tool_failure() {
        let fixture = Fixture::new();
        assert!(fixture.scaffold().output().unwrap().status.success());
        let key = fixture.temp.path().join("fixture-key.jks");
        fs::write(&key, "tool fixture, not a signing key").unwrap();
        for exit in ["0", "9"] {
            let output = fixture
                .command()
                .args(["omni", "android", "--release"])
                .env(SIGNING_ENV[0], key.canonicalize().unwrap())
                .env(SIGNING_ENV[1], "fixture-alias")
                .env(SIGNING_ENV[2], "never-print-this-secret")
                .env(SIGNING_ENV[3], "never-print-this-secret")
                .env("BUILD_EXIT", exit)
                .output()
                .unwrap();
            assert_eq!(
                output.status.success(),
                exit == "0",
                "{}",
                output_text(&output)
            );
            assert!(!output_text(&output).contains("never-print-this-secret"));
        }
        let log = fs::read_to_string(fixture.temp.path().join("commands.log")).unwrap();
        assert!(log.ends_with("exec --offline -- tauri android build --apk --ci\n"));
        assert!(!log.contains("never-print-this-secret"));
    }
}
