//! Android release signing uses application-owned keys supplied by the caller.
use std::{fs, path::Path};

const GRADLE_SIGNING: &str = include_str!("signing.gradle.kts");
const SIGNING_MARKER: &str = "// Rullst application-owned release signing v2";
const VARIABLES: [&str; 4] = [
    "RULLST_ANDROID_KEYSTORE",
    "RULLST_ANDROID_KEY_ALIAS",
    "RULLST_ANDROID_STORE_PASSWORD",
    "RULLST_ANDROID_KEY_PASSWORD",
];

#[derive(thiserror::Error)]
enum SigningError {
    #[error(
        "Android release signing requires {0}; see the Omni signing tutorial (never put passwords in command arguments)"
    )]
    Missing(&'static str),
    #[error("RULLST_ANDROID_KEYSTORE must name an existing absolute keystore path")]
    Keystore,
    #[error(
        "the generated Android project has no Rullst signing guard; follow the migration steps in the Omni signing tutorial"
    )]
    Unconfigured,
    #[error("Android release build failed; no distributable APK is certified")]
    Build,
}

impl std::fmt::Debug for SigningError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

pub(super) fn configure_android_signing(omni_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let path = omni_dir.join("gen/android/app/build.gradle.kts");
    let mut source = fs::read_to_string(&path)?;
    if source.contains("// Rullst application-owned release signing v1") {
        return Err(SigningError::Unconfigured.into());
    }
    if !source.contains(SIGNING_MARKER) {
        source.push_str(GRADLE_SIGNING);
        fs::write(path, source)?;
    }
    Ok(())
}

pub fn build_android_release() -> Result<(), Box<dyn std::error::Error>> {
    for variable in VARIABLES {
        if std::env::var_os(variable).is_none_or(|value| value.is_empty()) {
            return Err(SigningError::Missing(variable).into());
        }
    }
    let key = std::env::var_os(VARIABLES[0]).ok_or(SigningError::Missing(VARIABLES[0]))?;
    if !Path::new(&key).is_absolute() || !Path::new(&key).is_file() {
        return Err(SigningError::Keystore.into());
    }
    let omni_dir = Path::new("omni-app");
    let gradle = fs::read_to_string(omni_dir.join("gen/android/app/build.gradle.kts"))?;
    if !gradle.contains(SIGNING_MARKER) {
        return Err(SigningError::Unconfigured.into());
    }
    let status = super::runner::get_tauri_command(omni_dir)?
        .args(["android", "build", "--apk", "--ci"])
        .current_dir(omni_dir)
        .status()?;
    if !status.success() {
        return Err(SigningError::Build.into());
    }
    println!(
        "Android release build completed with application-owned signing. Verify the APK certificate and test it on a device before distribution; store acceptance is separate."
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signing_hook_preserves_existing_gradle_and_is_idempotent() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("gen/android/app/build.gradle.kts");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "// application-owned settings\n").unwrap();
        configure_android_signing(temp.path()).unwrap();
        let first = fs::read_to_string(&path).unwrap();
        configure_android_signing(temp.path()).unwrap();
        assert_eq!(first, fs::read_to_string(path).unwrap());
        assert!(first.starts_with("// application-owned settings\n"));
        for variable in VARIABLES {
            assert!(first.contains(variable));
        }
        assert!(first.contains("it.name.endsWith(\"ReleaseBuild\")"));
        assert!(first.contains("signingConfig = signingConfigs.getByName(\"rullstRelease\")"));
        assert!(!first.contains("keystore.properties"));
        // Gradle's `java` extension shadows package-qualified java.io.File.
        // File is provided by Kotlin DSL's implicit java.io imports.
        assert!(!first.contains("java.io.File"));
    }

    #[test]
    fn old_application_owned_guard_requires_review_without_overwrite() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("gen/android/app/build.gradle.kts");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let original = "// Rullst application-owned release signing v1\n// custom settings";
        fs::write(&path, original).unwrap();
        assert!(configure_android_signing(temp.path()).is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), original);
    }
}
