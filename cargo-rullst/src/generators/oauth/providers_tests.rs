//! `make:oauth github|microsoft|apple` and several providers in one project.
use super::*;

const SAAS: usize = crate::blueprints::SAAS_BLUEPRINT_ID;

fn migrations_of(root: &Path) -> usize {
    fs::read_dir(root.join("src/migrations"))
        .unwrap()
        .filter(|entry| {
            entry
                .as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(MIGRATION_SUFFIX)
        })
        .count()
}

fn clean(source: &str) {
    assert!(!source.contains("__"), "unreplaced placeholder");
    for forbidden in [".unwrap(", ".expect(", "panic!("] {
        assert!(!source.contains(forbidden), "{forbidden}");
    }
}

#[test]
fn github_alone_is_oauth_keyed_by_the_numeric_id() {
    let (_temporary, root) = starter(SAAS, false, "Sqlite");
    let notes = installed_provider(&root, "github");
    assert_eq!(
        notes[0],
        "GitHub sign-in added: GET /auth/github starts it and GET /auth/github/callback finishes it."
    );
    assert!(notes.contains(&"Set GITHUB_CLIENT_ID, GITHUB_CLIENT_SECRET, and register <RULLST_PUBLIC_ORIGIN>/auth/github/callback (or GITHUB_REDIRECT_URL) with the provider.".to_owned()));
    assert!(notes.contains(&"Try it offline: GITHUB_CLIENT_ID=mock_github GITHUB_CLIENT_SECRET=mock_github cargo run --features oauth-mock (development only).".to_owned()));
    assert!(notes.iter().any(|note| note.contains("numeric account id")));
    let controller = read(&root, "src/controllers/oauth/mod.rs");
    assert!(controller.contains("pub mod github;"));
    assert!(!controller.contains("pub mod google;"));
    let github = read(&root, "src/controllers/oauth/github.rs");
    assert!(github.contains("LoginOptions::oauth(PROVIDER)"));
    assert!(github.contains("pub(crate) const PROVIDER: &str = \"github\";"));
    assert!(github.contains("GithubProvider::try_new("));
    clean(&controller);
    clean(&github);
    let router = read(&root, "src/main.rs");
    assert!(router.contains("controllers::oauth::github::initialize()?;"));
    assert!(router.contains(
        ".route(\"/auth/github/callback\", rullst::routing::get(controllers::oauth::github::callback)"
    ));
    assert!(
        read(&root, "src/pages/auth.rs")
            .contains("{crate::controllers::oauth::github::login_link()}")
    );
    let env = read(&root, ".env.example");
    assert!(env.contains("\nGITHUB_CLIENT_ID=\nGITHUB_CLIENT_SECRET=\n"));
    assert!(env.contains("# A GitHub OAuth App."));
    assert!(env.contains("\nGITHUB_REDIRECT_URL=\n"));
    assert!(!root.join("Rullst.toml").exists());
    assert_eq!(migrations_of(&root), 1);
}

#[test]
fn microsoft_needs_an_explicit_tenant() {
    let (_temporary, root) = starter(SAAS, false, "Sqlite");
    let notes = installed_provider(&root, "microsoft");
    assert!(notes.contains(&"Set MICROSOFT_TENANT, MICROSOFT_CLIENT_ID, MICROSOFT_CLIENT_SECRET, and register <RULLST_PUBLIC_ORIGIN>/auth/microsoft/callback (or MICROSOFT_REDIRECT_URL) with the provider.".to_owned()));
    assert!(notes.contains(&"Try it offline: MICROSOFT_TENANT=common MICROSOFT_CLIENT_ID=mock_microsoft MICROSOFT_CLIENT_SECRET=mock_microsoft cargo run --features oauth-mock (development only).".to_owned()));
    assert!(notes.iter().any(|note| note.contains("xms_edov")));
    let microsoft = read(&root, "src/controllers/oauth/microsoft.rs");
    assert!(microsoft.contains("LoginOptions::oidc(PROVIDER)"));
    assert!(microsoft.contains("MicrosoftTenant::parse("));
    assert!(microsoft.contains("super::setting(\"MICROSOFT_TENANT\")?"));
    clean(&microsoft);
    let env = read(&root, ".env.example");
    assert!(env.contains("\nMICROSOFT_TENANT=\nMICROSOFT_CLIENT_ID=\nMICROSOFT_CLIENT_SECRET=\n"));
    assert!(env.contains("# MICROSOFT_TENANT: common, organizations, consumers"));
}

#[test]
fn apple_posts_its_callback_outside_the_csrf_token() {
    let (_temporary, root) = starter(SAAS, false, "Sqlite");
    let notes = installed_provider(&root, "apple");
    assert_eq!(
        notes[0],
        "Apple sign-in added: GET /auth/apple starts it and POST /auth/apple/callback finishes it."
    );
    assert!(notes.contains(&"Set APPLE_CLIENT_ID, APPLE_TEAM_ID, APPLE_KEY_ID, APPLE_PRIVATE_KEY, and register <RULLST_PUBLIC_ORIGIN>/auth/apple/callback (or APPLE_REDIRECT_URL) with the provider.".to_owned()));
    assert!(notes.contains(&"Try it offline: APPLE_CLIENT_ID=mock_apple APPLE_TEAM_ID=mock_apple APPLE_KEY_ID=mock_apple APPLE_PRIVATE_KEY=mock_apple cargo run --features oauth-mock (development only).".to_owned()));
    assert!(
        notes
            .iter()
            .any(|note| note.contains("csrf_signed_webhook_paths"))
    );
    let router = read(&root, "src/main.rs");
    assert!(router.contains(
        ".route(\"/auth/apple/callback\", rullst::routing::post(controllers::oauth::apple::callback)\n        .get(controllers::oauth::apple::mock_callback)\n        .layer(rullst::server::from_fn(controllers::auth_controller::credential_rate_limit)))"
    ));
    let apple = read(&root, "src/controllers/oauth/apple.rs");
    assert!(apple.contains(".verify_signing_key()"));
    assert!(apple.contains("AppleProvider::name_from_user_field"));
    clean(&apple);
    assert!(!apple.contains("BEGIN PRIVATE KEY"));
    let config: toml::Table = toml::from_str(&read(&root, "Rullst.toml")).unwrap();
    assert_eq!(
        config["security"]["csrf_signed_webhook_paths"],
        toml::Value::Array(vec!["/auth/apple/callback".into()])
    );
    assert!(read(&root, "Rullst.toml").contains("# make:oauth: the identity provider POSTs"));
    let env = read(&root, ".env.example");
    assert!(
        env.contains("\nAPPLE_CLIENT_ID=\nAPPLE_TEAM_ID=\nAPPLE_KEY_ID=\nAPPLE_PRIVATE_KEY=\n")
    );
}

#[test]
fn the_csrf_exemption_joins_an_existing_rullst_toml() {
    let path = "/auth/apple/callback";
    let (_temporary, root) = starter(SAAS, false, "Sqlite");
    let toml_path = root.join("Rullst.toml");
    fs::write(
        &toml_path,
        "[app]\nport = 3000\n\n[security]\n# signed\ncsrf_signed_webhook_paths = [\"/billing/webhook\"]\n",
    )
    .unwrap();
    writes::apply(&[edits::csrf_exemption(&root, path).unwrap().unwrap()]).unwrap();
    let updated = read(&root, "Rullst.toml");
    assert!(updated.contains("# signed\n"));
    let config: toml::Table = toml::from_str(&updated).unwrap();
    assert_eq!(
        config["security"]["csrf_signed_webhook_paths"],
        toml::Value::Array(vec!["/billing/webhook".into(), path.into()])
    );
    assert_eq!(config["app"]["port"].as_integer(), Some(3000));
    // Listed once.
    assert!(edits::csrf_exemption(&root, path).unwrap().is_none());

    fs::write(&toml_path, "[app]\nport = 3000\n").unwrap();
    writes::apply(&[edits::csrf_exemption(&root, path).unwrap().unwrap()]).unwrap();
    let config: toml::Table = toml::from_str(&read(&root, "Rullst.toml")).unwrap();
    assert_eq!(
        config["security"]["csrf_signed_webhook_paths"],
        toml::Value::Array(vec![path.into()])
    );

    fs::write(&toml_path, "[security]\nenable_pii_masking = false\n").unwrap();
    writes::apply(&[edits::csrf_exemption(&root, path).unwrap().unwrap()]).unwrap();
    let updated = read(&root, "Rullst.toml");
    assert!(updated.contains("# make:oauth: the identity provider POSTs"));
    assert!(updated.contains("enable_pii_masking = false"));

    for refused in [
        "security = 1\n",
        "[security]\ncsrf_signed_webhook_paths = \"/x\"\n",
    ] {
        fs::write(&toml_path, refused).unwrap();
        assert!(edits::csrf_exemption(&root, path).is_err(), "{refused}");
    }
}

#[test]
fn several_providers_share_one_flow_table_and_migration() {
    let (_temporary, root) = starter(SAAS, false, "Sqlite");
    let first = installed_provider(&root, "google");
    assert!(first.iter().any(|note| note.contains("db:migrate")));
    for name in ["github", "microsoft", "apple"] {
        let notes = installed_provider(&root, name);
        // Only the first provider creates the table.
        assert!(
            notes.iter().all(|note| !note.contains("db:migrate")),
            "{name}"
        );
    }
    assert_eq!(migrations_of(&root), 1);
    let controller = read(&root, "src/controllers/oauth/mod.rs");
    for name in ["google", "github", "microsoft", "apple"] {
        assert_eq!(
            controller.matches(&format!("pub mod {name};")).count(),
            1,
            "{name}"
        );
        assert!(
            root.join(format!("src/controllers/oauth/{name}.rs"))
                .is_file()
        );
        let router = read(&root, "src/main.rs");
        assert_eq!(
            router
                .matches(&format!("controllers::oauth::{name}::initialize()?;"))
                .count(),
            1
        );
        assert!(router.contains(&format!(".route(\"/auth/{name}\", ")));
        assert!(read(&root, "src/pages/auth.rs").contains(&format!(
            "{{crate::controllers::oauth::{name}::login_link()}}"
        )));
    }
    assert_eq!(
        read(&root, "src/models/mod.rs")
            .matches("pub mod oauth_identity;")
            .count(),
        1
    );
    assert_eq!(
        read(&root, "src/controllers/mod.rs")
            .matches("pub mod oauth;")
            .count(),
        1
    );
    let env = read(&root, ".env.example");
    for setting in [
        "GOOGLE_CLIENT_ID=",
        "GITHUB_CLIENT_ID=",
        "MICROSOFT_TENANT=",
        "APPLE_PRIVATE_KEY=",
    ] {
        assert_eq!(env.matches(setting).count(), 1, "{setting}");
    }
    let manifest: toml::Table = toml::from_str(&read(&root, "Cargo.toml")).unwrap();
    let features = manifest["dependencies"]["rullst"]["features"]
        .as_array()
        .unwrap();
    assert_eq!(
        features
            .iter()
            .filter(|feature| feature.as_str() == Some("oauth"))
            .count(),
        1
    );
    // A provider already installed is refused, and nothing changes.
    let before = read(&root, "src/controllers/oauth/mod.rs");
    assert!(plan(&root, "github", TIMESTAMP).is_err());
    assert_eq!(read(&root, "src/controllers/oauth/mod.rs"), before);
}

#[test]
fn an_unrecognized_router_gets_the_form_post_routes_as_instructions() {
    let (_temporary, root) = starter(SAAS, false, "Sqlite");
    let router = read(&root, "src/main.rs").replace(
        "rullst::routing::post(controllers::auth_controller::login_submit)",
        "rullst::routing::post(controllers::auth_controller::my_login)",
    );
    fs::write(root.join("src/main.rs"), &router).unwrap();
    let notes = installed_provider(&root, "apple");
    assert_eq!(read(&root, "src/main.rs"), router);
    assert!(notes.contains(&"Mount the routes yourself, behind the CSRF layer: call controllers::oauth::apple::initialize()? where the router is built, then route GET /auth/apple to controllers::oauth::apple::start, and POST /auth/apple/callback to controllers::oauth::apple::callback with GET to controllers::oauth::apple::mock_callback, each with controllers::auth_controller::credential_rate_limit.".to_owned()));
    // The CSRF exemption does not depend on the router.
    assert!(read(&root, "Rullst.toml").contains("/auth/apple/callback"));
}
