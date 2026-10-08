use std::fs;
use std::path::Path;

use crate::generators::audit_purl::{CargoOrigin, cargo_origin};
use crate::generators::output_guard::write_output;
use crate::generators::source_walk::rust_sources;

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
        let source = package.get("source").and_then(toml::Value::as_str);
        push_component(&mut components, name, version, checksum, source, index);
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
    // Replace a previous SBOM, but never follow a symlink committed in the
    // audited checkout.
    write_output(
        output_path,
        serde_json::to_string_pretty(&sbom)?.as_bytes(),
        true,
    )?;
    Ok(count)
}

fn push_component(
    components: &mut Vec<serde_json::Value>,
    name: &str,
    version: &str,
    checksum: &str,
    source: Option<&str>,
    index: usize,
) {
    if name.is_empty() || version.is_empty() {
        return;
    }
    let origin = cargo_origin(name, version, source);
    let bom_ref = match origin.purl() {
        Some(purl) if purl.contains('?') => format!("{purl}&rullst-index={index}"),
        Some(purl) => format!("{purl}?rullst-index={index}"),
        None => format!("local:cargo/{name}@{version}?rullst-index={index}"),
    };
    let mut component = serde_json::json!({
        "type": "library",
        "name": name,
        "version": version,
        "bom-ref": bom_ref,
    });
    if let Some(purl) = origin.purl() {
        component["purl"] = serde_json::json!(purl);
    }
    if !matches!(origin, CargoOrigin::CratesIo(_)) {
        // Path, workspace, git and other-registry packages are not the crates.io
        // package of the same name; record the lockfile origin explicitly.
        component["properties"] = serde_json::json!([{
            "name": "rullst:cargo:source",
            "value": source.unwrap_or("local")
        }]);
    }
    if checksum.len() == 64 && checksum.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        component["hashes"] = serde_json::json!([{
            "alg": "SHA-256",
            "content": checksum
        }]);
    }
    components.push(component);
}

/// Bounded local network evidence from one `audit --network` run.
pub(crate) struct NetworkSurface {
    /// Bindings or listeners that accept non-loopback traffic.
    pub(crate) findings: usize,
    /// Open loopback ports followed by the finding descriptions.
    pub(crate) observations: Vec<String>,
    /// Parts of the inventory that could not run; the check is then incomplete.
    pub(crate) incomplete: Vec<String>,
}

/// Records loopback listeners and source bindings that expose Studio publicly.
///
/// A part of the inventory that could not run (for example a missing `ss`) is
/// counted and described as a finding, so callers fail closed.
pub fn scan_local_network_surface() -> (usize, Vec<String>) {
    let surface = inspect_local_network_surface();
    let findings = surface.findings + surface.incomplete.len();
    let mut observations = surface.observations;
    observations.extend(surface.incomplete);
    (findings, observations)
}

pub(crate) fn inspect_local_network_surface() -> NetworkSurface {
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
    let mut incomplete = Vec::new();
    inspect_bindings(Path::new("src"), &mut warnings, &mut incomplete);
    inspect_environment_binding(Path::new(".env"), &mut warnings);
    if let Err(reason) = inspect_system_listeners(LISTENER_PROGRAM, &mut warnings) {
        incomplete.push(reason);
    }
    let findings = warnings.len();
    observations.extend(warnings);
    NetworkSurface {
        findings,
        observations,
        incomplete,
    }
}

fn inspect_bindings(directory: &Path, warnings: &mut Vec<String>, incomplete: &mut Vec<String>) {
    let sources = rust_sources(directory);
    if let Some(reason) = sources.incomplete {
        incomplete.push(format!(
            "the source walk under '{}' is incomplete ({reason}); listener bindings beyond it were not scanned",
            directory.display()
        ));
    }
    for path in sources.files {
        if let Ok(content) = fs::read_to_string(&path)
            && contains_unspecified_binding(&content)
        {
            warnings.push(format!(
                "File '{}': source contains an unspecified-address listener; review whether it should be '127.0.0.1'",
                super::slash_path(&path)
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

/// Lists TCP listeners with iproute2's `ss`, available on Linux.
const LISTENER_PROGRAM: &str = "ss";

/// Adds a warning per listener bound to an unspecified address.
///
/// Returns why the inventory did not run, so a requested check that could not
/// execute (macOS, Windows, or a Linux image without iproute2) is reported as
/// incomplete instead of clean.
fn inspect_system_listeners(program: &str, warnings: &mut Vec<String>) -> Result<(), String> {
    let output = std::process::Command::new(program)
        .args(["-ltnH"])
        .output()
        .map_err(|error| {
            format!(
                "the TCP listener inventory `{program} -ltnH` could not run ({error}); it requires iproute2's `ss`, which macOS and Windows do not provide"
            )
        })?;
    if !output.status.success() {
        return Err(format!(
            "the TCP listener inventory `{program} -ltnH` failed ({})",
            output.status
        ));
    }
    warnings.extend(listener_warnings(&String::from_utf8_lossy(&output.stdout)));
    Ok(())
}

fn listener_warnings(inventory: &str) -> Vec<String> {
    inventory
        .lines()
        .filter_map(|line| line.split_whitespace().nth(3))
        .filter(|address| {
            address.starts_with("0.0.0.0:") || address.starts_with("[::]:") || address.starts_with("*:")
        })
        .map(|address| {
            format!(
                "Active TCP listener '{address}' accepts non-loopback traffic; review whether it should be '127.0.0.1'"
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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
            "version = 4\n\n[[package]]\nname = \"demo\"\nversion = \"1.2.3\"\n\n[[package]]\nname = \"dep\"\nversion = \"2.0.0\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\nchecksum = \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"\n",
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
        let components = document["components"].as_array().expect("components");
        assert_eq!(components.len(), 2);
        // The root package has no lockfile source: it is not a crates.io crate.
        assert!(components[0].get("purl").is_none());
        assert_eq!(components[0]["properties"][0]["value"], "local");
        assert_eq!(components[1]["purl"], "pkg:cargo/dep@2.0.0");
        assert_eq!(
            components[1]["bom-ref"],
            "pkg:cargo/dep@2.0.0?rullst-index=1"
        );
        assert!(components[1].get("properties").is_none());
        let serial = document["serialNumber"]
            .as_str()
            .expect("serial number")
            .trim_start_matches("urn:uuid:");
        uuid::Uuid::parse_str(serial).expect("valid UUID serial number");
        fs::remove_dir_all(directory).expect("temporary SBOM cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn sbom_is_never_written_through_a_committed_symlink() {
        let directory = tempfile::tempdir().expect("temporary checkout");
        let lock = directory.path().join("Cargo.lock");
        fs::write(
            &lock,
            "version = 4\n\n[[package]]\nname = \"demo\"\nversion = \"1.0.0\"\n",
        )
        .expect("temporary lockfile");
        let victim = directory.path().join("bashrc");
        fs::write(&victim, "export PATH").expect("victim file");
        let output = directory.path().join("sbom-cyclonedx.json");
        std::os::unix::fs::symlink(&victim, &output).expect("committed symlink");

        assert!(
            generate_cyclonedx_sbom_at(&lock, &directory.path().join("Cargo.toml"), &output)
                .is_err()
        );
        assert_eq!(
            fs::read_to_string(&victim).expect("victim contents"),
            "export PATH"
        );

        fs::remove_file(&output).expect("remove symlink");
        fs::write(&output, "previous SBOM").expect("previous regular SBOM");
        generate_cyclonedx_sbom_at(&lock, &directory.path().join("Cargo.toml"), &output)
            .expect("a previous regular SBOM is replaced");
        assert!(
            fs::read_to_string(&output)
                .expect("new SBOM")
                .contains("CycloneDX")
        );
    }

    #[test]
    fn unavailable_listener_inventory_is_reported_instead_of_clean() {
        let mut warnings = Vec::new();
        let reason = inspect_system_listeners("rullst-missing-listener-inventory", &mut warnings)
            .expect_err("a missing listener tool must not pass as an empty inventory");
        assert!(reason.contains("could not run"), "{reason}");
        assert!(warnings.is_empty());

        #[cfg(unix)]
        {
            let failed = inspect_system_listeners("false", &mut warnings)
                .expect_err("a failing listener tool must not pass");
            assert!(failed.contains("failed"), "{failed}");
        }
    }

    #[test]
    fn listener_inventory_flags_only_unspecified_addresses() {
        let warnings = listener_warnings(
            "LISTEN 0 4096 127.0.0.1:5432 0.0.0.0:*\n\
             LISTEN 0 4096 0.0.0.0:5555 0.0.0.0:*\n\
             LISTEN 0 4096 [::]:3000 [::]:*\n\
             LISTEN 0 4096 *:8080 *:*\n\
             short line\n",
        );
        assert_eq!(warnings.len(), 3);
        assert!(warnings[0].contains("0.0.0.0:5555"));
        assert!(warnings[1].contains("[::]:3000"));
        assert!(warnings[2].contains("*:8080"));
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
