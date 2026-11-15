// cargo-rullst/src/generators/auth/sessions.rs — The login-session registry
// feature of `cargo rullst auth`.

use std::io::{Error as IoError, ErrorKind};

/// The `rullst` feature of the session registry over the project's strict
/// database profile (the one `cargo rullst new --database ...` selects).
pub(crate) fn registry_feature(manifest: &str) -> Result<&'static str, IoError> {
    let invalid = |message: &str| IoError::new(ErrorKind::InvalidInput, message.to_owned());
    let manifest: toml::Table =
        toml::from_str(manifest).map_err(|_| invalid("Cargo.toml is not valid TOML"))?;
    let dependencies = manifest.get("dependencies").and_then(toml::Value::as_table);
    let mut selected = Vec::new();
    for name in ["rullst", "rullst-orm"] {
        let features = dependencies
            .and_then(|dependencies| dependencies.get(name))
            .and_then(toml::Value::as_table)
            .and_then(|dependency| dependency.get("features"))
            .and_then(toml::Value::as_array);
        for feature in features
            .into_iter()
            .flatten()
            .filter_map(toml::Value::as_str)
        {
            let registry = match feature {
                "strict-sqlite" => "auth-session-registry-sqlite",
                "strict-postgres" => "auth-session-registry-postgres",
                "strict-mysql" => "auth-session-registry-mysql",
                _ => continue,
            };
            if !selected.contains(&registry) {
                selected.push(registry);
            }
        }
    }
    match selected.as_slice() {
        [registry] => Ok(registry),
        _ => Err(invalid(
            "cargo rullst auth keeps login sessions in the application database and needs exactly one strict database profile (strict-sqlite, strict-postgres or strict-mysql), as `cargo rullst new --database ...` selects",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(rullst: &str, orm: &str) -> String {
        format!(
            "[package]\nname = \"demo\"\n\n[dependencies]\nrullst = {{ version = \"13\", features = [{rullst}] }}\nrullst-orm = {{ version = \"13\", features = [{orm}] }}\n"
        )
    }

    #[test]
    fn the_registry_follows_the_strict_profile() {
        for (rullst, orm, feature) in [
            (
                "\"orm\", \"strict-sqlite\"",
                "\"strict-sqlite\"",
                "auth-session-registry-sqlite",
            ),
            (
                "\"orm\"",
                "\"strict-postgres\"",
                "auth-session-registry-postgres",
            ),
            ("\"strict-mysql\"", "", "auth-session-registry-mysql"),
        ] {
            assert_eq!(registry_feature(&manifest(rullst, orm)).unwrap(), feature);
        }
    }

    #[test]
    fn no_or_several_strict_profiles_are_refused() {
        for (rullst, orm) in [
            ("\"orm\", \"drivers-all\"", ""),
            ("\"strict-sqlite\"", "\"strict-postgres\""),
            ("", ""),
        ] {
            let error = registry_feature(&manifest(rullst, orm)).unwrap_err();
            assert_eq!(error.kind(), ErrorKind::InvalidInput);
            assert!(
                error
                    .to_string()
                    .contains("exactly one strict database profile")
            );
        }
        assert!(registry_feature("[package]\nname = \"demo\"\n").is_err());
        assert_eq!(
            registry_feature("not toml [").unwrap_err().to_string(),
            "Cargo.toml is not valid TOML"
        );
    }
}
