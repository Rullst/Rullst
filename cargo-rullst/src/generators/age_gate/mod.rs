//! Opt-in v13 declaration consumer for the recognized authenticated SaaS starter.
use clap::{Arg, ArgMatches, Command};
use std::{
    fs, io,
    path::{Path, PathBuf},
};
use toml_edit::{Array, DocumentMut, InlineTable, Item, Value};

mod routing;
mod writes;
use writes::Edit;

pub(crate) fn command() -> Command {
    Command::new("make:age-gate")
        .about("Add an explicit first-party age declaration to the SaaS dashboard (v13 preview)")
        .arg(
            Arg::new("privacy-source")
                .long("privacy-source")
                .required(true)
                .value_parser(clap::value_parser!(PathBuf))
                .help("Path to the unpublished v13 rullst-privacy package"),
        )
        .arg(
            Arg::new("minimum-age")
                .long("minimum-age")
                .required(true)
                .value_parser(clap::value_parser!(u8).range(1..=120)),
        )
        .arg(
            Arg::new("policy-version")
                .long("policy-version")
                .required(true),
        )
        .arg(
            Arg::new("tenant-ref")
                .long("tenant-ref")
                .required(true)
                .help("Server-owned opaque reference for this single-tenant SaaS deployment"),
        )
        .arg(
            Arg::new("replay-store")
                .long("replay-store")
                .required(true)
                .value_parser(["sqlite", "postgres"]),
        )
}

pub(crate) fn run(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::current_dir()?;
    let source = matches
        .get_one::<PathBuf>("privacy-source")
        .ok_or_else(|| invalid("privacy source is required"))?;
    let minimum = *matches
        .get_one::<u8>("minimum-age")
        .ok_or_else(|| invalid("minimum age is required"))?;
    let version = argument(matches, "policy-version")?;
    let tenant = argument(matches, "tenant-ref")?;
    let profile = argument(matches, "replay-store")?;
    let edits = plan(&root, source, minimum, version, tenant, profile)?;
    writes::apply(&edits)?;
    println!(
        "Age declaration installed for /dashboard. Configure the required private key and replay store in AGE_GATE.md before starting the app. Answers remain declared, not verified."
    );
    Ok(())
}

fn argument<'a>(matches: &'a ArgMatches, name: &str) -> Result<&'a str, io::Error> {
    matches
        .get_one::<String>(name)
        .map(String::as_str)
        .ok_or_else(|| invalid("required age-gate argument is missing"))
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn plan(
    root: &Path,
    source: &Path,
    minimum: u8,
    version: &str,
    tenant: &str,
    profile: &str,
) -> Result<Vec<Edit>, Box<dyn std::error::Error>> {
    for token in [version, tenant] {
        if token.is_empty()
            || token.len() > 128
            || !token
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b':'))
        {
            return Err(invalid(
                "policy version and tenant reference must be bounded opaque ASCII tokens",
            )
            .into());
        }
    }
    let source = source.canonicalize()?;
    let privacy: toml::Value = toml::from_str(&fs::read_to_string(source.join("Cargo.toml"))?)?;
    let package = privacy
        .get("package")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| invalid("privacy source has no package table"))?;
    if package.get("name").and_then(toml::Value::as_str) != Some("rullst-privacy")
        || package.get("version").and_then(toml::Value::as_str) != Some("13.0.0-alpha.1")
        || package.get("publish").and_then(toml::Value::as_bool) != Some(false)
        || !source
            .join("src/age_assurance/challenge_tokens.rs")
            .is_file()
    {
        return Err(invalid(
            "source must be the unpublished v13 privacy package with challenge transport",
        )
        .into());
    }
    let manifest_path = root.join("Cargo.toml");
    let manifest = writes::read(&manifest_path)?;
    let mut parsed = manifest.parse::<DocumentMut>()?;
    if parsed
        .get("dependencies")
        .and_then(|deps| deps.get("rullst"))
        .is_none()
    {
        return Err(invalid("run make:age-gate inside a generated SaaS project").into());
    }
    if parsed["dependencies"].get("rullst-privacy").is_some() {
        return Err(
            invalid("an existing privacy dependency requires a manual integration review").into(),
        );
    }
    let baseline = crate::blueprints::saas::file_manifest(
        "unused",
        false,
        "Active Record",
        "Zero-Bundle HTMX",
    );
    for name in [
        "src/middlewares/auth_middleware.rs",
        "src/controllers/auth_controller.rs",
    ] {
        let expected = baseline
            .iter()
            .find(|(path, _)| *path == name)
            .ok_or_else(|| invalid("SaaS template is missing"))?;
        if !routing::equivalent(&writes::read(&root.join(name))?, &expected.1)? {
            return Err(invalid("this generator requires the recognized SaaS authentication controller and middleware; review custom authentication separately").into());
        }
    }
    let mut edits = Vec::new();
    let main_path = root.join("src/main.rs");
    let main = writes::read(&main_path)?;
    let startup = "rullst::artisan!(crate::migrations::get_migrations());";
    if main.matches(startup).count() != 1 {
        return Err(invalid("unrecognized SaaS startup shape").into());
    }
    let mut main_updated = main.replacen(
        startup,
        &format!("{startup}\n    controllers::age_controller::initialize().await?;"),
        1,
    );
    let lib_path = root.join("src/lib.rs");
    let (router_path, router) = if lib_path.exists() {
        (lib_path.clone(), writes::read(&lib_path)?)
    } else {
        (main_path.clone(), main_updated.clone())
    };
    let updated_router = routing::protect(&router)?;
    if router_path == main_path {
        main_updated = updated_router;
    } else {
        edits.push(Edit::replace(router_path, router, updated_router));
    }
    syn::parse_file(&main_updated)?;
    edits.push(Edit::replace(main_path, main, main_updated));

    let registry = root.join("src/controllers/mod.rs");
    let previous = writes::read(&registry)?;
    let updated = format!("{previous}\npub mod age_controller;\n");
    syn::parse_file(&updated)?;
    edits.push(Edit::replace(registry, previous, updated));

    let (store_type, store_open) = match profile {
        "sqlite" => (
            "SqliteReplayStore",
            "SqliteReplayStore::open(database, 10_000).await?",
        ),
        "postgres" => (
            "PostgresReplayStore",
            "PostgresReplayStore::connect(database, 10_000).await?",
        ),
        _ => return Err(invalid("unsupported age replay store").into()),
    };
    let config = include_str!("config.rs.template")
        .replace("__STORE_TYPE__", store_type)
        .replace("__STORE_OPEN__", store_open)
        .replace("__POLICY_VERSION__", &format!("{version:?}"))
        .replace("__TENANT_REF__", &format!("{tenant:?}"))
        .replace("__MINIMUM_AGE__", &minimum.to_string());
    for (name, content) in [
        (
            "src/controllers/age_controller.rs",
            include_str!("controller.rs.template").to_owned(),
        ),
        ("src/controllers/age_gate/config.rs", config),
        (
            "src/controllers/age_gate/page.rs",
            include_str!("page.rs.template").to_owned(),
        ),
    ] {
        syn::parse_file(&content)?;
        edits.push(Edit::create(root.join(name), content)?);
    }
    let mut dependency = InlineTable::new();
    dependency.insert(
        "path",
        Value::from(
            source
                .to_str()
                .ok_or_else(|| invalid("privacy source path must be UTF-8"))?,
        ),
    );
    dependency.insert("version", Value::from("=13.0.0-alpha.1"));
    dependency.insert("default-features", Value::from(false));
    let features: Array = ["challenge-tokens", profile].into_iter().collect();
    dependency.insert("features", Value::Array(features));
    parsed["dependencies"]["rullst-privacy"] = Item::Value(Value::InlineTable(dependency));
    for (name, version) in [("ring", "0.17"), ("hex", "0.4")] {
        if parsed["dependencies"].get(name).is_none() {
            parsed["dependencies"][name] = toml_edit::value(version);
        }
    }
    edits.push(Edit::replace(manifest_path, manifest, parsed.to_string()));
    edits.push(Edit::create(
        root.join("AGE_GATE.md"),
        include_str!("README.md.template").replace("__PROFILE__", profile),
    )?);
    Ok(edits)
}
