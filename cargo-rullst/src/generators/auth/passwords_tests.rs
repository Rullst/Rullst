use super::*;
use crate::blueprints::{lms, saas};
use crate::generators::auth::views::AUTH_VIEWS_TEMPLATE;

fn starter(name: &str, hot: bool) -> Vec<(&'static str, String)> {
    let (orm, frontend) = ("Active Record", "Zero-Bundle HTMX");
    match name {
        "saas" => saas::file_manifest("demo", hot, orm, frontend),
        _ => lms::file_manifest("demo", hot, orm, frontend),
    }
}

fn file<'a>(manifest: &'a [(&'static str, String)], path: &str) -> &'a str {
    manifest
        .iter()
        .find_map(|(candidate, source)| (*candidate == path).then_some(source.as_str()))
        .unwrap_or_else(|| panic!("missing {path}"))
}

#[test]
fn password_templates_parse_and_never_panic() {
    let migration =
        render_password_reset_migration("m20260101000000_create_password_reset_tokens_table");
    for source in [
        render_password_controller(),
        render_password_reset_model(),
        migration.clone(),
    ] {
        syn::parse_file(&source).expect("password source must parse");
        for forbidden in [
            ".unwrap(",
            ".expect(",
            "panic!(",
            "use axum",
            "rullst_orm::",
        ] {
            assert!(!source.contains(forbidden), "{forbidden}");
        }
    }
    assert!(!migration.contains("__MIGRATION_NAME__"));
    assert!(migration.contains("\"m20260101000000_create_password_reset_tokens_table\""));
    assert_eq!(MIGRATION_SUFFIX, "_create_password_reset_tokens_table.rs");
}

#[test]
fn reset_tokens_are_digest_only_single_use_and_expire() {
    let model = render_password_reset_model();
    assert!(model.contains("pub const RESET_TOKEN_LIFETIME_MINUTES: u32 = 30;"));
    assert!(model.contains("pub const RESET_REQUEST_COOLDOWN_SECONDS: i64 = 60;"));
    // Only the digest of a CSPRNG token reaches the database.
    assert!(
        model.contains("let token = PasswordResetToken::generate().map_err(ResetError::Random)?;")
    );
    assert!(model.contains("            .bind(token.digest())\n"));
    assert!(!model.contains(".bind(token.expose())"));
    // A new request replaces the account's token once the cooldown passed.
    assert!(model.contains("now.saturating_sub(issued_at) < RESET_REQUEST_COOLDOWN_SECONDS"));
    assert!(model.contains("\"DELETE FROM password_reset_tokens WHERE user_id = ?\""));
    // Lookup: live, compared in constant time.
    assert!(model.contains(
        "*expires_at > now && rullst::auth::password_reset_digests_match(stored, digest)"
    ));
    // Redemption claims the live token once, then stores the hash and bumps
    // the epoch in the same transaction.
    assert!(model.contains(
        "\"DELETE FROM password_reset_tokens WHERE user_id = ? AND token_hash = ? AND expires_at > ?\""
    ));
    assert!(model.contains("        if claimed != 1 {\n            return Ok(false);\n        }"));
    assert!(model.contains(
        "\"UPDATE users SET password_hash = ?, updated_at = ?, session_epoch = session_epoch + 1 WHERE id = ?\""
    ));
    assert!(model.contains("Ok::<bool, sqlx::Error>(updated == 1)"));
    assert!(model.contains("rullst::orm::replace_placeholders(sql)"));

    let migration = render_password_reset_migration("m1_create_password_reset_tokens_table");
    assert!(migration.contains("table.integer(\"user_id\").not_null().primary();"));
    assert!(migration.contains(
        "table.string(\"token_hash\").not_null().col_type = \"VARCHAR(64)\".to_string();"
    ));
    assert!(migration.contains(
        "CREATE UNIQUE INDEX password_reset_tokens_hash_unique ON password_reset_tokens(token_hash)"
    ));
    assert!(migration.contains("Schema::drop_if_exists(\"password_reset_tokens\").await"));
}

#[test]
fn reset_links_use_the_configured_origin_and_requests_answer_alike() {
    let controller = render_password_controller();
    assert!(controller.contains("rullst::config::project_setting(\"RULLST_PUBLIC_ORIGIN\")"));
    assert!(controller.contains("const DEVELOPMENT_ORIGIN: &str = \"http://localhost:3000\";"));
    assert!(controller.contains("Ok(_) if development() => Ok(DEVELOPMENT_ORIGIN.to_owned()),"));
    assert!(
        controller
            .contains("let url = format!(\"{origin}/password/reset?token={}\", token.expose());")
    );
    assert!(controller.contains("match ActionLink::new(url, origin) {"));
    for host in ["header::HOST", "\"host\"", "Host:"] {
        assert!(!controller.contains(host), "{host}");
    }
    // Unknown, throttled and accepted requests share the answer and its
    // minimum duration; delivery never delays the response.
    assert!(
        controller
            .contains("const RESET_REQUEST_MIN_DURATION: Duration = Duration::from_millis(400);")
    );
    assert!(controller.contains(
        "rullst::runtime::time::sleep_until(started + RESET_REQUEST_MIN_DURATION).await;"
    ));
    assert!(controller.contains("if valid && let Err(error) = request_reset(&email).await {"));
    assert!(controller.contains("rullst::runtime::spawn(async move {"));
    assert!(controller.contains("Ok(mail) => Mail::send(mail.into_message()).await,"));
    assert!(controller.contains("expires_in_minutes: RESET_TOKEN_LIFETIME_MINUTES,"));
    // The development preview needs a debug build, development and no driver.
    assert!(controller.contains(
        "if cfg!(debug_assertions)\n        && development()\n        && matches!(rullst::config::project_setting(\"MAIL_DRIVER\"), Ok(None))"
    ));
    // Pages carrying a token stay out of caches and Referer headers.
    assert!(controller.contains("HeaderValue::from_static(\"no-referrer\")"));
    assert!(controller.contains(
        "headers.insert(header::CACHE_CONTROL, HeaderValue::from_static(\"no-store\"));"
    ));
    // A completed reset signs nobody in.
    assert!(controller.contains("Ok(()) => Redirect::to(\"/login?reset=done\").into_response(),"));
    let reset = controller
        .split("async fn reset_password(payload: ResetPasswordDto)")
        .nth(1)
        .and_then(|rest| rest.split("fn cookie_epoch(").next())
        .expect("reset handlers");
    assert!(!reset.contains("make_login_cookie"));
}

#[test]
fn password_change_checks_the_current_password_and_keeps_this_session() {
    let controller = render_password_controller();
    // Argon2 runs on the bounded blocking pool, never the async helpers.
    assert!(!controller.contains("_async("));
    assert!(controller.contains(
        "run_password_work(move || rullst_auth::verify_password(&current, &stored_hash)).await?;"
    ));
    assert!(
        controller.contains("run_password_work(move || rullst_auth::hash_password(&password))")
    );
    assert!(controller.contains("if !(current_valid && has_password) {\n        return Err(PasswordError::WrongCurrentPassword);"));
    assert!(
        controller.contains("Self::WrongCurrentPassword => \"The current password is incorrect\",")
    );
    // The cookie's own epoch guards the update, and only this browser gets
    // a cookie at the new epoch.
    assert!(controller.contains("(claims.user_id == user_id).then_some(claims.epoch)"));
    assert!(
        controller
            .contains("User::change_password(user_id, epoch, &password_hash, &utc_timestamp())")
    );
    assert!(controller.contains(
        "rullst_auth::make_login_cookie_with_epoch(user_id, next_epoch, rullst_auth::DEFAULT_SESSION_MAX_AGE)"
    ));
    assert!(
        controller.contains(
            "Ok(cookie) => redirect_with_cookie(\"/password/change?changed=1\", &cookie),"
        )
    );
    assert!(controller.contains(
        "Err(PasswordError::SessionRevoked) => {\n            redirect_with_cookie(\"/login\", &rullst_auth::make_logout_cookie())"
    ));
    // Both flows apply the registration policy.
    assert_eq!(
        controller
            .matches("check_new_password(&payload.password, &payload.password_confirmation)?;")
            .count(),
        2
    );
    assert!(
        controller
            .contains("if !valid_password(password) {\n        return Err(PasswordError::Policy);")
    );
}

#[test]
fn every_auth_view_renders_the_password_pages() {
    let views = [
        ("make:auth", AUTH_VIEWS_TEMPLATE.to_string()),
        (
            "saas",
            file(&starter("saas", false), "src/pages/auth.rs").to_string(),
        ),
        (
            "lms",
            file(&starter("lms", false), "src/pages/auth.rs").to_string(),
        ),
    ];
    for (name, source) in views {
        syn::parse_file(&source).unwrap_or_else(|error| panic!("{name}: {error}"));
        for expected in [
            "pub fn login_page(\n    csrf_token: &str,\n    error: Option<&str>,\n    notice: Option<&str>,\n",
            "pub fn forgot_password_page(\n    csrf_token: &str,\n    error: Option<&str>,\n    notice: Option<&str>,\n",
            "pub fn reset_password_page(\n    csrf_token: &str,\n    token: &str,\n    error: Option<&str>,\n",
            "pub fn change_password_page(\n    csrf_token: &str,\n    error: Option<&str>,\n    notice: Option<&str>,\n",
            "action=\"/password/forgot\"",
            "action=\"/password/reset\"",
            "action=\"/password/change\"",
            "<input type=\"hidden\" name=\"token\" value={token} />",
            "name=\"current_password\"",
            "name=\"password_confirmation\"",
            "autocomplete=\"new-password\"",
            "href=\"/password/forgot\"",
            "href=\"/password/change\"",
        ] {
            assert!(source.contains(expected), "{name}: {expected}");
        }
        // Each form posts its CSRF token; boolean attributes carry a value.
        assert_eq!(
            source.matches("<form").count(),
            source.matches("name=\"_token\"").count(),
            "{name}"
        );
        assert_eq!(
            source.matches(" required").count(),
            source.matches(" required=\"required\"").count(),
            "{name}"
        );
    }
}

#[test]
fn starters_ship_and_mount_the_password_flows() {
    for blueprint in ["saas", "lms"] {
        for hot in [false, true] {
            let manifest = starter(blueprint, hot);
            assert_eq!(
                file(&manifest, "src/controllers/password_controller.rs"),
                render_password_controller()
            );
            assert_eq!(
                file(&manifest, "src/models/password_reset.rs"),
                render_password_reset_model()
            );
            let migration = match blueprint {
                "saas" => "m20260601000001_create_password_reset_tokens_table",
                _ => "m20260827000001_create_password_reset_tokens_table",
            };
            assert_eq!(
                file(&manifest, &format!("src/migrations/{migration}.rs")),
                render_password_reset_migration(migration)
            );
            let migrations = file(&manifest, "src/migrations/mod.rs");
            assert!(migrations.contains(&format!("pub mod {migration};")));
            assert!(migrations.contains(&format!("Box::new({migration}::MigrationImpl),")));
            assert!(file(&manifest, "src/models/mod.rs").contains("pub mod password_reset;\n"));
            assert!(
                file(&manifest, "src/controllers/mod.rs")
                    .contains("pub mod password_controller;\n")
            );
            assert!(
                file(&manifest, "src/models/user.rs").contains(
                    "pub async fn change_password(\n        id: i32,\n        epoch: u64,"
                )
            );

            let router = file(&manifest, if hot { "src/lib.rs" } else { "src/main.rs" });
            for view in ["forgot_password_view", "reset_password_view"] {
                let path = view.trim_end_matches("_password_view");
                assert!(
                    router.contains(&format!(
                        "get(\"/password/{path}\" => controllers::password_controller::{view}),"
                    )),
                    "{blueprint}: {view}"
                );
            }
            for (path, handler) in [
                ("forgot", "forgot_password_submit"),
                ("reset", "reset_password_submit"),
            ] {
                assert!(router.contains(&format!(
                    ".route(\"/password/{path}\", rullst::routing::post(controllers::password_controller::{handler})\n        .layer(rullst::server::from_fn(controllers::auth_controller::credential_rate_limit)))"
                )), "{blueprint}: {handler} must be rate limited");
            }
            let change = match blueprint {
                "saas" => {
                    ".route(\"/password/change\", rullst::routing::post(controllers::password_controller::change_password_submit)\n        .layer(rullst::server::from_fn(controllers::auth_controller::credential_rate_limit))\n        .layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware)))"
                }
                _ => {
                    "    .route(\"/password/change\", rullst::routing::post(controllers::password_controller::change_password_submit)\n        .layer(rullst::server::from_fn(controllers::auth_controller::credential_rate_limit)))\n    .layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware));"
                }
            };
            assert!(
                router.contains(change),
                "{blueprint}: password change route"
            );
            // Every password route sits before the CSRF layer.
            let csrf = router.find("csrf_middleware").expect("CSRF layer");
            assert!(
                router.rfind("/password/").is_some_and(|last| last < csrf),
                "{blueprint}"
            );
        }
    }
}
