use std::fs;
use std::path::Path;

/// Generates a CycloneDX 1.5 SBOM from the packages recorded in Cargo.lock.
pub fn generate_cyclonedx_sbom(
    lock_path: &Path,
) -> Result<(usize, String), Box<dyn std::error::Error>> {
    if !lock_path.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Cargo lockfile '{}' does not exist", lock_path.display()),
        )
        .into());
    }

    let (mut project_name, mut project_version) = ("rullst-app".to_string(), "0.1.0".to_string());
    if let Ok(cargo_toml) = fs::read_to_string("Cargo.toml") {
        for line in cargo_toml.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("name = ") {
                project_name = value.replace(['"', '\''], "").trim().to_string();
            } else if let Some(value) = trimmed.strip_prefix("version = ") {
                project_version = value.replace(['"', '\''], "").trim().to_string();
            }
        }
    }

    let mut components = Vec::new();
    let lock_content = fs::read_to_string(lock_path)?;
    let (mut name, mut version, mut checksum) = (String::new(), String::new(), String::new());
    for line in lock_content.lines() {
        let trimmed = line.trim();
        if trimmed == "[[package]]" {
            push_component(&mut components, &name, &version, &checksum);
            name.clear();
            version.clear();
            checksum.clear();
        } else if let Some(value) = trimmed.strip_prefix("name = ") {
            name = unquote(value);
        } else if let Some(value) = trimmed.strip_prefix("version = ") {
            version = unquote(value);
        } else if let Some(value) = trimmed.strip_prefix("checksum = ") {
            checksum = unquote(value);
        }
    }
    push_component(&mut components, &name, &version, &checksum);

    let count = components.len();
    let sbom = serde_json::json!({
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "serialNumber": format!("urn:uuid:{}", rand::random::<u128>()),
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
    fs::write("sbom-cyclonedx.json", serde_json::to_string_pretty(&sbom)?)?;
    Ok((count, "sbom-cyclonedx.json".to_string()))
}

fn unquote(value: &str) -> String {
    value.trim().trim_matches(['"', '\'']).to_string()
}

fn push_component(
    components: &mut Vec<serde_json::Value>,
    name: &str,
    version: &str,
    checksum: &str,
) {
    if name.is_empty() || version.is_empty() {
        return;
    }
    let mut component = serde_json::json!({
        "type": "library",
        "name": name,
        "version": version,
        "purl": format!("pkg:cargo/{name}@{version}"),
    });
    if !checksum.is_empty() {
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
            && content.contains("\"0.0.0.0:")
            && (content.contains("5555") || content.contains("studio"))
        {
            warnings.push(format!(
                "File '{}': Studio or an internal control room binds to 0.0.0.0 instead of loopback",
                path.display()
            ));
        }
    }
}
