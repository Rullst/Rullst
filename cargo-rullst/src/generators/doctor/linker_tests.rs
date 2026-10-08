#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use crate::generators::doctor::render;
use crate::generators::doctor::report::{Group, Report, Status};
use crate::ui::style::Style;
use std::collections::HashMap;
use std::fs;

const TRIPLE: Option<&str> = Some("aarch64-unknown-linux-gnu");

fn facts(configured: bool, default_lld: bool, mold: bool, lld: bool) -> LinkerFacts {
    LinkerFacts {
        table: "target.aarch64-unknown-linux-gnu".to_string(),
        configured,
        default_lld,
        mold,
        lld,
    }
}

#[test]
fn a_config_selects_a_linker_through_fuse_ld_or_a_linker_entry() {
    for selecting in [
        "[target.aarch64-unknown-linux-gnu]\nrustflags = [\"-C\", \"link-arg=-fuse-ld=mold\"]\n",
        "[target.aarch64-unknown-linux-gnu]\nrustflags = \"-C link-arg=-fuse-ld=lld\"\n",
        "[target.aarch64-unknown-linux-gnu]\nlinker = \"clang\"\n",
        "[target.'cfg(target_os = \"linux\")']\nrustflags = [\"-Clink-arg=--ld-path=/usr/bin/mold\"]\n",
        "[build]\nrustflags = [\"-C\", \"link-arg=-fuse-ld=lld\"]\n",
    ] {
        assert!(config_selects_linker(selecting, TRIPLE), "{selecting}");
    }
    for other in [
        // The generated fallback only splits debug information.
        "[target.aarch64-unknown-linux-gnu]\nrustflags = [\"-C\", \"split-debuginfo=unpacked\"]\n",
        // Another host's table does not apply here.
        "[target.x86_64-unknown-linux-gnu]\nrustflags = [\"-C\", \"link-arg=-fuse-ld=mold\"]\n",
        "[target.aarch64-unknown-linux-gnu]\nrustflags = [\"-C\", \"link-arg=-fuse-ld=gold\"]\n",
        "# -fuse-ld=mold only in a comment\n",
        "not = [valid toml",
    ] {
        assert!(!config_selects_linker(other, TRIPLE), "{other}");
    }
}

#[test]
fn project_ancestor_and_cargo_home_configs_and_environment_are_detected() {
    let root = tempfile::tempdir().unwrap();
    let project = root.path().join("workspace/app");
    let home = root.path().join("cargo-home");
    fs::create_dir_all(project.join("src")).unwrap();
    fs::create_dir_all(&home).unwrap();
    let none: HashMap<String, String> = HashMap::new();
    let cwd = project.join("src");
    assert!(!configured(&cwd, Some(&home), TRIPLE, &none));

    let mold =
        "[target.aarch64-unknown-linux-gnu]\nrustflags = [\"-C\", \"link-arg=-fuse-ld=mold\"]\n";
    for config in [
        project.join(".cargo/config.toml"),
        root.path().join("workspace/.cargo/config"),
        home.join("config.toml"),
    ] {
        fs::create_dir_all(config.parent().unwrap()).unwrap();
        fs::write(&config, mold).unwrap();
        assert!(configured(&cwd, Some(&home), TRIPLE, &none), "{config:?}");
        fs::remove_file(&config).unwrap();
    }

    for (name, value) in [
        ("RUSTFLAGS", "-C link-arg=-fuse-ld=mold"),
        ("CARGO_ENCODED_RUSTFLAGS", "-C\u{1f}link-arg=-fuse-ld=lld"),
        ("CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER", "clang"),
    ] {
        let vars = HashMap::from([(name.to_string(), value.to_string())]);
        assert!(configured(&cwd, Some(&home), TRIPLE, &vars), "{name}");
    }
    let vars = HashMap::from([("RUSTFLAGS".to_string(), "-C opt-level=3".to_string())]);
    assert!(!configured(&cwd, Some(&home), TRIPLE, &vars));
}

#[test]
fn the_hint_is_a_warning_with_the_exact_snippet_and_skipped_when_not_needed() {
    assert_eq!(check(&facts(true, false, true, true)), None);
    assert_eq!(check(&facts(false, true, false, false)), None);

    let mold = check(&facts(false, false, true, true)).unwrap();
    assert_eq!(mold.id, "toolchain.linker");
    assert_eq!(mold.status, Status::Warn);
    assert_eq!(mold.detail, "not configured; mold is installed");
    assert_eq!(
        mold.fix.as_deref(),
        Some(
            "Add to .cargo/config.toml:\n[target.aarch64-unknown-linux-gnu]\nrustflags = [\"-C\", \"link-arg=-fuse-ld=mold\"]"
        )
    );
    let lld = check(&facts(false, false, false, true)).unwrap();
    assert_eq!(lld.detail, "not configured; ld.lld is installed");
    assert!(lld.fix.unwrap().ends_with("\"link-arg=-fuse-ld=lld\"]"));
    let neither = check(&facts(false, false, false, false)).unwrap();
    assert!(neither.detail.contains("neither mold nor ld.lld"));
    assert!(neither.fix.unwrap().starts_with("Install mold"));

    // A warning never fails the doctor; the snippet stays aligned and linked.
    let report = Report::new(vec![Group::new(
        "toolchain",
        "Toolchain",
        "toolchain",
        vec![mold],
    )]);
    assert!(report.ok);
    assert_eq!(report.summary.warn, 1);
    let check = report.check("toolchain.linker").unwrap();
    assert_eq!(
        check.docs.as_deref(),
        Some("https://rullst.github.io/Rullst/book/cli_reference.html#doctor-toolchain")
    );
    let text = render::render(&report, Style::PLAIN);
    assert!(
        text.contains("            [target.aarch64-unknown-linux-gnu]\n"),
        "{text}"
    );
    let json = serde_json::to_value(&report).unwrap();
    assert_eq!(json["groups"][0]["checks"][0]["status"], "warn");
}

#[cfg(target_os = "linux")]
#[test]
fn detection_reports_rust_lld_as_the_x86_64_gnu_default() {
    let directory = tempfile::tempdir().unwrap();
    let rustc = Probe {
        found: true,
        success: true,
        first_line: "rustc 1.98.1 (fixture)".to_string(),
        stdout: String::new(),
    };
    let vars = HashMap::from([(
        "CARGO_HOME".to_string(),
        directory.path().display().to_string(),
    )]);
    let facts = detect(directory.path(), &rustc, &vars).expect("Linux facts");
    let x86_64_gnu = cfg!(all(target_arch = "x86_64", target_env = "gnu"));
    assert_eq!(facts.default_lld, x86_64_gnu);
    let old = Probe {
        first_line: "rustc 1.89.0 (fixture)".to_string(),
        ..rustc
    };
    assert!(!detect(directory.path(), &old, &vars).unwrap().default_lld);
    let missing = detect(directory.path(), &Probe::default(), &vars).unwrap();
    assert_eq!(missing.default_lld, x86_64_gnu);
}
