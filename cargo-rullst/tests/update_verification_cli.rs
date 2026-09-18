//! Process composition with an explicitly caller-installed verifier fixture.
//! This tests CLI policy/bytes/cleanup, not a substitute for real provenance.
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn target() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") if cfg!(target_env = "gnu") => "x86_64-unknown-linux-gnu",
        ("windows", "x86_64") if cfg!(target_env = "msvc") => "x86_64-pc-windows-msvc",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        _ => panic!("fixture requires one of the four supported native targets"),
    }
}

fn invoke(base: &Path, downloads: &Path, bin: &Path) -> Output {
    let old = std::env::var_os("PATH").unwrap_or_default();
    let path =
        std::env::join_paths(std::iter::once(bin.to_path_buf()).chain(std::env::split_paths(&old)))
            .unwrap();
    Command::new(env!("CARGO_BIN_EXE_cargo-rullst"))
        .args([
            "update",
            "verify",
            "--to",
            env!("CARGO_PKG_VERSION"),
            "--directory",
        ])
        .arg(downloads)
        .arg("--json")
        .env("PATH", path)
        .env("XDG_CACHE_HOME", base)
        .env("LOCALAPPDATA", base)
        .env("CARGO_NET_OFFLINE", "false")
        .env("RULLST_DISABLE_UPDATE_CHECK", "true")
        .env("RULLST_VERIFIER_MARKER", base.join("verifier-called"))
        .output()
        .unwrap()
}

#[test]
fn verification_composes_policy_with_exact_bytes_and_never_installs_or_executes_candidates() {
    #[cfg(windows)]
    let temp = tempfile::tempdir_in(std::env::var_os("LOCALAPPDATA").unwrap()).unwrap();
    #[cfg(not(windows))]
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&base, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let bin = base.join("bin");
    let downloads = base.join("downloads");
    fs::create_dir(&bin).unwrap();
    fs::create_dir(&downloads).unwrap();
    let source = base.join("verifier.rs");
    fs::write(&source, r#"
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let expected = format!("https://github.com/Rullst/Rullst/.github/workflows/release.yml@refs/tags/v{}", env!("RULLST_FIXTURE_VERSION"));
    assert_eq!(&args[..2], ["attestation", "verify"]);
    for pair in [["--hostname", "github.com"], ["--repo", "Rullst/Rullst"],
        ["--cert-identity", expected.as_str()], ["--cert-oidc-issuer", "https://token.actions.githubusercontent.com"],
        ["--predicate-type", "https://slsa.dev/provenance/v1"]] {
        assert!(args.windows(2).any(|v| v == pair));
    }
    assert!(args.iter().any(|v| v == "--deny-self-hosted-runners"));
    assert!(!args.iter().any(|v| v == "--signer-workflow"));
    assert!(!std::fs::read(&args[2]).unwrap().is_empty());
    let marker = std::env::var_os("RULLST_VERIFIER_MARKER").unwrap();
    std::fs::write(marker, &args[2]).unwrap();
}
"#).unwrap();
    let verifier = bin.join(if cfg!(windows) { "gh.exe" } else { "gh" });
    let compiled = Command::new("rustc")
        .arg(&source)
        .arg("-o")
        .arg(&verifier)
        .env("RULLST_FIXTURE_VERSION", env!("CARGO_PKG_VERSION"))
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let body = b"intentionally not an executable; must only be hashed";
    let suffix = if cfg!(windows) { ".exe" } else { "" };
    let version = env!("CARGO_PKG_VERSION");
    let records = ["cargo-rullst", "rullst"].map(|name| {
        let filename = format!("{name}-{version}-{}{suffix}", target());
        fs::write(downloads.join(&filename), body).unwrap();
        json!({"name":filename,"executable":name,"bytes":body.len(),"sha256":hex::encode(Sha256::digest(body))})
    });
    let manifest = json!({"schema":"rullst.cli-artifacts.v1","version":version,"target":target(),
        "source_commit":"a".repeat(40),"repository":"Rullst/Rullst","release_tag":format!("v{version}"),
        "build_runner":"fixture","files":records});
    let manifest_path = downloads.join(format!("cli-manifest-{}.json", target()));
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let output = invoke(&base, &downloads, &bin);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["schema_version"], "rullst.update-verification.v1");
    assert_eq!(report["authority"]["artifact_verified"], true);
    for (key, value) in report["authority"].as_object().unwrap() {
        assert_eq!(value, &(key == "artifact_verified"));
    }
    let snapshot = fs::read_to_string(base.join("verifier-called")).unwrap();
    assert!(
        !Path::new(&snapshot).exists(),
        "private snapshot must be cleaned up"
    );
    assert_eq!(
        fs::read_dir(base.join("rullst-update-v1")).unwrap().count(),
        0
    );
    let first = downloads.join(records[0]["name"].as_str().unwrap());
    assert_eq!(fs::read(&first).unwrap(), body);
    fs::write(&first, vec![b'x'; body.len()]).unwrap();
    let rejected = invoke(&base, &downloads, &bin);
    assert!(!rejected.status.success());
    assert!(rejected.stdout.is_empty());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("executable digest does not match"));
    fs::remove_file(base.join("verifier-called")).unwrap();
    let mut invalid = manifest;
    invalid["release_tag"] = serde_json::Value::Null;
    fs::write(&manifest_path, serde_json::to_vec(&invalid).unwrap()).unwrap();
    assert!(!invoke(&base, &downloads, &bin).status.success());
    assert!(
        !base.join("verifier-called").exists(),
        "invalid inventory must fail before verifier execution"
    );
}

#[test]
fn staging_offline_rejection_precedes_registry_network_and_cache_creation() {
    for explicit in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let missing = directory.path().join("absent-cache");
        let mut command = Command::new(env!("CARGO_BIN_EXE_cargo-rullst"));
        command
            .args([
                "update",
                "stage",
                "--to",
                env!("CARGO_PKG_VERSION"),
                "--json",
            ])
            .env("XDG_CACHE_HOME", &missing)
            .env("LOCALAPPDATA", &missing)
            .env("RULLST_DISABLE_UPDATE_CHECK", "true");
        if explicit {
            command.arg("--offline").env_remove("CARGO_NET_OFFLINE");
        } else {
            command.env("CARGO_NET_OFFLINE", "true");
        }
        let result = command.output().unwrap();
        assert!(!result.status.success());
        assert!(
            String::from_utf8_lossy(&result.stderr)
                .contains("offline mode forbids authenticated downloads")
        );
        assert!(!missing.exists());
    }
}

#[test]
fn installation_review_offline_rejection_does_not_create_or_inspect_destinations() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("not-created");
    let output = Command::new(env!("CARGO_BIN_EXE_cargo-rullst"))
        .args([
            "update",
            "install",
            "review",
            "--to",
            env!("CARGO_PKG_VERSION"),
            "--directory",
        ])
        .arg(directory.path().join("missing-artifacts"))
        .arg("--root")
        .arg(&root)
        .args(["--offline", "--json"])
        .env("RULLST_DISABLE_UPDATE_CHECK", "true")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("offline mode forbids authenticated installation operations")
    );
    assert!(!root.exists());
}

#[test]
fn installation_apply_and_recovery_require_explicit_approval_before_destination_io() {
    for action in ["apply", "recover"] {
        for approval in [
            None,
            Some("invalid-digest"),
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        ] {
            let directory = tempfile::tempdir().unwrap();
            let root = directory.path().join("not-created");
            let mut command = Command::new(env!("CARGO_BIN_EXE_cargo-rullst"));
            command
                .args(["update", "install", action, "--root"])
                .arg(&root)
                .env("CARGO_NET_OFFLINE", "false")
                .env("RULLST_DISABLE_UPDATE_CHECK", "true");
            if action == "apply" {
                command
                    .args(["--to", env!("CARGO_PKG_VERSION"), "--directory"])
                    .arg(directory.path().join("not-inspected"));
            }
            if let Some(value) = approval {
                command.args(["--approved-review", value]);
            }
            if approval.is_some_and(|value| value.len() == 64) {
                command.arg("--offline");
            }
            let result = command.output().unwrap();
            assert!(!result.status.success());
            assert!(!root.exists());
            assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
            let error = String::from_utf8_lossy(&result.stderr);
            assert!(
                error.contains("approved-review")
                    || error.contains("approval")
                    || error.contains("offline mode"),
                "{error}"
            );
        }
    }
}
