//! The Turso-primary starter refuses an empty `TURSO_DATABASE_URL` in staging
//! and production before the offline fallback can create its local file.

use super::blank;

#[test]
fn every_turso_layout_checks_the_url_before_initializing_the_store() {
    for (api, hot_reload) in [(false, false), (true, false), (false, true), (true, true)] {
        let manifest = blank::file_manifest(
            "demo",
            "demo",
            api,
            hot_reload,
            true,
            "Turso Active Record",
            "Zero-Bundle HTMX",
        );
        let (_, main) = manifest
            .iter()
            .find(|(path, _)| *path == "src/main.rs")
            .unwrap_or_else(|| panic!("api {api}, hot {hot_reload}: no src/main.rs"));
        let check = main
            .find("if turso_url.trim().is_empty() && environment.requires_secure_defaults()")
            .unwrap_or_else(|| panic!("api {api}, hot {hot_reload}: no URL check"));
        let init = main
            .find("TursoOrm::init(turso)")
            .unwrap_or_else(|| panic!("api {api}, hot {hot_reload}: no init"));
        assert!(check < init, "api {api}, hot {hot_reload}");
        assert!(main.contains("rullst::config::Environment::resolve("));
        assert!(main.contains("TURSO_DATABASE_URL is empty or unset in the {environment}"));
        assert!(main.contains("TursoConfig::new(\n        turso_url,"));
    }
}
