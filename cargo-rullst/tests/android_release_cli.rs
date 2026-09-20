//! Native process/selection protocol fixtures. These do not sign or verify APKs.
//! The hosted Android workflow separately supplies the real SDK/JDK and APK.
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn success(output: &Output) {
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

const TOOL: &str = r#"
use std::{env, fs, path::Path};
fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    let mode = env::var("APK_FIXTURE_MODE").unwrap();
    let root = env::var("APK_FIXTURE_ROOT").unwrap();
    let apk = Path::new(&root).join("omni-app/gen/android/app/build/outputs/apk/arm64/release/app-arm64-release.apk");
    if env::current_exe().unwrap().file_name().unwrap().to_str().unwrap().starts_with("java") {
        for name in ["RULLST_ANDROID_KEYSTORE", "RULLST_ANDROID_KEY_ALIAS", "RULLST_ANDROID_STORE_PASSWORD", "RULLST_ANDROID_KEY_PASSWORD", "JAVA_TOOL_OPTIONS", "JDK_JAVA_OPTIONS", "_JAVA_OPTIONS"] {
            assert!(env::var_os(name).is_none(), "signing environment must not reach verifier");
        }
        assert_eq!(&args[..1], ["-jar"]);
        assert_eq!(&args[2..6], ["verify", "--verbose", "--print-certs", "-Werr"]);
        let snapshot = args.last().unwrap();
        assert_ne!(Path::new(snapshot), apk);
        assert_eq!(fs::read(snapshot).unwrap(), b"fixture APK bytes, not a real APK");
        fs::write(Path::new(&root).join("snapshot-path"), snapshot).unwrap();
        if mode == "verify-failure" { eprintln!("never-print-fixture-password"); std::process::exit(9); }
        if mode == "mutate-original" { fs::write(&apk, "changed").unwrap(); }
        if mode == "mutate-snapshot" { fs::write(snapshot, "changed").unwrap(); }
        if mode == "huge-report" { println!("{}", "x".repeat(70000)); }
        let digest = if mode == "wrong-key" { "00".repeat(32) } else { env::var("APK_FIXTURE_CERT_DIGEST").unwrap() };
        println!("Verifies\nNumber of signers: 1\nSigner #1 certificate SHA-256 digest: {digest}");
        if mode == "multiple-signers" { println!("Number of signers: 2"); }
    } else {
        if args == ["tauri", "--version"] { println!("fixture Tauri"); return; }
        assert_eq!(args, ["tauri", "android", "build", "--apk", "--ci", "--target", "aarch64"]);
        assert_eq!(env::var("RULLST_ANDROID_STORE_PASSWORD").unwrap(), "never-print-fixture-password");
        fs::write(Path::new(&root).join("build-started"), "yes").unwrap();
        if mode == "build-failure" { eprintln!("never-print-fixture-password"); std::process::exit(9); }
        if mode == "missing" || mode == "stale" { return; }
        fs::create_dir_all(apk.parent().unwrap()).unwrap();
        fs::write(&apk, b"fixture APK bytes, not a real APK").unwrap();
        if mode == "ambiguous" { fs::write(apk.with_file_name("other-release.apk"), b"other").unwrap(); }
        if mode == "oversized" { fs::File::create(&apk).unwrap().set_len(512 * 1024 * 1024 + 1).unwrap(); }
    }
}
"#;

#[test]
fn release_binds_fresh_selected_bytes_and_expected_certificate_without_exposing_secrets() {
    let tools = tempfile::tempdir().unwrap();
    let source = tools.path().join("tool.rs");
    fs::write(&source, TOOL).unwrap();
    let executable = tools
        .path()
        .join(format!("cargo{}", std::env::consts::EXE_SUFFIX));
    success(
        &Command::new("rustc")
            .arg(&source)
            .arg("-o")
            .arg(&executable)
            .output()
            .unwrap(),
    );
    fs::copy(
        &executable,
        tools
            .path()
            .join(format!("java{}", std::env::consts::EXE_SUFFIX)),
    )
    .unwrap();
    // Keep OS process-lifecycle helpers (`ps`/`kill` or `taskkill`) available.
    // The absolute fixture directory still takes precedence for Cargo and Java.
    let fixture_path = std::env::join_paths(
        std::iter::once(tools.path().to_path_buf()).chain(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        )),
    )
    .unwrap();
    for mode in [
        "valid",
        "selected",
        "build-failure",
        "missing",
        "stale",
        "ambiguous",
        "wrong-key",
        "verify-failure",
        "multiple-signers",
        "oversized",
        "huge-report",
        "mutate-original",
        "mutate-snapshot",
        "escape",
    ] {
        let app = tempfile::Builder::new()
            .prefix("rullst android 'literal' ")
            .tempdir()
            .unwrap();
        let root = app.path().canonicalize().unwrap();
        let gradle = root.join("omni-app/gen/android/app/build.gradle.kts");
        fs::create_dir_all(gradle.parent().unwrap()).unwrap();
        fs::write(&gradle, "// Rullst application-owned release signing v2\n").unwrap();
        let key = root.join("private.jks");
        fs::write(&key, "not a real key").unwrap();
        let certificate = root.join("public.der");
        fs::write(&certificate, "not a real certificate").unwrap();
        let digest = hex::encode(Sha256::digest(fs::read(&certificate).unwrap()));
        let jar = root.join("apksigner.jar");
        fs::write(&jar, "protocol fixture").unwrap();
        let relative = "arm64/release/app-arm64-release.apk";
        let apk = root
            .join("omni-app/gen/android/app/build/outputs/apk")
            .join(relative);
        if mode == "stale" {
            fs::create_dir_all(apk.parent().unwrap()).unwrap();
            fs::write(&apk, "previous artifact").unwrap();
        }
        let mut command = Command::new(env!("CARGO_BIN_EXE_rullst"));
        command
            .current_dir(&root)
            .args([
                "omni",
                "android",
                "--release",
                "--android-arch",
                "aarch64",
                "--signing-certificate",
            ])
            .arg(certificate)
            .arg("--apksigner-jar")
            .arg(jar)
            .env("PATH", &fixture_path)
            .env("RULLST_DISABLE_UPDATE_CHECK", "1")
            .env("RULLST_ANDROID_KEYSTORE", key)
            .env("RULLST_ANDROID_KEY_ALIAS", "fixture")
            .env(
                "RULLST_ANDROID_STORE_PASSWORD",
                "never-print-fixture-password",
            )
            .env(
                "RULLST_ANDROID_KEY_PASSWORD",
                "never-print-fixture-password",
            )
            .env("JAVA_TOOL_OPTIONS", "must not reach the verifier")
            .env(
                "APK_FIXTURE_MODE",
                if mode == "selected" {
                    "ambiguous"
                } else {
                    mode
                },
            )
            .env("APK_FIXTURE_ROOT", &root)
            .env("APK_FIXTURE_CERT_DIGEST", &digest);
        if mode == "selected" {
            command.args(["--apk", relative]);
        }
        if mode == "escape" {
            command.args(["--apk", "../outside-release.apk"]);
        }
        let output = command.output().unwrap();
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !text.contains("never-print-fixture-password"),
            "{mode}: {text}"
        );
        assert_eq!(
            output.status.success(),
            ["valid", "selected"].contains(&mode),
            "{mode}: {text}"
        );
        if output.status.success() {
            let evidence: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(evidence["schema_version"], "rullst.android-release.v1");
            assert_eq!(evidence["certificate_sha256"], digest);
            assert_eq!(
                evidence["apk_sha256"],
                hex::encode(Sha256::digest(fs::read(&apk).unwrap()))
            );
            assert_eq!(evidence["bytes"], fs::metadata(&apk).unwrap().len());
        }
        if mode == "escape" {
            assert!(!root.join("build-started").exists());
        }
        if mode == "stale" {
            assert_eq!(fs::read_to_string(&apk).unwrap(), "previous artifact");
        }
        if let Ok(path) = fs::read_to_string(root.join("snapshot-path")) {
            assert!(
                !Path::new(&path).exists(),
                "private snapshot must be cleaned on success and failure"
            );
        }
    }
}
