//! SBOM components, bind-address warnings and the network evidence count.
use super::*;
use serde_json::json;

const CRATES_IO: &str = "registry+https://github.com/rust-lang/crates.io-index";

#[test]
fn components_need_a_name_and_version_and_keep_only_valid_checksums() {
    let mut components = Vec::new();
    push_component(&mut components, "named", "", "", None, 0);
    push_component(&mut components, "", "1.0.0", "", None, 1);
    assert!(components.is_empty());
    let valid = "0123456789abcdef".repeat(4);
    push_component(&mut components, "dep", "1.0.0", &valid, Some(CRATES_IO), 2);
    push_component(
        &mut components,
        "dep",
        "1.0.1",
        &"g".repeat(64),
        Some(CRATES_IO),
        3,
    );
    push_component(&mut components, "dep", "1.0.2", "abc", Some(CRATES_IO), 4);
    push_component(
        &mut components,
        "forked",
        "0.1.0",
        "",
        Some("git+https://example.com/forked.git?rev=abc#0123456789abcdef0123456789abcdef01234567"),
        5,
    );
    assert_eq!(
        components[0]["hashes"],
        json!([{"alg": "SHA-256", "content": valid}])
    );
    assert!(components[1].get("hashes").is_none());
    assert!(components[2].get("hashes").is_none());
    let reference = components[3]["bom-ref"].as_str().unwrap();
    assert!(
        reference.starts_with("pkg:cargo/forked@0.1.0?vcs_url="),
        "{reference}"
    );
    assert!(reference.ends_with("&rullst-index=5"), "{reference}");
}

const HOST_WARNING: &str = "Environment key 'HOST' uses an unspecified bind address; review whether it should be '127.0.0.1'";
const SOURCE_WARNING: &str = "src/main.rs': source contains an unspecified-address listener; review whether it should be '127.0.0.1'";

fn exposed_project() -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join(".env"),
        "HOST=0.0.0.0\n# BIND=0.0.0.0\nLOCAL=127.0.0.1\nV6=\"[::]\"\nNOT_AN_ASSIGNMENT\n",
    )
    .unwrap();
    fs::create_dir(directory.path().join("src")).unwrap();
    fs::write(
        directory.path().join("src/main.rs"),
        "fn main() { let _ = TcpListener::bind(\"0.0.0.0:5555\"); }\n",
    )
    .unwrap();
    directory
}

#[test]
fn unspecified_bind_addresses_in_env_files_and_sources_are_warnings() {
    let project = exposed_project();
    let mut warnings = Vec::new();
    inspect_environment_binding(&project.path().join(".env"), &mut warnings);
    assert_eq!(
        warnings,
        [
            HOST_WARNING,
            "Environment key 'V6' uses an unspecified bind address; review whether it should be '127.0.0.1'",
        ]
    );
    let (mut warnings, mut incomplete) = (Vec::new(), Vec::new());
    inspect_bindings(&project.path().join("src"), &mut warnings, &mut incomplete);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].ends_with(SOURCE_WARNING), "{warnings:?}");
    assert!(incomplete.is_empty());
}

/// Runs the working-directory evidence functions in a child test process.
#[test]
fn evidence_child_fixture() {
    let Some(report) = std::env::var_os("RULLST_EVIDENCE_REPORT") else {
        return;
    };
    let text = if std::env::var_os("RULLST_EVIDENCE_SBOM").is_some() {
        match generate_cyclonedx_sbom(Path::new("Cargo.lock")) {
            Ok((count, path)) => format!("{count} {path}"),
            Err(error) => format!("err {error}"),
        }
    } else {
        let (findings, observations) = scan_local_network_surface();
        format!("{findings}\n{}", observations.join("\n"))
    };
    fs::write(report, text).unwrap();
}

fn evidence_in(project: &Path, sbom: bool) -> String {
    let report = project.join("report");
    let programs = project.join("no-programs");
    fs::create_dir_all(&programs).unwrap();
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "generators::audit_evidence::evidence_tests::evidence_child_fixture",
            "--nocapture",
        ])
        .current_dir(project)
        // Without `ss`, the listener inventory is reported as incomplete.
        .env("PATH", &programs)
        .env("RULLST_EVIDENCE_REPORT", &report)
        .env_remove("RULLST_EVIDENCE_SBOM")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null());
    if sbom {
        command.env("RULLST_EVIDENCE_SBOM", "1");
    }
    assert!(command.status().unwrap().success());
    fs::read_to_string(report).unwrap()
}

#[cfg(unix)]
#[test]
fn network_findings_count_warnings_and_incomplete_inventories() {
    let project = exposed_project();
    let text = evidence_in(project.path(), false);
    let mut lines = text.lines();
    assert_eq!(lines.next(), Some("4"), "{text}");
    let observations: Vec<_> = lines.collect();
    let tail = &observations[observations.len() - 4..];
    assert!(tail[0].ends_with(SOURCE_WARNING), "{text}");
    assert_eq!(tail[1], HOST_WARNING);
    assert!(tail[2].starts_with("Environment key 'V6'"), "{text}");
    assert!(
        tail[3].starts_with("the TCP listener inventory `ss -ltnH` could not run"),
        "{text}"
    );
}

#[test]
fn the_sbom_is_written_next_to_the_project_manifest() {
    let project = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"1.2.3\"\n",
    )
    .unwrap();
    fs::write(
        project.path().join("Cargo.lock"),
        format!("version = 4\n\n[[package]]\nname = \"demo\"\nversion = \"1.2.3\"\n\n[[package]]\nname = \"dep\"\nversion = \"2.0.0\"\nsource = \"{CRATES_IO}\"\n"),
    )
    .unwrap();
    assert_eq!(evidence_in(project.path(), true), "2 sbom-cyclonedx.json");
    let document: serde_json::Value =
        serde_json::from_slice(&fs::read(project.path().join("sbom-cyclonedx.json")).unwrap())
            .unwrap();
    assert_eq!(document["metadata"]["component"]["name"], "demo");
}
