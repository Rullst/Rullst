//! Opt-in v13 declaration consumer for the recognized authenticated SaaS starter.
use super::{consumer_files as writes, consumer_support};
use clap::{Arg, ArgMatches, Command};
use std::{
    io,
    path::{Path, PathBuf},
};
use toml_edit::DocumentMut;

mod lms_routing;
mod routing;
use writes::Edit;

pub(crate) fn command() -> Command {
    Command::new("make:age-gate")
        .about("Add an explicit first-party age declaration to a SaaS/LMS dashboard (v13 preview)")
        .arg(Arg::new("blueprint").long("blueprint").default_value("saas").value_parser(["saas", "lms"]))
        .arg(
            Arg::new("privacy-source")
                .long("privacy-source")
                .value_parser(clap::value_parser!(PathBuf))
                .help("Explicit local privacy source matching this CLI; defaults to the matching registry version"),
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
                .required_if_eq("blueprint", "saas")
                .help("Server-owned SaaS tenant; the LMS profile uses authenticated school membership"),
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
        .map(PathBuf::as_path);
    let minimum = *matches
        .get_one::<u8>("minimum-age")
        .ok_or_else(|| invalid("minimum age is required"))?;
    let version = argument(matches, "policy-version")?;
    let tenant = matches.get_one::<String>("tenant-ref").map(String::as_str);
    let consumer = argument(matches, "blueprint")?;
    let profile = argument(matches, "replay-store")?;
    let edits = plan(&root, source, minimum, version, tenant, profile, consumer)?;
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
    source: Option<&Path>,
    minimum: u8,
    version: &str,
    tenant: Option<&str>,
    profile: &str,
    consumer: &str,
) -> Result<Vec<Edit>, Box<dyn std::error::Error>> {
    if !matches!((consumer, tenant), ("saas", Some(_)) | ("lms", None)) {
        return Err(invalid("SaaS requires --tenant-ref; LMS resolves its school from authenticated membership and rejects a fixed tenant").into());
    }
    for token in std::iter::once(version).chain(tenant) {
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
    let source = consumer_support::privacy_source(source, "src/age_assurance/challenge_tokens.rs")?;
    let manifest_path = root.join("Cargo.toml");
    let manifest = writes::read(&manifest_path)?;
    let mut parsed = manifest.parse::<DocumentMut>()?;
    consumer_support::recognized_auth(root, consumer)?;
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
    let updated_router = if consumer == "saas" {
        routing::protect(&router)?
    } else {
        lms_routing::protect(&router)?
    };
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
        .replace(
            "__TENANT_CONFIG__",
            &tenant
                .map(|tenant| format!("Some({tenant:?}.to_owned())"))
                .unwrap_or_else(|| "None".to_owned()),
        )
        .replace(
            "__AUDIENCE__",
            if consumer == "saas" {
                "\"saas-dashboard\""
            } else {
                "\"lms-dashboard\""
            },
        )
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
        (
            "src/controllers/age_gate/selection.rs",
            include_str!("selection.rs.template").to_owned(),
        ),
    ] {
        syn::parse_file(&content)?;
        edits.push(Edit::create(
            root.join(name),
            consumer_support::formatted(&content)?,
        )?);
    }
    consumer_support::privacy_dependency(
        root,
        &mut parsed,
        source.as_deref(),
        &["challenge-tokens", profile],
    )?;
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
