use super::OmniScaffoldOptions;
use semver::Version;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OmniIdentity {
    pub(super) product_name: String,
    pub(super) identifier: String,
    pub(super) version: String,
    pub(super) uses_placeholder_identifier: bool,
}

pub(super) fn resolve_identity(
    root: &Path,
    options: &OmniScaffoldOptions,
    mobile_selected: bool,
) -> Result<OmniIdentity, Box<dyn std::error::Error>> {
    let manifest = read_manifest(root)?;
    let package = manifest
        .get("package")
        .and_then(toml::Value::as_table)
        .ok_or("Cargo.toml must contain a [package] table for Omni scaffolding")?;
    let package_name = package
        .get("name")
        .and_then(toml::Value::as_str)
        .ok_or("Cargo.toml [package].name must be a string")?;

    let product_name = options
        .product_name
        .clone()
        .unwrap_or_else(|| humanize_package_name(package_name));
    validate_product_name(&product_name)?;

    let version = match options.app_version.as_deref() {
        Some(version) => version.to_string(),
        None => package_version(package, &manifest)?.to_string(),
    };
    Version::parse(&version)
        .map_err(|error| format!("Omni app version `{version}` is not valid SemVer: {error}"))?;

    let (identifier, uses_placeholder_identifier) = match options.identifier.as_deref() {
        Some(identifier) if mobile_selected && identifier.starts_with("com.example.") => {
            return Err(
                "mobile Omni identifiers may not use the reserved `com.example` placeholder namespace"
                    .into(),
            );
        }
        Some("com.rullst.omni") => {
            return Err(
                "`com.rullst.omni` is a reserved framework placeholder; use an application-owned identifier"
                    .into(),
            );
        }
        Some(identifier) => (identifier.to_string(), false),
        None if mobile_selected => {
            return Err(
                "--identifier is required for Android/iOS and must be an application-owned reverse-DNS identifier"
                    .into(),
            );
        }
        None => (
            format!("com.example.{}", identifier_segment(package_name)),
            true,
        ),
    };
    validate_identifier(&identifier)?;

    Ok(OmniIdentity {
        product_name,
        identifier,
        version,
        uses_placeholder_identifier,
    })
}

fn read_manifest(root: &Path) -> Result<toml::Value, Box<dyn std::error::Error>> {
    let manifest_path = root.join("Cargo.toml");
    let content = fs::read_to_string(&manifest_path)?;
    toml::from_str(&content)
        .map_err(|error| format!("{} is not valid TOML: {error}", manifest_path.display()).into())
}

fn package_version<'a>(
    package: &'a toml::value::Table,
    manifest: &'a toml::Value,
) -> Result<&'a str, Box<dyn std::error::Error>> {
    if let Some(version) = package.get("version").and_then(toml::Value::as_str) {
        return Ok(version);
    }
    let inherits_workspace_version = package
        .get("version")
        .and_then(toml::Value::as_table)
        .and_then(|version| version.get("workspace"))
        .and_then(toml::Value::as_bool)
        == Some(true);
    if inherits_workspace_version {
        return manifest
            .get("workspace")
            .and_then(|workspace| workspace.get("package"))
            .and_then(|package| package.get("version"))
            .and_then(toml::Value::as_str)
            .ok_or_else(|| {
                "Cargo.toml inherits package.version but [workspace.package].version is missing"
                    .into()
            });
    }
    Err("Cargo.toml [package].version must be a SemVer string".into())
}

fn humanize_package_name(package_name: &str) -> String {
    package_name
        .split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut characters = part.chars();
            match characters.next() {
                Some(first) => first.to_uppercase().chain(characters).collect(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn identifier_segment(package_name: &str) -> String {
    let mut segment = package_name
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .map(|character| character.to_ascii_lowercase())
        .collect::<String>();
    if !segment.starts_with(|character: char| character.is_ascii_lowercase()) {
        segment.insert_str(0, "app");
    }
    segment
}

fn validate_product_name(value: &str) -> Result<(), Box<dyn std::error::Error>> {
    let length = value.chars().count();
    let has_invalid_character = value.chars().any(|character| {
        !(character.is_alphanumeric() || character == ' ' || matches!(character, '-' | '_' | '.'))
    });
    if !(1..=64).contains(&length)
        || value.trim() != value
        || value.starts_with('.')
        || has_invalid_character
    {
        return Err(
            "Omni product name must contain 1-64 letters, numbers, spaces, dots, hyphens or underscores and may not start with a dot"
                .into(),
        );
    }
    Ok(())
}

fn validate_identifier(value: &str) -> Result<(), Box<dyn std::error::Error>> {
    let segments = value.split('.').collect::<Vec<_>>();
    let valid = value.len() <= 155
        && segments.len() >= 2
        && segments.iter().all(|segment| {
            let mut characters = segment.chars();
            characters
                .next()
                .is_some_and(|first| first.is_ascii_lowercase())
                && characters
                    .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
        });
    if !valid {
        return Err(
            "Omni identifier must be a lowercase reverse-DNS value such as `com.example.myapp`; each segment must start with a letter and contain only letters or digits"
                .into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn validates_cross_platform_identity_boundaries() {
        assert!(validate_product_name("Acme Chat").is_ok());
        assert!(validate_product_name("../Acme").is_err());
        assert!(validate_product_name("Acme/Chat").is_err());
        assert!(validate_product_name("Acme\nChat").is_err());
        assert!(validate_identifier("com.acme.chat").is_ok());
        assert!(validate_identifier("com.acme.chatapp2").is_ok());
        assert!(validate_identifier("com.acme.chat_app2").is_err());
        assert!(validate_identifier("Com.Acme.Chat").is_err());
        assert!(validate_identifier("com.acme.chat-app").is_err());
        assert!(validate_identifier("single").is_err());
    }

    #[test]
    fn derives_safe_human_and_identifier_names() {
        assert_eq!(humanize_package_name("acme-chat"), "Acme Chat");
        assert_eq!(identifier_segment("42-chat"), "app42chat");
    }

    #[test]
    fn resolves_manifest_defaults_and_requires_owned_mobile_identifier() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "rullst-omni-identity-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("temporary identity directory");
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"acme-chat\"\nversion = \"2.3.4\"\n",
        )
        .expect("temporary identity manifest");

        let desktop = resolve_identity(&root, &OmniScaffoldOptions::new(["desktop"]), false)
            .expect("derived desktop identity");
        assert_eq!(desktop.product_name, "Acme Chat");
        assert_eq!(desktop.version, "2.3.4");
        assert_eq!(desktop.identifier, "com.example.acmechat");
        assert!(desktop.uses_placeholder_identifier);

        assert!(resolve_identity(&root, &OmniScaffoldOptions::new(["ios"]), true).is_err());
        assert!(
            resolve_identity(
                &root,
                &OmniScaffoldOptions::new(["android"]).identifier("com.example.acme"),
                true,
            )
            .is_err()
        );
        assert!(
            resolve_identity(
                &root,
                &OmniScaffoldOptions::new(["android"]).identifier("com.rullst.omni"),
                true,
            )
            .is_err()
        );
        let mobile = resolve_identity(
            &root,
            &OmniScaffoldOptions::new(["android"]).identifier("com.acme.chat"),
            true,
        )
        .expect("application-owned mobile identity");
        assert!(!mobile.uses_placeholder_identifier);

        let _ = fs::remove_dir_all(root);
    }
}
