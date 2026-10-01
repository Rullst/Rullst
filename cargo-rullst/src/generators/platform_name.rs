//! Platform-safe names derived from the Cargo package name.
//!
//! Cargo accepts `my_app` or `MyApp`, but Kubernetes object names, image
//! references, ingress hosts and Fly.io app names require lowercase RFC 1123
//! labels.

use std::fs;
use std::path::Path;

/// Leaves room for the `-service`/`-ingress` suffixes within a 63-byte label.
const MAX_BASE_LABEL: usize = 55;
const FALLBACK_LABEL: &str = "rullst-app";

/// Reads `[package].name` from a Cargo manifest.
pub(crate) fn package_name(manifest: &Path) -> Option<String> {
    let manifest = fs::read_to_string(manifest).ok()?;
    let manifest = toml::from_str::<toml::Value>(&manifest).ok()?;
    manifest
        .get("package")?
        .get("name")?
        .as_str()
        .map(str::to_string)
}

/// Maps `name` to a lowercase RFC 1123 label that starts with a letter.
///
/// Characters other than ASCII letters and digits become `-`, runs of `-`
/// collapse, and the result is trimmed and capped at 55 bytes.
pub(crate) fn dns_label(name: &str) -> String {
    let mut label = String::with_capacity(name.len());
    for character in name.chars().map(|character| character.to_ascii_lowercase()) {
        if character.is_ascii_lowercase() || character.is_ascii_digit() {
            label.push(character);
        } else if !label.is_empty() && !label.ends_with('-') {
            label.push('-');
        }
    }
    if label.starts_with(|character: char| character.is_ascii_digit()) {
        label.insert_str(0, "app-");
    }
    // Only ASCII remains, so truncating at a byte index is a char boundary.
    label.truncate(MAX_BASE_LABEL);
    let label = label.trim_end_matches('-');
    if label.is_empty() {
        FALLBACK_LABEL.to_string()
    } else {
        label.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_names_become_lowercase_rfc_1123_labels() {
        assert_eq!(dns_label("my_app"), "my-app");
        assert_eq!(dns_label("MyApp"), "myapp");
        assert_eq!(dns_label("rullst_app"), "rullst-app");
        assert_eq!(dns_label("already-valid"), "already-valid");
        assert_eq!(dns_label("__weird__name__"), "weird-name");
        assert_eq!(dns_label("9lives"), "app-9lives");
        assert_eq!(dns_label("___"), FALLBACK_LABEL);
        assert_eq!(dns_label("ünïcode"), "n-code");

        let long = dns_label(&format!("{}_tail", "a".repeat(80)));
        assert_eq!(long.len(), MAX_BASE_LABEL);
        assert!(format!("{long}-service").len() <= 63);
        let dash_at_cut = dns_label(&format!("{}_b", "a".repeat(MAX_BASE_LABEL - 1)));
        assert!(!dash_at_cut.ends_with('-'));
    }

    #[test]
    fn package_name_reads_only_the_package_table() {
        let directory = tempfile::tempdir().unwrap();
        let manifest = directory.path().join("Cargo.toml");
        fs::write(
            &manifest,
            "[workspace]\nmembers = []\n\n[[bin]]\nname = \"helper\"\npath = \"src/helper.rs\"\n\n[package]\nname = \"my_app\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        assert_eq!(package_name(&manifest).as_deref(), Some("my_app"));

        fs::write(&manifest, "[workspace]\nmembers = [\"app\"]\n").unwrap();
        assert_eq!(package_name(&manifest), None);
        assert_eq!(package_name(&directory.path().join("missing.toml")), None);
    }
}
