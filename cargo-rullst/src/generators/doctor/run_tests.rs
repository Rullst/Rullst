//! `cargo rullst doctor` end to end in a child test process against fake
//! `rustc`, `cargo` and `rustup` scripts: the header, JSON-only output, the
//! `--fix` decision and the exit status.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use super::*;

/// Runs the doctor when a parent test names a report file; otherwise does
/// nothing. Markers separate the doctor's output from the harness's.
#[test]
fn doctor_child_fixture() {
    let Some(report) = std::env::var_os("RULLST_DOCTOR_REPORT") else {
        return;
    };
    let mode = std::env::var("RULLST_DOCTOR_MODE").unwrap();
    let mut stdout = io::stdout();
    stdout.write_all(b"<<<doctor\n").unwrap();
    stdout.flush().unwrap();
    let result = match mode.as_str() {
        "human" => run_doctor(false),
        "json" => run(DoctorOptions {
            fix: false,
            json: true,
        }),
        "json-fix" => run(DoctorOptions {
            fix: true,
            json: true,
        }),
        other => panic!("unknown mode {other}"),
    };
    stdout.write_all(b"doctor>>>\n").unwrap();
    stdout.flush().unwrap();
    let outcome = match result {
        Ok(()) => "ok".to_string(),
        Err(error) => error.to_string(),
    };
    std::fs::write(report, outcome).unwrap();
}

#[cfg(unix)]
mod unix {
    use super::*;
    use std::path::Path;

    const CARGO: &str = r#"case "$1" in
  --version) echo 'cargo 1.98.1 (fake)' ;;
  fmt) echo 'rustfmt 1.8.0' ;;
  clippy)
    if [ -f "$FAKE_STATE/clippy" ]; then echo 'clippy 0.1.98'; else exit 101; fi ;;
  *) exit 101 ;;
esac"#;

    /// `install` adds clippy; `install-fails` adds it but exits 1; `noop`
    /// exits 0 without adding it.
    const RUSTUP: &str = r#"[ "$1" = component ] || exit 1
case "$FAKE_RUSTUP" in
  install) : > "$FAKE_STATE/clippy" ;;
  install-fails) : > "$FAKE_STATE/clippy"; exit 1 ;;
esac"#;

    fn script(bin: &Path, name: &str, body: &str) {
        use std::os::unix::fs::PermissionsExt;
        let path = bin.join(name);
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        // Another test's fork may briefly hold the new file open for writing
        // (ETXTBSY); run it once so the doctor's probes never see that.
        for _ in 0..100 {
            match Command::new(&path).arg("--version").output() {
                Err(error) if error.raw_os_error() == Some(26) => {
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
                _ => return,
            }
        }
        panic!("{name} stayed busy");
    }

    fn tools(rustc: bool, clippy: bool) -> tempfile::TempDir {
        let tools = tempfile::tempdir().unwrap();
        let bin = tools.path().join("bin");
        std::fs::create_dir(&bin).unwrap();
        std::fs::create_dir(tools.path().join("state")).unwrap();
        if rustc {
            script(&bin, "rustc", "echo 'rustc 1.98.1 (fake)'");
        }
        script(&bin, "cargo", CARGO);
        script(&bin, "rustup", RUSTUP);
        if clippy {
            std::fs::write(tools.path().join("state/clippy"), "").unwrap();
        }
        tools
    }

    /// The outcome and standard output of the doctor in `mode`, run outside
    /// any project with only the fake tools on `PATH`.
    fn doctor(tools: &tempfile::TempDir, mode: &str, rustup: &str) -> (String, String) {
        let outside = tempfile::tempdir().unwrap();
        let report = tools.path().join("report");
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "generators::doctor::run_tests::doctor_child_fixture",
                "--nocapture",
            ])
            .current_dir(outside.path())
            .env("PATH", tools.path().join("bin"))
            .env("FAKE_STATE", tools.path().join("state"))
            .env("FAKE_RUSTUP", rustup)
            .env("RULLST_DOCTOR_REPORT", &report)
            .env("RULLST_DOCTOR_MODE", mode)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        let shown = stdout
            .split("<<<doctor\n")
            .nth(1)
            .and_then(|rest| rest.split("doctor>>>\n").next())
            .unwrap()
            .to_string();
        (std::fs::read_to_string(report).unwrap(), shown)
    }

    /// The components check of a JSON report that is the whole output.
    fn components(shown: &str) -> serde_json::Value {
        assert!(shown.starts_with('{'), "{shown}");
        let report: serde_json::Value = serde_json::from_str(shown).unwrap();
        report["groups"][0]["checks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|check| check["id"] == "toolchain.components")
            .unwrap()
            .clone()
    }

    #[test]
    fn a_healthy_toolchain_prints_the_header_and_passes() {
        let tools = tools(true, true);
        let (outcome, shown) = doctor(&tools, "human", "noop");
        assert_eq!(outcome, "ok");
        let header = format!("Rullst doctor · v{}\n\n", env!("CARGO_PKG_VERSION"));
        assert!(shown.starts_with(&header), "{shown}");
        assert!(shown.contains("Summary  "), "{shown}");
        assert!(!shown.contains("Fixing"), "{shown}");
    }

    #[test]
    fn fix_installs_missing_components_and_keeps_json_alone_on_stdout() {
        let tools = tools(true, false);
        let (outcome, shown) = doctor(&tools, "json-fix", "install");
        assert_eq!(outcome, "ok");
        let check = components(&shown);
        assert_eq!(check["status"], "pass");
        assert_eq!(check["detail"], "installed by --fix");
    }

    #[test]
    fn without_fix_missing_components_stay_a_warning() {
        let tools = tools(true, false);
        let (outcome, shown) = doctor(&tools, "json", "install");
        assert_eq!(outcome, "ok");
        let check = components(&shown);
        assert_eq!(check["status"], "warn");
        assert!(!tools.path().join("state/clippy").exists());
    }

    #[test]
    fn a_fix_counts_only_when_rustup_succeeds_and_the_tools_answer() {
        for rustup in ["install-fails", "noop"] {
            let tools = tools(true, false);
            let (outcome, shown) = doctor(&tools, "json-fix", rustup);
            assert_eq!(outcome, "ok", "{rustup}");
            let detail = components(&shown)["detail"].as_str().unwrap().to_string();
            assert!(
                detail.starts_with("--fix could not install them"),
                "{rustup}: {detail}"
            );
        }
    }

    #[test]
    fn a_failing_check_fails_the_doctor() {
        let tools = tools(false, true);
        let (outcome, shown) = doctor(&tools, "json", "noop");
        assert_eq!(outcome, "cargo rullst doctor found 1 failing check(s)");
        let report: serde_json::Value = serde_json::from_str(&shown).unwrap();
        assert_eq!(report["ok"], false);
    }
}
