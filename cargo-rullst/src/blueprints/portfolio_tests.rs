#![allow(clippy::panic)]

use super::*;

fn file<'a>(manifest: &'a [(&str, String)], path: &str) -> &'a str {
    manifest
        .iter()
        .find_map(|(candidate, contents)| (*candidate == path).then_some(contents.as_str()))
        .unwrap_or_else(|| panic!("missing portfolio manifest entry {path}"))
}

#[test]
fn repository_portfolio_emits_every_repository_and_routes_through_them() {
    for pattern in ["Repository", "Hybrid"] {
        let manifest = file_manifest("portfolio_app", false, pattern, "Zero-Bundle HTMX");
        for path in [
            "src/repositories/mod.rs",
            "src/repositories/profile_repository.rs",
            "src/repositories/project_repository.rs",
            "src/repositories/experience_repository.rs",
            "src/repositories/skill_repository.rs",
        ] {
            assert!(
                manifest.iter().any(|(candidate, _)| *candidate == path),
                "{pattern} profile omitted {path}"
            );
        }
        let controller = file(&manifest, "src/controllers/portfolio_controller.rs");
        for repository in [
            "ProfileRepository::get()",
            "ProjectRepository::all()",
            "ExperienceRepository::all()",
            "SkillRepository::all()",
        ] {
            assert!(controller.contains(repository));
        }
        assert!(file(&manifest, "src/main.rs").contains("pub mod repositories;"));
    }
}

#[test]
fn hot_repository_portfolio_keeps_library_and_binary_modules_consistent() {
    let manifest = file_manifest("portfolio_app", true, "Repository", "Zero-Bundle HTMX");
    let library = file(&manifest, "src/lib.rs");
    let binary = file(&manifest, "src/main.rs");
    assert!(library.contains("pub mod repositories;"));
    assert!(binary.contains("pub mod repositories;"));
    assert!(binary.contains("portfolio_app::router()?"));
    assert!(file(&manifest, "src/pages/home.rs").contains("Zero-Bundle HTMX"));
}
