use super::*;
use crate::blueprints::{self, BLANK_BLUEPRINT_ID, LMS_BLUEPRINT_ID};
use crate::generators::project::wizard::catalog::BLUEPRINTS;
use crate::generators::project::wizard::plan::Database;

fn plan(name: &str, blueprint: usize) -> ProjectPlan {
    ProjectPlan {
        name: name.to_string(),
        blueprint,
        api: false,
        database: Database::Provider("Sqlite"),
        ai: false,
        redis: false,
        docker: false,
        nix: false,
        buildah: false,
        requested: Vec::new(),
        add_ons: Vec::new(),
        skip_initial_migration: false,
    }
}

/// The blueprint's own template manifest, independent of the preview code.
fn manifest_paths(blueprint: usize) -> Vec<&'static str> {
    let (orm, frontend) = ("Active Record", "Zero-Bundle HTMX");
    let manifest = match blueprint {
        BLANK_BLUEPRINT_ID => {
            blueprints::blank::file_manifest("demo", "demo", false, false, true, orm, frontend)
        }
        LMS_BLUEPRINT_ID => blueprints::lms::file_manifest("demo", false, orm, frontend),
        blueprints::SAAS_BLUEPRINT_ID => {
            blueprints::saas::file_manifest("demo", false, orm, frontend)
        }
        blueprints::BLOG_BLUEPRINT_ID => {
            blueprints::blog::file_manifest("demo", false, orm, frontend)
        }
        blueprints::PORTFOLIO_BLUEPRINT_ID => {
            blueprints::portfolio::file_manifest("demo", false, orm, frontend)
        }
        _ => blueprints::erp::file_manifest("demo", false, orm, frontend),
    };
    manifest.into_iter().map(|(path, _)| path).collect()
}

#[test]
fn previews_come_from_the_real_writers_for_every_blueprint() {
    for info in BLUEPRINTS {
        let files = rendered_files(&plan("preview-probe", info.id)).expect("preview renders");
        for path in manifest_paths(info.id) {
            assert!(
                files.iter().any(|file| file == path),
                "{}: {path}",
                info.flag
            );
        }
        for path in [
            "Cargo.toml",
            ".env",
            ".env.example",
            ".gitignore",
            ".llms.txt",
            ".rullst/context-map.json",
            ".cargo/config.toml",
            "static/htmx-1.9.12.min.js",
        ] {
            assert!(
                files.iter().any(|file| file == path),
                "{}: {path}",
                info.flag
            );
        }
        assert!(!files.iter().any(|file| file.contains('\\')));
    }
    assert!(
        !std::path::Path::new("preview-probe").exists(),
        "a preview never writes to the destination"
    );
}

#[test]
fn previews_follow_profile_and_packaging_choices() {
    let html = rendered_files(&plan("probe", BLANK_BLUEPRINT_ID)).expect("html preview");
    let mut api = plan("probe", BLANK_BLUEPRINT_ID);
    api.api = true;
    api.database = Database::None;
    let api = rendered_files(&api).expect("api preview");
    assert!(html.iter().any(|file| file == "static/rullst.css"));
    assert!(!api.iter().any(|file| file.starts_with("static/")));
    assert!(!api.iter().any(|file| file.starts_with("src/migrations/")));

    let mut packaged = plan("probe", BLANK_BLUEPRINT_ID);
    packaged.docker = true;
    packaged.nix = true;
    packaged.buildah = true;
    let packaged = rendered_files(&packaged).expect("packaged preview");
    for path in [
        "Dockerfile",
        ".dockerignore",
        "flake.nix",
        ".envrc",
        "build_buildah.sh",
    ] {
        assert!(packaged.iter().any(|file| file == path), "{path}");
        assert!(!html.iter().any(|file| file == path), "{path}");
    }

    assert_eq!(rendered_files(&plan("bad name", BLANK_BLUEPRINT_ID)), None);
}

#[test]
fn listing_is_sorted_relative_and_skips_links() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("b/c")).unwrap();
    std::fs::write(root.path().join("b/c/d.rs"), "").unwrap();
    std::fs::write(root.path().join("a.rs"), "").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(root.path().join("a.rs"), root.path().join("link.rs")).unwrap();
    assert_eq!(
        list_files(root.path()),
        Some(vec!["a.rs".to_string(), "b/c/d.rs".to_string()])
    );
}
