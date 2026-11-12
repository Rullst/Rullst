//! Explicit preview consumer: authenticated choices and an own-account export,
//! for the recognized SaaS authentication or an application's own sign-in
//! through its authentication contract (`src/auth_contract.rs`).
use super::{auth_contract, consumer_files as files, consumer_support};
use clap::{Arg, ArgMatches, Command};
use files::Edit;
use std::{
    io,
    path::{Path, PathBuf},
};
use toml_edit::DocumentMut;

#[cfg(test)]
mod contract_tests;
mod routing;

pub(crate) fn command() -> Command {
    Command::new("make:privacy")
        .about("Add authenticated privacy choices and an own-account profile export (v13 preview)")
        .arg(
            Arg::new("blueprint")
                .long("blueprint")
                .default_value("saas")
                .value_parser(["saas"]),
        )
        .arg(
            Arg::new("privacy-source")
                .long("privacy-source")
                .value_parser(clap::value_parser!(PathBuf))
                .help("Explicit local privacy source matching this CLI; defaults to the matching registry version"),
        )
        .arg(
            Arg::new("purpose-version")
                .long("purpose-version")
                .required(true),
        )
        .arg(
            Arg::new("validity-seconds")
                .long("validity-seconds")
                .required(true)
                .value_parser(clap::value_parser!(u32).range(1..=31_536_000)),
        )
        .arg(
            Arg::new("tenant-ref")
                .long("tenant-ref")
                .required(true)
                .help("Server-owned SaaS tenant, fixed at generation"),
        )
        .arg(
            Arg::new("consent-store")
                .long("consent-store")
                .value_parser(["sqlite", "postgres"])
                .help("Consent store: PostgreSQL by default when the project's primary database is PostgreSQL, else a shared-local SQLite file"),
        )
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

pub(crate) fn run(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::current_dir()?;
    let source = matches
        .get_one::<PathBuf>("privacy-source")
        .map(PathBuf::as_path);
    let consumer = matches
        .get_one::<String>("blueprint")
        .ok_or_else(|| invalid("blueprint is required"))?;
    let version = matches
        .get_one::<String>("purpose-version")
        .ok_or_else(|| invalid("purpose version is required"))?;
    let lifetime = *matches
        .get_one::<u32>("validity-seconds")
        .ok_or_else(|| invalid("grant validity is required"))?;
    let tenant = matches
        .get_one::<String>("tenant-ref")
        .ok_or_else(|| invalid("tenant reference is required"))?;
    let store = matches
        .get_one::<String>("consent-store")
        .map(String::as_str);
    let planned = plan(&root, source, consumer, version, lifetime, tenant, store)?;
    files::apply(&planned.edits)?;
    println!(
        "Privacy choices installed at /privacy, with an own-account profile export. Read PRIVACY.md, initialize the private consent store once, and configure the independent form key before enabling choices."
    );
    for note in planned.notes {
        println!("{note}");
    }
    Ok(())
}

/// Where the authenticated account comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Accounts {
    /// The unmodified authentication of the SaaS starter.
    Starter,
    /// The application's own sign-in, through `src/auth_contract.rs`.
    Contract,
}

/// The validated edits and what is left to the developer.
struct Planned {
    edits: Vec<Edit>,
    notes: Vec<String>,
}

/// The recognized SaaS authentication, else the authentication contract.
fn accounts(root: &Path, consumer: &str) -> Result<Accounts, Box<dyn std::error::Error>> {
    let contract = auth_contract::present(root)?;
    let remedy = format!(
        "An application with its own sign-in can use the authentication contract in {} instead ({})",
        auth_contract::PATH,
        auth_contract::DOCS
    );
    match consumer_support::recognized_auth(root, consumer, &remedy) {
        Ok(()) => Ok(Accounts::Starter),
        Err(_) if contract => {
            auth_contract::check(root, auth_contract::PRIVACY)?;
            Ok(Accounts::Contract)
        }
        Err(error) => Err(error),
    }
}

/// Where the consent records live.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Store {
    /// One shared-local SQLite file (`consent-sqlite`).
    Sqlite,
    /// The authoritative PostgreSQL database (`consent-postgres`).
    Postgres,
}

impl Store {
    /// `--consent-store`, else PostgreSQL for a PostgreSQL project.
    fn select(requested: Option<&str>, manifest: &DocumentMut) -> Result<Self, io::Error> {
        match requested {
            Some("sqlite") => Ok(Self::Sqlite),
            Some("postgres") => Ok(Self::Postgres),
            Some(_) => Err(invalid("--consent-store must be sqlite or postgres")),
            None if consumer_support::primary_postgres(manifest) => Ok(Self::Postgres),
            None => Ok(Self::Sqlite),
        }
    }

    fn feature(self) -> &'static str {
        match self {
            Self::Sqlite => "consent-sqlite",
            Self::Postgres => "consent-postgres",
        }
    }

    /// Fills the store placeholders of a template.
    fn apply(self, template: &str) -> String {
        let (store, open, init, setup) = match self {
            Self::Sqlite => (
                "SqliteConsentStore",
                "let database = std::env::var_os(\"RULLST_PRIVACY_DATABASE\")\n            .filter(|value| !value.is_empty()).ok_or(ConsentError::InvalidConfiguration)?;\n        let store = SqliteConsentStore::open(std::path::Path::new(&database), 10_000).await?;",
                "let database = std::env::var_os(\"RULLST_PRIVACY_DATABASE\")\n        .filter(|value| !value.is_empty()).ok_or(ConsentError::InvalidConfiguration)?;\n    let store = SqliteConsentStore::initialize(std::path::Path::new(&database), 10_000).await?;\n    store.close().await;\n    println!(\"Initialized the private local consent store. Keep all application processes on this same file.\");",
                include_str!("store_sqlite.md.template"),
            ),
            Self::Postgres => (
                "PostgresConsentStore",
                "let database = std::env::var(\"RULLST_PRIVACY_DATABASE\")\n            .ok().filter(|value| !value.is_empty()).ok_or(ConsentError::InvalidConfiguration)?;\n        let store = PostgresConsentStore::connect(database, 10_000).await?;",
                "let database = std::env::var(\"RULLST_PRIVACY_DATABASE\")\n        .ok().filter(|value| !value.is_empty()).ok_or(ConsentError::InvalidConfiguration)?;\n    let store = PostgresConsentStore::initialize(database, 10_000).await?;\n    store.close().await;\n    println!(\"Initialized (or validated) the PostgreSQL consent store. Application startup only connects to it.\");",
                include_str!("store_postgres.md.template"),
            ),
        };
        template
            .replace("__STORE_TYPE__", store)
            .replace("__STORE_OPEN__", open)
            .replace("__STORE_INIT__", init)
            .replace("__STORE_SETUP__", setup.trim_end())
    }
}

fn plan(
    root: &Path,
    source: Option<&Path>,
    consumer: &str,
    version: &str,
    lifetime: u32,
    tenant: &str,
    store: Option<&str>,
) -> Result<Planned, Box<dyn std::error::Error>> {
    if consumer != "saas" {
        return Err(invalid("make:privacy supports only the SaaS consumer").into());
    }
    for value in [version, tenant] {
        if value.is_empty()
            || value.len() > 128
            || !value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':')
            })
        {
            return Err(invalid(
                "purpose version and tenant must be bounded opaque ASCII references",
            )
            .into());
        }
    }
    if !(1..=31_536_000).contains(&lifetime) {
        return Err(invalid("invalid consent validity").into());
    }
    let source = consumer_support::privacy_source(source, "src/consent/mod.rs")?;
    // Links above the project are the developer's; links beneath it are refused.
    let root = &files::project_root(root)?;
    let accounts = accounts(root, consumer)?;
    let manifest_path = root.join("Cargo.toml");
    let original = files::read(&manifest_path)?;
    let mut manifest: DocumentMut = original.parse()?;
    let store = Store::select(store, &manifest)?;
    consumer_support::privacy_dependency(
        root,
        &mut manifest,
        source.as_deref(),
        &[store.feature()],
    )?;
    for (name, version) in [("ring", "0.17"), ("hex", "0.4")] {
        if manifest["dependencies"].get(name).is_none() {
            manifest["dependencies"][name] = toml_edit::value(version);
        }
    }
    // A second binary must not make ordinary `cargo run` ambiguous.
    consumer_support::default_run(&mut manifest)?;
    let router_path = if root.join("src/lib.rs").exists() {
        root.join("src/lib.rs")
    } else {
        root.join("src/main.rs")
    };
    let router = files::read(&router_path)?;
    let mounted = match accounts {
        Accounts::Starter => Some(routing::mount(&router)?),
        Accounts::Contract => routing::mount_if_bounded(&router)?,
    };
    let registry = root.join("src/controllers/mod.rs");
    let old_registry = files::read(&registry)?;
    let new_registry = format!("{old_registry}\npub mod privacy_controller;\n");
    syn::parse_file(&new_registry)?;
    let mut notes = Vec::new();
    let mut edits = vec![Edit::replace(manifest_path, original, manifest.to_string())];
    match mounted {
        Some(updated) => edits.push(Edit::replace(router_path, router, updated)),
        None => notes.push("Mount the routes inside your CSRF layer (rullst::security::csrf_middleware): add `.merge_axum(controllers::privacy_controller::routes().into_axum())` to your router; the routes require auth_contract::require_account themselves.".to_owned()),
    }
    edits.push(Edit::replace(registry, old_registry, new_registry));
    let (controller, proof, profile) = match accounts {
        Accounts::Starter => (
            include_str!("controller.rs.template").to_owned(),
            include_str!("proof.rs.template").to_owned(),
            include_str!("profile.rs.template").to_owned(),
        ),
        Accounts::Contract => (
            include_str!("controller.rs.template").replace(
                "crate::middlewares::auth_middleware::auth_middleware",
                "crate::auth_contract::require_account",
            ),
            include_str!("proof.rs.template").replace(
                "rullst::auth::extract_session_cookie(headers)",
                "crate::auth_contract::session_binding(headers)",
            ),
            include_str!("profile_contract.rs.template").to_owned(),
        ),
    };
    let config = store
        .apply(include_str!("config.rs.template"))
        .replace("__NOTICE_VERSION__", &format!("{version:?}"))
        .replace("__VALIDITY_SECONDS__", &lifetime.to_string())
        .replace("__TENANT_REF__", &format!("{tenant:?}"));
    for (path, source) in [
        ("src/controllers/privacy_controller.rs", controller),
        ("src/controllers/privacy/config.rs", config),
        (
            "src/controllers/privacy/page.rs",
            include_str!("page.rs.template").to_owned(),
        ),
        ("src/controllers/privacy/proof.rs", proof),
        ("src/controllers/privacy/profile.rs", profile),
        (
            "src/bin/privacy-init.rs",
            store.apply(include_str!("init.rs.template")),
        ),
    ] {
        syn::parse_file(&source)?;
        edits.push(Edit::create(
            root.join(path),
            consumer_support::formatted(&source)?,
        )?);
    }
    edits.push(Edit::create(
        root.join("PRIVACY.md"),
        store.apply(include_str!("README.md.template")),
    )?);
    Ok(Planned { edits, notes })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_routes_bind_only_the_fixed_tenant_and_refuse_scope_hints() {
        let temporary = tempfile::tempdir().unwrap();
        // A symlinked parent, as macOS spells its temporary directory.
        let root = &files::symlinked_project(temporary.path());
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst = \"13\"\n",
        )
        .unwrap();
        crate::blueprints::apply(
            crate::blueprints::SAAS_BLUEPRINT_ID,
            root,
            "consumer",
            "consumer",
            false,
            false,
            true,
            "Active Record",
            "Zero-Bundle HTMX",
        )
        .unwrap();
        for tenant in ["", "tenant/other", "tenant?school=x"] {
            assert!(plan(root, None, "saas", "greeting-v1", 3600, tenant, None).is_err());
        }
        // A link beneath the project is still refused, and nothing is written through it.
        #[cfg(unix)]
        {
            let outside = temporary.path().join("outside");
            std::fs::create_dir(&outside).unwrap();
            let linked = root.join("src/controllers/privacy");
            std::os::unix::fs::symlink(&outside, &linked).unwrap();
            assert!(
                plan(
                    root,
                    None,
                    "saas",
                    "greeting-v1",
                    3600,
                    "tenant-alpha",
                    None
                )
                .is_err()
            );
            std::fs::remove_file(&linked).unwrap();
            assert_eq!(std::fs::read_dir(&outside).unwrap().count(), 0);
        }
        files::apply(
            &plan(
                root,
                None,
                "saas",
                "greeting-v1",
                3600,
                "tenant-alpha",
                None,
            )
            .unwrap()
            .edits,
        )
        .unwrap();
        let privacy = root.join("src/controllers/privacy");
        let config = std::fs::read_to_string(privacy.join("config.rs")).unwrap();
        assert!(config.contains("const TENANT_REF: &str = \"tenant-alpha\";"));
        assert!(!privacy.join("selection.rs").exists());
        let controller =
            std::fs::read_to_string(root.join("src/controllers/privacy_controller.rs")).unwrap();
        assert!(controller.contains(".layer(rullst::server::from_fn(deny_scope_hints))"));
        let page = std::fs::read_to_string(privacy.join("page.rs")).unwrap();
        assert!(page.contains("action=\"/privacy\""));
        for (name, source) in [
            ("config", &config),
            ("controller", &controller),
            ("page", &page),
        ] {
            for removed in [
                "TenantContext",
                "select_school",
                "selection::",
                "config::url",
                "FIXED_TENANT",
            ] {
                assert!(!source.contains(removed), "{name} still contains {removed}");
            }
        }
    }

    fn saas_project(root: &Path, rullst: &str) {
        std::fs::write(
            root.join("Cargo.toml"),
            format!("[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst = {rullst}\n"),
        )
        .unwrap();
        crate::blueprints::apply(
            crate::blueprints::SAAS_BLUEPRINT_ID,
            root,
            "consumer",
            "consumer",
            false,
            false,
            true,
            "Active Record",
            "Zero-Bundle HTMX",
        )
        .unwrap();
    }

    #[test]
    fn a_postgres_project_gets_the_postgres_consent_store() {
        let temporary = tempfile::tempdir().unwrap();
        let root = &temporary.path().canonicalize().unwrap();
        saas_project(
            root,
            "{ version = \"13\", features = [\"orm\", \"strict-postgres\"] }",
        );
        let edits = plan(
            root,
            None,
            "saas",
            "greeting-v1",
            3600,
            "tenant-alpha",
            None,
        )
        .unwrap()
        .edits;
        files::apply(&edits).unwrap();
        let config =
            std::fs::read_to_string(root.join("src/controllers/privacy/config.rs")).unwrap();
        assert!(
            config.contains("ConsentGate<PostgresConsentStore>"),
            "{config}"
        );
        assert!(config.contains("PostgresConsentStore::connect(database, 10_000)"));
        let init = std::fs::read_to_string(root.join("src/bin/privacy-init.rs")).unwrap();
        assert!(init.contains("PostgresConsentStore::initialize(database, 10_000)"));
        assert!(!init.contains("Sqlite"));
        let manifest = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
        assert!(manifest.contains("\"consent-postgres\""), "{manifest}");
        assert!(!manifest.contains("consent-sqlite"));
        let guide = std::fs::read_to_string(root.join("PRIVACY.md")).unwrap();
        assert!(guide.contains("PostgreSQL connection URL"));
        assert!(!guide.contains("__STORE"));
        // Another consumer still composes with the PostgreSQL consent feature.
        let mut parsed: DocumentMut = manifest.parse().unwrap();
        consumer_support::privacy_dependency(root, &mut parsed, None, &["challenge-tokens"])
            .unwrap();
        assert!(parsed.to_string().contains("\"consent-postgres\""));
    }

    #[test]
    fn the_consent_store_can_be_chosen_explicitly() {
        let temporary = tempfile::tempdir().unwrap();
        let root = &temporary.path().canonicalize().unwrap();
        saas_project(
            root,
            "{ version = \"13\", features = [\"orm\", \"strict-postgres\"] }",
        );
        let edits = plan(
            root,
            None,
            "saas",
            "greeting-v1",
            3600,
            "tenant-alpha",
            Some("sqlite"),
        )
        .unwrap()
        .edits;
        files::apply(&edits).unwrap();
        let config =
            std::fs::read_to_string(root.join("src/controllers/privacy/config.rs")).unwrap();
        assert!(
            config.contains("SqliteConsentStore::open(std::path::Path::new(&database), 10_000)")
        );
        let guide = std::fs::read_to_string(root.join("PRIVACY.md")).unwrap();
        assert!(guide.contains("shared-local SQLite profile"));
        let manifest = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
        assert!(manifest.contains("\"consent-sqlite\""));

        let other = tempfile::tempdir().unwrap();
        let other = &other.path().canonicalize().unwrap();
        saas_project(other, "\"13\"");
        assert_eq!(
            Store::select(
                None,
                &std::fs::read_to_string(other.join("Cargo.toml"))
                    .unwrap()
                    .parse()
                    .unwrap()
            )
            .unwrap(),
            Store::Sqlite
        );
        assert!(Store::select(Some("mysql"), &DocumentMut::new()).is_err());
        for (manifest, expected) in [
            (
                "[dependencies]\nrullst = { version = \"13\", features = [\"orm\", \"strict-sqlite\"] }\n",
                Store::Sqlite,
            ),
            (
                "[dependencies]\nrullst-orm = { version = \"13\", features = [\"strict-postgres\"] }\n",
                Store::Postgres,
            ),
            (
                "[dependencies]\nrullst = { version = \"13\", features = [\"strict-mysql\"] }\n",
                Store::Sqlite,
            ),
        ] {
            assert_eq!(
                Store::select(None, &manifest.parse().unwrap()).unwrap(),
                expected,
                "{manifest}"
            );
        }
        let planned = plan(
            other,
            None,
            "saas",
            "greeting-v1",
            3600,
            "tenant-alpha",
            Some("postgres"),
        );
        assert!(planned.unwrap().edits.iter().any(|_| true));
    }
}
