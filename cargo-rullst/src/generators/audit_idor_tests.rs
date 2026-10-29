//! Source selection, crate-wide guard evidence and route recognition.
use super::*;

fn findings(source: &str) -> Vec<String> {
    scan_idor_source(Path::new("src/routes.rs"), source)
}

fn write(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

#[test]
fn only_production_sources_under_src_are_collected() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path().canonicalize().unwrap();
    for file in [
        "src/routes.rs",
        "src/tests.rs",
        "src/handlers/tests/mod.rs",
        "tests/integration.rs",
        "examples/demo.rs",
    ] {
        write(&root.join(file), "fn f() {}\n");
    }
    let mut files = collect_rust_source_files(&root).files;
    files.sort();
    assert_eq!(files, [root.join("src/routes.rs")]);
    let mut files = collect_rust_source_files(&root.join("src")).files;
    files.sort();
    assert_eq!(files, [root.join("src/routes.rs")]);
}

#[test]
fn guard_evidence_accumulates_and_each_guard_counts() {
    let mut evidence = GuardEvidence::default();
    evidence.include("RbacGuard::authorize_owner_or_role(ctx, owner, \"admin\");");
    evidence.include("fn unrelated() {}");
    assert!(evidence.owner && evidence.role && !evidence.admin);
    for (content, admin) in [
        ("RbacGuard::authorize(ctx, \"editor\");", false),
        ("router.layer(RequireRoleLayer::new(\"admin\"));", true),
        ("access.protect_router(router);", true),
    ] {
        let mut evidence = GuardEvidence::default();
        evidence.include(content);
        assert!(evidence.role && !evidence.owner, "{content}");
        assert_eq!(evidence.admin, admin, "{content}");
    }
}

#[test]
fn guard_evidence_belongs_to_the_crate_that_holds_it() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path().canonicalize().unwrap();
    write(
        &root.join("guarded/src/lib.rs"),
        "fn guard() { RbacGuard::authorize_owner_or_role(c, o, \"admin\"); }\n",
    );
    write(
        &root.join("open/src/lib.rs"),
        "// rullst-access: owner — the handler compares the subject.\nget(\"/orders/{id}\" => show),\n",
    );
    let (count, warnings) = scan_idor_vulnerabilities(&root);
    assert_eq!(count, 1, "{warnings:?}");
    assert!(warnings[0].contains("open/src/lib.rs"), "{warnings:?}");
}

#[test]
fn quoted_paths_outside_route_calls_are_not_routes() {
    let source = "fn link() -> String {\n    let link = \"/users/{id}\";\n    link.into()\n}\n";
    assert!(findings(source).is_empty(), "{:?}", findings(source));
}

#[test]
fn a_bare_route_call_continues_onto_the_path_line() {
    let source = "let app = with(\nroute(\n    \"/items/{id}\",\n    delete(remove),\n)\n";
    assert_eq!(findings(source).len(), 1);
}

#[test]
fn role_routes_require_a_role_guard() {
    let classified =
        "// rullst-access: role — editors manage drafts.\nput(\"/drafts/{id}\" => update),\n";
    assert_eq!(findings(classified).len(), 1);
    let guarded = format!("{classified}fn check() {{ RbacGuard::authorize(ctx, \"editor\"); }}\n");
    assert!(findings(&guarded).is_empty());
}

#[test]
fn only_named_segments_are_parameters() {
    for parameter in [":id", "{id}", "{{id}}"] {
        assert!(is_parameter_segment(parameter), "{parameter}");
    }
    for literal in [":", "{}", "{*rest}", "items"] {
        assert!(!is_parameter_segment(literal), "{literal}");
    }
}

#[test]
fn public_is_accepted_only_for_get_routes() {
    let source = "// rullst-access: public — anyone may call it.\nRouter::new().route(\"/items/{id}\", any(handler)),\n";
    let result = findings(source);
    assert_eq!(result.len(), 1);
    assert!(result[0].contains("accepted only for a recognized GET"));
}
