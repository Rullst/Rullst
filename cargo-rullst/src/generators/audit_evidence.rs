use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The lockfile `cargo audit` must read for `root`: its own `Cargo.lock`, or
/// for a workspace member the one in the workspace root that `cargo metadata`
/// (through `program`) reports. `None` leaves the choice to `cargo audit`.
pub(crate) fn audit_lockfile(program: &OsStr, root: &Path) -> Option<PathBuf> {
    #[derive(serde::Deserialize)]
    struct Workspace {
        workspace_root: PathBuf,
    }

    let local = root.join("Cargo.lock");
    if local.is_file() {
        return Some(local);
    }
    let output = Command::new(program)
        .args([
            "metadata",
            "--no-deps",
            "--offline",
            "--format-version",
            "1",
        ])
        .current_dir(root)
        .output()
        .ok()
        .filter(|output| output.status.success())?;
    let workspace = serde_json::from_slice::<Workspace>(&output.stdout).ok()?;
    Some(workspace.workspace_root.join("Cargo.lock")).filter(|lockfile| lockfile.is_file())
}

/// Runs `cargo audit` (through `program`) in `root` with `arguments`, on the
/// lockfile [`audit_lockfile`] finds.
pub(crate) fn run_cargo_audit(
    program: &OsStr,
    root: &Path,
    arguments: &[String],
) -> std::io::Result<Output> {
    let mut audit = Command::new(program);
    audit.args(arguments).current_dir(root);
    // A workspace member has no `Cargo.lock` of its own.
    if let Some(lockfile) = audit_lockfile(program, root) {
        audit.arg("--file").arg(lockfile);
    }
    audit.output()
}

/// Generates a CycloneDX 1.5 SBOM from the packages recorded in Cargo.lock.
pub fn generate_cyclonedx_sbom(
    lock_path: &Path,
) -> Result<(usize, String), Box<dyn std::error::Error>> {
    let output = Path::new("sbom-cyclonedx.json");
    let count = generate_cyclonedx_sbom_at(lock_path, Path::new("Cargo.toml"), output)?;
    Ok((count, output.display().to_string()))
}

fn generate_cyclonedx_sbom_at(
    lock_path: &Path,
    manifest_path: &Path,
    output_path: &Path,
) -> Result<usize, Box<dyn std::error::Error>> {
    if !lock_path.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Cargo lockfile '{}' does not exist", lock_path.display()),
        )
        .into());
    }

    let (mut project_name, mut project_version) = ("rullst-app".to_string(), "0.1.0".to_string());
    if let Ok(cargo_toml) = fs::read_to_string(manifest_path)
        && let Ok(manifest) = toml::from_str::<toml::Value>(&cargo_toml)
        && let Some(package) = manifest.get("package")
    {
        if let Some(name) = package.get("name").and_then(toml::Value::as_str) {
            project_name = name.to_string();
        }
        if let Some(version) = package.get("version").and_then(toml::Value::as_str) {
            project_version = version.to_string();
        }
    }

    let mut components = Vec::new();
    let lock_content = fs::read_to_string(lock_path)?;
    let lockfile = toml::from_str::<toml::Value>(&lock_content)?;
    let packages = lockfile
        .get("package")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| std::io::Error::other("Cargo.lock does not contain a package array"))?;
    for (index, package) in packages.iter().enumerate() {
        let Some(name) = package.get("name").and_then(toml::Value::as_str) else {
            continue;
        };
        let Some(version) = package.get("version").and_then(toml::Value::as_str) else {
            continue;
        };
        let checksum = package
            .get("checksum")
            .and_then(toml::Value::as_str)
            .unwrap_or_default();
        push_component(&mut components, name, version, checksum, index);
    }

    let count = components.len();
    let sbom = serde_json::json!({
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "serialNumber": format!("urn:uuid:{}", uuid::Uuid::new_v4()),
        "version": 1,
        "metadata": {
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "tools": [{
                "vendor": "Rullst Core Team",
                "name": "cargo-rullst",
                "version": env!("CARGO_PKG_VERSION")
            }],
            "component": {
                "type": "application",
                "name": project_name,
                "version": project_version
            }
        },
        "components": components
    });
    fs::write(output_path, serde_json::to_string_pretty(&sbom)?)?;
    Ok(count)
}

fn push_component(
    components: &mut Vec<serde_json::Value>,
    name: &str,
    version: &str,
    checksum: &str,
    index: usize,
) {
    if name.is_empty() || version.is_empty() {
        return;
    }
    let mut component = serde_json::json!({
        "type": "library",
        "name": name,
        "version": version,
        "bom-ref": format!("pkg:cargo/{name}@{version}?rullst-index={index}"),
        "purl": format!("pkg:cargo/{name}@{version}"),
    });
    if checksum.len() == 64 && checksum.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        component["hashes"] = serde_json::json!([{
            "alg": "SHA-256",
            "content": checksum
        }]);
    }
    components.push(component);
}

/// Records loopback listeners and source bindings that expose Studio publicly.
pub fn scan_local_network_surface() -> (usize, Vec<String>) {
    use std::net::{SocketAddr, TcpStream};
    use std::time::Duration;

    let ports = [
        (3000, "Rullst Web Server / SSR"),
        (5555, "Rullst Studio Control Room"),
        (8000, "REST API Backend"),
        (8080, "Alternative Web Service"),
        (5432, "PostgreSQL Database"),
        (3306, "MySQL Database"),
        (6379, "Redis Cache / Queue"),
        (1883, "MQTT IoT Broker"),
        (9092, "Kafka Message Stream"),
    ];
    let mut observations = Vec::new();
    for (port, description) in ports {
        let address = SocketAddr::from(([127, 0, 0, 1], port));
        if TcpStream::connect_timeout(&address, Duration::from_millis(60)).is_ok() {
            observations.push(format!("Port {port} ({description}): OPEN on 127.0.0.1"));
        }
    }

    let mut warnings = Vec::new();
    inspect_bindings(Path::new("src"), &mut warnings);
    inspect_environment_binding(Path::new(".env"), &mut warnings);
    inspect_system_listeners(&mut warnings);
    let finding_count = warnings.len();
    observations.extend(warnings);
    (finding_count, observations)
}

fn inspect_bindings(directory: &Path, warnings: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            inspect_bindings(&path, warnings);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs")
            && let Ok(content) = fs::read_to_string(&path)
            && contains_unspecified_binding(&content)
        {
            warnings.push(format!(
                "File '{}': source contains an unspecified-address listener; review whether it should be '127.0.0.1'",
                path.display()
            ));
        }
    }
}

fn contains_unspecified_binding(content: &str) -> bool {
    let production = super::audit_source::production_source(content);
    [
        "\"0.0.0.0:",
        "\"[::]:",
        "([0, 0, 0, 0],",
        "Ipv4Addr::UNSPECIFIED",
        "Ipv6Addr::UNSPECIFIED",
    ]
    .iter()
    .any(|pattern| production.contains(pattern))
}

fn inspect_environment_binding(path: &Path, warnings: &mut Vec<String>) {
    let Ok(contents) = fs::read_to_string(path) else {
        return;
    };
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        let normalized = value.trim().trim_matches(['"', '\'']);
        if matches!(normalized, "0.0.0.0" | "::" | "[::]") {
            warnings.push(format!(
                "Environment key '{}' uses an unspecified bind address; review whether it should be '127.0.0.1'",
                key.trim()
            ));
        }
    }
}

fn inspect_system_listeners(warnings: &mut Vec<String>) {
    let Ok(output) = std::process::Command::new("ss").args(["-ltnH"]).output() else {
        return;
    };
    if !output.status.success() {
        return;
    }
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let Some(address) = line.split_whitespace().nth(3) else {
            continue;
        };
        if address.starts_with("0.0.0.0:")
            || address.starts_with("[::]:")
            || address.starts_with("*:")
        {
            warnings.push(format!(
                "Active TCP listener '{address}' accepts non-loopback traffic; review whether it should be '127.0.0.1'"
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent directory");
        }
        fs::write(path, contents).expect("fixture file");
    }

    #[test]
    fn a_workspace_member_audits_the_workspace_lockfile() {
        let project = tempfile::tempdir().expect("temporary project");
        let root = project.path().canonicalize().expect("canonical root");
        write(
            &root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"blog\"]\nresolver = \"3\"\n",
        );
        write(
            &root.join("blog/Cargo.toml"),
            "[package]\nname = \"blog\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        );
        write(&root.join("blog/src/main.rs"), "fn main() {}\n");
        write(&root.join("Cargo.lock"), "version = 4\n");

        let cargo = OsStr::new("cargo");
        // The member has no `Cargo.lock`; `cargo audit` there used to fail.
        assert_eq!(
            audit_lockfile(cargo, &root.join("blog")),
            Some(root.join("Cargo.lock"))
        );
        assert_eq!(audit_lockfile(cargo, &root), Some(root.join("Cargo.lock")));
        let outside = tempfile::tempdir().expect("temporary directory");
        assert_eq!(audit_lockfile(cargo, outside.path()), None);
    }

    #[cfg(unix)]
    #[test]
    fn cargo_audit_receives_the_workspace_lockfile() {
        use std::os::unix::fs::PermissionsExt as _;
        let project = tempfile::tempdir().expect("temporary project");
        let root = project.path();
        write(&root.join("Cargo.lock"), "version = 4\n");
        let member = root.join("blog");
        fs::create_dir_all(&member).expect("member directory");
        let lockfile = root.join("Cargo.lock");
        // Succeeds only when `--file <workspace>/Cargo.lock` is passed.
        let script = format!(
            "#!/bin/sh\nif [ \"$1\" = metadata ]; then printf '{{\"workspace_root\":\"%s\"}}' '{root}'; exit 0; fi\nprevious=\nfor argument in \"$@\"; do\n  if [ \"$previous\" = --file ] && [ \"$argument\" = '{lock}' ]; then exit 0; fi\n  previous=$argument\ndone\necho 'error: Cargo.lock not found' >&2\nexit 1\n",
            root = root.display(),
            lock = lockfile.display(),
        );
        let cargo = project.path().join("fake-cargo");
        fs::write(&cargo, script).expect("fake cargo");
        fs::set_permissions(&cargo, fs::Permissions::from_mode(0o755)).expect("executable");

        let output = run_cargo_audit(cargo.as_os_str(), &member, &["audit".to_string()])
            .expect("fake cargo runs");
        assert!(output.status.success());
    }

    #[test]
    fn generated_sbom_has_parseable_cyclonedx_identity_and_components() {
        let directory =
            std::env::temp_dir().join(format!("rullst-sbom-evidence-{}", rand::random::<u64>()));
        fs::create_dir_all(&directory).expect("temporary SBOM directory");
        let manifest = directory.join("Cargo.toml");
        let lock = directory.join("Cargo.lock");
        let output = directory.join("sbom.json");
        fs::write(
            &manifest,
            "[package]\nname = \"demo\"\nversion = \"1.2.3\"\n",
        )
        .expect("temporary manifest");
        fs::write(
            &lock,
            "version = 4\n\n[[package]]\nname = \"demo\"\nversion = \"1.2.3\"\n\n[[package]]\nname = \"dep\"\nversion = \"2.0.0\"\nchecksum = \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"\n",
        )
        .expect("temporary lockfile");

        assert_eq!(
            generate_cyclonedx_sbom_at(&lock, &manifest, &output).expect("SBOM generation"),
            2
        );
        let document: serde_json::Value =
            serde_json::from_slice(&fs::read(&output).expect("generated CycloneDX document"))
                .expect("valid JSON");
        assert_eq!(document["bomFormat"], "CycloneDX");
        assert_eq!(document["specVersion"], "1.5");
        assert_eq!(document["metadata"]["component"]["name"], "demo");
        assert_eq!(
            document["components"].as_array().expect("components").len(),
            2
        );
        let serial = document["serialNumber"]
            .as_str()
            .expect("serial number")
            .trim_start_matches("urn:uuid:");
        uuid::Uuid::parse_str(serial).expect("valid UUID serial number");
        fs::remove_dir_all(directory).expect("temporary SBOM cleanup");
    }

    #[test]
    fn network_source_heuristic_covers_ipv4_ipv6_and_ignores_test_tail() {
        assert!(contains_unspecified_binding(
            "TcpListener::bind(\"0.0.0.0:5555\")"
        ));
        assert!(contains_unspecified_binding(
            "TcpListener::bind(\"[::]:3000\")"
        ));
        assert!(contains_unspecified_binding("Ipv4Addr::UNSPECIFIED"));
        assert!(!contains_unspecified_binding(
            "TcpListener::bind(\"127.0.0.1:5555\")\n#[cfg(test)]\nfn test() { let _ = \"0.0.0.0:1\"; }"
        ));
        // Only the test item is skipped; a later listener is still found.
        assert!(contains_unspecified_binding(
            "#[cfg(test)]\nmod tests;\nfn serve() { TcpListener::bind(\"0.0.0.0:5555\"); }"
        ));
    }
}
