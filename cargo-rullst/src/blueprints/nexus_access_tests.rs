//! The SaaS starter admits its own administrators to Nexus.

use super::NEXUS_ACCESS;
use crate::blueprints::saas;

/// The auth starter, with and without the reloadable layout.
fn auth_starter_manifests() -> Vec<(&'static str, Vec<(&'static str, String)>)> {
    let (orm, frontend) = ("Active Record", "Zero-Bundle HTMX");
    [false, true]
        .into_iter()
        .map(|hot| ("saas", saas::file_manifest("demo", hot, orm, frontend)))
        .collect()
}

#[test]
fn auth_starters_admit_their_own_administrators_to_nexus() {
    let file = |manifest: &[(&str, String)], wanted: &str| {
        manifest
            .iter()
            .find_map(|(path, source)| (*path == wanted).then(|| source.clone()))
            .unwrap_or_else(|| panic!("missing {wanted}"))
    };
    for (blueprint, manifest) in auth_starter_manifests() {
        let access = file(&manifest, "src/middlewares/nexus_access.rs");
        syn::parse_file(&access).unwrap_or_else(|error| panic!("{blueprint}: {error}"));
        assert!(access.contains("impl NexusAuthorizer for NexusAdmins {"));
        assert!(access.contains("NexusAccess::allow(format!(\"user:{}\", user.id))"));
        assert!(file(&manifest, "src/middlewares/mod.rs").contains("pub mod nexus_access;"));
        // Hot-reload starters build the router in `src/lib.rs`.
        let main = manifest
            .iter()
            .filter(|(path, _)| matches!(*path, "src/main.rs" | "src/lib.rs"))
            .map(|(_, source)| source.as_str())
            .collect::<String>();
        assert!(main.contains(
            "NexusAuthPolicy::local_development_or_application(\n        middlewares::nexus_access::NexusAdmins,\n    );"
        ));
        assert!(!main.contains("local_development_or_basic_from_env"));
        let user = file(&manifest, "src/models/user.rs");
        assert!(user.contains("    #[orm(db_default)]\n    pub is_admin: bool,\n"));
        assert!(user.contains("FieldMeta { name: \"is_admin\", label: \"Nexus administrator\", kind: FieldKind::Boolean, hidden: false, readonly: false },"));
        assert!(
            manifest
                .iter()
                .any(|(path, source)| path.starts_with("src/migrations/")
                    && source.contains(
                        "table.boolean(\"is_admin\").not_null().default(ColumnDefault::Integer(0));"
                    ))
        );
        assert!(
            file(&manifest, "src/controllers/auth_controller.rs")
                .contains("        is_admin: false,\n")
        );
    }
}

#[test]
fn the_authorizer_template_is_the_emitted_file() {
    for (blueprint, manifest) in auth_starter_manifests() {
        let emitted = manifest
            .iter()
            .find(|(path, _)| *path == "src/middlewares/nexus_access.rs")
            .map(|(_, source)| source.as_str());
        assert_eq!(emitted, Some(NEXUS_ACCESS), "{blueprint}");
    }
}
