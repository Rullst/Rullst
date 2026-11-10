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

/// Route fixtures, including deliberately unclassified ones. The repository's
/// own `cargo rullst audit --idor` reads this file as a standalone source, so
/// they stay inside a top-level `#[cfg(test)]` item, which the scan skips.
#[cfg(test)]
mod fixtures {
    use super::*;

    #[test]
    fn public_read_routes_require_an_explicit_adjacent_classification() {
        let classified = r#"
    // rullst-access: public — published articles are intentionally public.
    get("/posts/{slug}" => show),
    "#;
        assert!(findings(classified).is_empty());

        let unclassified = r#"get("/posts/{slug}" => show),"#;
        let result = findings(unclassified);
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("missing an adjacent"));
        assert!(result[0].contains("/posts/{slug}"));
    }

    #[test]
    fn public_classification_never_suppresses_a_mutating_route() {
        let source = r#"
    // rullst-access: public — this annotation is unsafe for a mutation.
    delete("/documents/{id}" => destroy),
    "#;
        let result = findings(source);
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("accepted only for a recognized GET"));
    }

    #[test]
    fn classifications_require_a_concrete_adjacent_reason() {
        let source = r#"
    // rullst-access: public
    get("/posts/{slug}" => show),
    "#;
        let result = findings(source);
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("must include a concrete reason"));
    }

    #[test]
    fn owner_routes_require_the_owner_or_role_guard() {
        let guarded = r#"
    // rullst-access: owner — the handler compares the authenticated subject.
    get("/accounts/:account_id" => show_account),

    fn authorize(ctx: &UserContext, owner: &str) {
    let _ = RbacGuard::authorize_owner_or_role(ctx, owner, "admin");
    }
    "#;
        assert!(findings(guarded).is_empty());

        let missing_guard = r#"
    // rullst-access: owner — no ownership check actually exists.
    get("/accounts/:account_id" => show_account),
    "#;
        assert_eq!(findings(missing_guard).len(), 1);
    }

    #[test]
    fn admin_routes_require_a_recognized_server_boundary() {
        let protected = r#"
    // rullst-access: admin — protected as a group below.
    post("/products/{{id}}/add-stock" => add_stock),
    let protected = admin_access.protect_router(admin_routes.into_axum())?;
    "#;
        assert!(findings(protected).is_empty());

        let annotation_only = r#"
    // rullst-access: admin — a comment alone must not suppress the finding.
    post("/products/{id}/add-stock" => add_stock),
    "#;
        assert_eq!(findings(annotation_only).len(), 1);
    }

    #[test]
    fn unrelated_user_context_no_longer_hides_an_unclassified_route() {
        let source = r#"
    get("/orders/{order_id}" => show_order),
    fn unrelated(_context: UserContext) {}
    "#;
        assert_eq!(findings(source).len(), 1);
    }

    #[test]
    fn routes_after_an_early_test_declaration_are_still_scanned() {
        // The old scan truncated the file at the first top-level
        // `#[cfg(test)]` and reported this unclassified route as clean.
        let source = r#"use rullst::prelude::*;
    #[cfg(test)]
    mod tests;

    fn routes() -> Router {
    routes! {
        get("/accounts/{id}" => show_account),
    }
    }

    #[cfg(test)]
    mod inline {
    fn fixture() { get("/fixtures/{id}" => show) }
    }
    "#;
        let result = findings(source);
        assert_eq!(result.len(), 1, "{result:?}");
        assert!(result[0].contains("routes.rs:7"));
        assert!(result[0].contains("/accounts/{id}"));
    }

    #[test]
    fn every_route_on_a_single_line_list_is_checked() {
        // The old scan read only the first literal, "/", and skipped the line.
        let source = r#"routes! { get("/" => home), get("/users/{id}" => show_user) }"#;
        let result = findings(source);
        assert_eq!(result.len(), 1, "{result:?}");
        assert!(result[0].contains("/users/{id}"));

        let both = r#"routes! { get("/teams/{team}" => team), delete("/users/{id}" => drop) }"#;
        assert_eq!(findings(both).len(), 2);
        assert!(quoted_literals(r#"a("x\"y", "/z")"#).eq([r#"x\"y"#, "/z"]));
    }

    #[test]
    fn guard_mentions_in_comments_are_not_evidence() {
        // The old scan accepted any substring, so these comments satisfied
        // the owner classification.
        let commented = r#"
    // rullst-access: owner — the handler compares the authenticated subject.
    get("/accounts/{id}" => show_account),
    // TODO use RbacGuard::authorize_owner_or_role
    /// Calls `admin_access.protect_router(..)` later.
    fn show_account() {}
    "#;
        let result = findings(commented);
        assert_eq!(result.len(), 1, "{result:?}");
        assert!(result[0].contains("this crate's non-test code"));
    }

    #[test]
    fn guard_evidence_is_crate_wide_and_ignores_test_items() {
        let project = tempfile::tempdir().expect("temporary project");
        let src = project.path().join("src");
        fs::create_dir_all(&src).expect("source directory");
        fs::write(
            src.join("routes.rs"),
            "// rullst-access: owner — checked by the guard module.\nget(\"/orders/{id}\" => show),\n",
        )
        .expect("route fixture");
        fs::write(
            src.join("guard.rs"),
            "#[cfg(test)]\nmod tests { fn t() { RbacGuard::authorize_owner_or_role(c, o, \"admin\"); } }\n",
        )
        .expect("test-only guard fixture");
        let (count, warnings) = scan_idor_vulnerabilities(&src);
        assert_eq!(count, 1, "{warnings:?}");

        fs::write(
            src.join("guard.rs"),
            "fn guard() { RbacGuard::authorize_owner_or_role(c, o, \"admin\"); }\n",
        )
        .expect("guard fixture");
        let (count, warnings) = scan_idor_vulnerabilities(&src);
        assert_eq!(count, 0, "{warnings:?}");
    }

    #[test]
    fn a_comment_inside_a_multiline_route_call_keeps_the_route_in_scope() {
        // The old scan ended the `.route(` continuation at the comment line and
        // never inspected the path literal below it.
        let note_only = r#"
    Router::new().route(
    // Removes one invoice.
    "/invoices/{id}",
    delete(remove_invoice),
    )
    "#;
        let result = findings(note_only);
        assert_eq!(result.len(), 1, "{result:?}");
        assert!(result[0].contains("missing an adjacent"));

        let classified = r#"
    let protected = admin_access.protect_router(router)?;
    Router::new().route(
    // rullst-access: admin — composed behind the administrator boundary.
    "/invoices/{id}",
    delete(remove_invoice),
    )
    "#;
        assert!(findings(classified).is_empty());
    }

    #[test]
    fn multiline_axum_routes_and_generic_parameter_names_are_detected() {
        let source = r#"
    Router::new().route(
    "/invoices/{invoice_number}",
    delete(remove_invoice),
    )
    "#;
        let result = findings(source);
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("/invoices/{invoice_number}"));
    }

    #[test]
    fn machine_routes_need_a_registration_with_the_same_method_and_path() {
        let registered = r#"
    routes! {
        // rullst-access: machine — the provider's address token, confirmed at the source.
        post("/payments/{reference}/{token}" => webhook),
    }
    fn policy() -> MachineEndpointPolicy {
        MachineEndpointPolicy::new(vec![MachineEndpoint::address_token(
            Method::POST,
            "/payments/{reference}/{token}",
            Verifier,
        )?])
    }
    "#;
        assert!(
            findings(registered).is_empty(),
            "{:?}",
            findings(registered)
        );

        for (route, why) in [
            (
                r#"post("/other/{reference}/{token}" => webhook),"#,
                "another path",
            ),
            (
                r#"put("/payments/{reference}/{token}" => webhook),"#,
                "another method",
            ),
            (
                r#"get("/payments/{reference}/{token}" => show),"#,
                "a read route",
            ),
            (
                r#".route("/payments/{reference}/{token}", get(show).post(webhook))"#,
                "a read method beside the machine one",
            ),
        ] {
            let source = format!(
                "// rullst-access: machine — confirmed at the source.\n{route}\n\
                 MachineEndpoint::address_token(Method::POST, \"/payments/{{reference}}/{{token}}\", v)?;\n"
            );
            let result = findings(&source);
            assert_eq!(result.len(), 1, "{why}: {result:?}");
            assert!(
                result[0].contains("machine classification requires"),
                "{why}"
            );
            assert!(!result[0].contains("classify it as machine"), "{why}");
        }

        let unregistered = r#"
    // rullst-access: machine — no registration exists.
    post("/payments/{reference}/{token}" => webhook),
    "#;
        assert_eq!(findings(unregistered).len(), 1);
    }

    #[test]
    fn an_unclassified_machine_route_is_pointed_at_the_machine_class() {
        let source = r#"
    post("/payments/{reference}/{token}" => webhook),
    post("/orders/{id}" => update),
    fn policy() { MachineEndpoint::signed_webhook(Method::POST, "/payments/{r}/{t}", v); }
    "#;
        let result = findings(source);
        assert_eq!(result.len(), 2, "{result:?}");
        assert!(result[0].contains("public|owner|role|admin|machine"));
        assert!(result[0].ends_with("so classify it as machine"));
        assert!(!result[1].contains("classify it as machine"));
    }

    #[test]
    fn multiline_machine_routes_take_the_method_from_the_next_line() {
        let source = r#"
    Router::new().route(
        // rullst-access: machine — bearer API token.
        "/api/posts/{id}",
        // Updates one post.
        put(update_post),
    );
    MachineEndpoint::bearer(Method::PUT, "/api/posts/{id}", api_token)?;
    "#;
        assert!(findings(source).is_empty(), "{:?}", findings(source));
        let without_method = r#"
    Router::new().route(
        // rullst-access: machine — bearer API token.
        "/api/posts/{id}",
        handler,
    );
    MachineEndpoint::bearer(Method::PUT, "/api/posts/{id}", api_token)?;
    "#;
        assert_eq!(findings(without_method).len(), 1);
    }

    #[test]
    fn machine_registrations_are_crate_wide_constants_included() {
        let project = tempfile::tempdir().unwrap();
        let root = project.path().canonicalize().unwrap();
        let route = "// rullst-access: machine — address token checked by the verifier.\n\
                     post(\"/pagamentos/infinitepay/{referencia}/{token}\" => webhook),\n";
        write(&root.join("app/src/main.rs"), route);
        write(
            &root.join("app/src/services/infinitepay.rs"),
            "pub const ROTA_WEBHOOK: &str = \"/pagamentos/infinitepay/{referencia}/{token}\";\n",
        );
        // A registration only in test code is not evidence.
        write(
            &root.join("app/src/policy.rs"),
            "#[cfg(test)]\nmod tests { fn p() { MachineEndpoint::address_token(Method::POST, infinitepay::ROTA_WEBHOOK, v); } }\n",
        );
        let (count, warnings) = scan_idor_vulnerabilities(&root);
        assert_eq!(count, 1, "{warnings:?}");

        write(
            &root.join("app/src/policy.rs"),
            "fn p() { MachineEndpoint::address_token(Method::POST, infinitepay::ROTA_WEBHOOK, v); }\n",
        );
        let (count, warnings) = scan_idor_vulnerabilities(&root);
        assert_eq!(count, 0, "{warnings:?}");

        // Another crate's registration does not cover this one's route.
        write(&root.join("other/src/lib.rs"), route);
        let (count, warnings) = scan_idor_vulnerabilities(&root);
        assert_eq!(count, 1, "{warnings:?}");
        assert!(warnings[0].contains("other/src/lib.rs"), "{warnings:?}");
    }
}
