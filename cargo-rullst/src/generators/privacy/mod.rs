//! Explicit preview consumer: authenticated choices and an own-account export.
use super::{consumer_files as files, consumer_support};
use clap::{Arg, ArgMatches, Command};
use files::Edit;
use std::{
    io,
    path::{Path, PathBuf},
};
use toml_edit::DocumentMut;

mod routing;

pub(crate) fn command() -> Command {
    Command::new("make:privacy")
        .about("Add authenticated privacy choices and an own-account profile export (v13 preview)")
        .arg(
            Arg::new("blueprint")
                .long("blueprint")
                .default_value("saas")
                .value_parser(["saas", "lms"]),
        )
        .arg(
            Arg::new("privacy-source")
                .long("privacy-source")
                .required(true)
                .value_parser(clap::value_parser!(PathBuf)),
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
                .required_if_eq("blueprint", "saas"),
        )
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

pub(crate) fn run(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::current_dir()?;
    let source = matches
        .get_one::<PathBuf>("privacy-source")
        .ok_or_else(|| invalid("privacy source is required"))?;
    let consumer = matches
        .get_one::<String>("blueprint")
        .ok_or_else(|| invalid("blueprint is required"))?;
    let version = matches
        .get_one::<String>("purpose-version")
        .ok_or_else(|| invalid("purpose version is required"))?;
    let lifetime = *matches
        .get_one::<u32>("validity-seconds")
        .ok_or_else(|| invalid("grant validity is required"))?;
    let tenant = matches.get_one::<String>("tenant-ref").map(String::as_str);
    let edits = plan(&root, source, consumer, version, lifetime, tenant)?;
    files::apply(&edits)?;
    println!(
        "Privacy choices installed at /privacy, with an own-account profile export. Read PRIVACY.md, initialize the private consent store once, and configure the independent form key before enabling choices."
    );
    Ok(())
}

fn plan(
    root: &Path,
    source: &Path,
    consumer: &str,
    version: &str,
    lifetime: u32,
    tenant: Option<&str>,
) -> Result<Vec<Edit>, Box<dyn std::error::Error>> {
    if !matches!((consumer, tenant), ("saas", Some(_)) | ("lms", None)) {
        return Err(invalid("SaaS requires --tenant-ref; LMS requires authenticated school membership and rejects a fixed tenant").into());
    }
    for value in std::iter::once(version).chain(tenant) {
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
    consumer_support::recognized_auth(root, consumer)?;
    let manifest_path = root.join("Cargo.toml");
    let original = files::read(&manifest_path)?;
    let mut manifest: DocumentMut = original.parse()?;
    consumer_support::privacy_dependency(root, &mut manifest, &source, &["consent-sqlite"])?;
    for (name, version) in [("ring", "0.17"), ("hex", "0.4")] {
        if manifest["dependencies"].get(name).is_none() {
            manifest["dependencies"][name] = toml_edit::value(version);
        }
    }
    // A second binary must not make ordinary `cargo run` ambiguous.
    if manifest["package"].get("default-run").is_none() {
        let name = manifest["package"]["name"]
            .as_str()
            .ok_or_else(|| invalid("project package name is missing"))?
            .to_owned();
        manifest["package"]["default-run"] = toml_edit::value(name);
    }
    if manifest["package"].get("autobins").is_some() || manifest.get("bin").is_some() {
        return Err(invalid(
            "custom binary inventory requires manual privacy initialization integration",
        )
        .into());
    }
    let router_path = if root.join("src/lib.rs").exists() {
        root.join("src/lib.rs")
    } else {
        root.join("src/main.rs")
    };
    let router = files::read(&router_path)?;
    let updated = routing::mount(&router)?;
    let registry = root.join("src/controllers/mod.rs");
    let old_registry = files::read(&registry)?;
    let new_registry = format!("{old_registry}\npub mod privacy_controller;\n");
    syn::parse_file(&new_registry)?;
    let mut edits = vec![
        Edit::replace(manifest_path, original, manifest.to_string()),
        Edit::replace(router_path, router, updated),
        Edit::replace(registry, old_registry, new_registry),
    ];
    let config = include_str!("config.rs.template")
        .replace("__NOTICE_VERSION__", &format!("{version:?}"))
        .replace("__VALIDITY_SECONDS__", &lifetime.to_string())
        .replace(
            "__TENANT_CONFIG__",
            &tenant
                .map(|tenant| format!("Some({tenant:?})"))
                .unwrap_or_else(|| "None".to_owned()),
        );
    let selector = include_str!("../age_gate/selection.rs.template")
        .replace("Used only on the LMS dashboard", "Used only on the privacy consumer routes")
        .replace("if let Some(school) = selection.school {", "if super::config::FIXED_TENANT.is_some() && (selection.school.is_some() || request.headers().contains_key(\"x-school-id\")) { return super::denied(); }\n    if let Some(school) = selection.school {");
    for (path, source) in [
        (
            "src/controllers/privacy_controller.rs",
            include_str!("controller.rs.template").to_owned(),
        ),
        ("src/controllers/privacy/config.rs", config),
        (
            "src/controllers/privacy/page.rs",
            include_str!("page.rs.template").to_owned(),
        ),
        (
            "src/controllers/privacy/proof.rs",
            include_str!("proof.rs.template").to_owned(),
        ),
        (
            "src/controllers/privacy/profile.rs",
            include_str!("profile.rs.template").to_owned(),
        ),
        ("src/controllers/privacy/selection.rs", selector),
        (
            "src/bin/privacy-init.rs",
            include_str!("init.rs.template").to_owned(),
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
        include_str!("README.md.template").to_owned(),
    )?);
    Ok(edits)
}
