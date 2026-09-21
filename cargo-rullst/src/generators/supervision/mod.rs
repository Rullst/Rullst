//! Explicit unpublished full-LMS/SQLite consumer. All edits are planned first.
use super::{consumer_files as files, consumer_support};
use clap::{Arg, ArgMatches, Command};
use files::Edit;
use std::{
    io,
    path::{Path, PathBuf},
};
use toml_edit::{Array, DocumentMut, InlineTable, Item, Value};
mod routing;

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

pub(crate) fn command() -> Command {
    Command::new("make:supervision")
        .about(
            "Add explicit transparent supervision to the full SQLite LMS (unpublished v13 preview)",
        )
        .arg(
            Arg::new("browser-observations")
                .long("browser-observations")
                .default_value("visibility")
                .help(
                    "Disclosed comma-separated categories: visibility,focus,clipboard,fullscreen",
                ),
        )
        .arg(
            Arg::new("supervision-source")
                .long("supervision-source")
                .required(true)
                .value_parser(clap::value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("policy-version")
                .long("policy-version")
                .required(true),
        )
        .arg(
            Arg::new("notice-version")
                .long("notice-version")
                .required(true),
        )
        .arg(
            Arg::new("retention-seconds")
                .long("retention-seconds")
                .required(true)
                .value_parser(clap::value_parser!(u32).range(3600..=604800)),
        )
        .arg(
            Arg::new("session-seconds")
                .long("session-seconds")
                .required(true)
                .value_parser(clap::value_parser!(u32).range(1..=28800)),
        )
}

pub(crate) fn run(args: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::current_dir()?;
    let source = args
        .get_one::<PathBuf>("supervision-source")
        .ok_or_else(|| invalid("missing source"))?;
    let policy = args
        .get_one::<String>("policy-version")
        .ok_or_else(|| invalid("missing policy"))?;
    let notice = args
        .get_one::<String>("notice-version")
        .ok_or_else(|| invalid("missing notice"))?;
    let retention = *args
        .get_one::<u32>("retention-seconds")
        .ok_or_else(|| invalid("missing retention"))?;
    let lifetime = *args
        .get_one::<u32>("session-seconds")
        .ok_or_else(|| invalid("missing lifetime"))?;
    let collection = browser_collection(
        args.get_one::<String>("browser-observations")
            .map(String::as_str)
            .unwrap_or("visibility"),
    )?;
    files::apply(&plan(
        &root, source, policy, notice, retention, lifetime, collection,
    )?)?;
    println!(
        "Supervision installed. Read SUPERVISION.md: initialize the private store and form key before startup; independently verify authority before administrative provisioning."
    );
    Ok(())
}

fn plan(
    root: &Path,
    source: &Path,
    policy: &str,
    notice: &str,
    retention: u32,
    lifetime: u32,
    collection: u16,
) -> Result<Vec<Edit>, Box<dyn std::error::Error>> {
    for value in [policy, notice] {
        if value.is_empty()
            || value.len() > 128
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':'))
        {
            return Err(
                invalid("policy and notice must be bounded opaque ASCII references").into(),
            );
        }
    }
    if !(3600..=604800).contains(&retention) || !(1..=28800).contains(&lifetime) {
        return Err(invalid("invalid retention or lifetime").into());
    }
    consumer_support::recognized_auth(root, "lms")?;
    let source = source.canonicalize()?;
    let package: toml::Value = toml::from_str(&files::read(&source.join("Cargo.toml"))?)?;
    if package
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(toml::Value::as_str)
        != Some("rullst-supervision")
        || package
            .get("package")
            .and_then(|p| p.get("version"))
            .and_then(toml::Value::as_str)
            != Some(env!("CARGO_PKG_VERSION"))
        || !source.join("src/sqlite/mod.rs").is_file()
    {
        return Err(invalid("source must be the supervision package matching this CLI").into());
    }
    let manifest_path = root.join("Cargo.toml");
    let original = files::read(&manifest_path)?;
    let mut manifest: DocumentMut = original.parse()?;
    let dependency = manifest
        .get("dependencies")
        .and_then(|d| d.get("rullst"))
        .ok_or_else(|| invalid("missing Rullst dependency"))?;
    let features = dependency
        .get("features")
        .and_then(Item::as_array)
        .ok_or_else(|| invalid("explicit Rullst features required"))?;
    if dependency.get("default-features").and_then(Item::as_bool) != Some(false)
        || !features.iter().any(|f| f.as_str() == Some("strict-sqlite"))
        || features.iter().any(|f| {
            matches!(
                f.as_str(),
                Some("strict-postgres" | "strict-mysql" | "turso")
            )
        })
    {
        return Err(invalid(
            "supervision currently requires the explicit strict SQLite LMS profile",
        )
        .into());
    }
    if manifest["dependencies"].get("rullst-supervision").is_some()
        || manifest["package"].get("autobins").is_some()
        || manifest.get("bin").is_some()
    {
        return Err(invalid(
            "existing supervision or custom binary inventory requires manual integration",
        )
        .into());
    }
    let mut dependency = InlineTable::new();
    dependency.insert(
        "path",
        Value::from(
            source
                .to_str()
                .ok_or_else(|| invalid("source path must be UTF-8"))?,
        ),
    );
    dependency.insert(
        "version",
        Value::from(format!("={}", env!("CARGO_PKG_VERSION"))),
    );
    dependency.insert("default-features", Value::from(false));
    dependency.insert(
        "features",
        Value::Array(["sqlite"].into_iter().collect::<Array>()),
    );
    manifest["dependencies"]["rullst-supervision"] = Item::Value(Value::InlineTable(dependency));
    for (name, version) in [("ring", "0.17"), ("hex", "0.4"), ("chrono", "0.4")] {
        if manifest["dependencies"].get(name).is_none() {
            manifest["dependencies"][name] = toml_edit::value(version);
        }
    }
    if manifest["package"].get("default-run").is_none() {
        let name = manifest["package"]["name"]
            .as_str()
            .ok_or_else(|| invalid("missing package name"))?
            .to_owned();
        manifest["package"]["default-run"] = toml_edit::value(name);
    }
    let baseline =
        crate::blueprints::lms::file_manifest("unused", false, "Active Record", "Zero-Bundle HTMX");
    let learning_path = root.join("src/services/learning_service.rs");
    let learning = files::read(&learning_path)?;
    let expected = baseline
        .iter()
        .find(|(path, _)| *path == "src/services/learning_service.rs")
        .ok_or_else(|| invalid("missing baseline learning service"))?;
    if !consumer_support::equivalent(&learning, &expected.1)? {
        return Err(invalid(
            "custom learning authorization requires manual supervision integration",
        )
        .into());
    }
    // Normalize formatting only after checking the complete recognized service.
    let normalized = consumer_support::formatted(&expected.1)?;
    let boundary = "    Ok(lesson)";
    if normalized.matches(boundary).count() != 1 {
        return Err(invalid("ambiguous lesson authorization return").into());
    }
    let updated_learning = normalized.replacen(boundary, "    crate::services::supervision_service::authorize_learning(user_id, context, lesson.course_id).await.map_err(|_| LearningError::Forbidden)?;\n    Ok(lesson)", 1);
    let main_path = root.join("src/main.rs");
    let main = files::read(&main_path)?;
    let startup = "rullst::artisan!(crate::migrations::get_migrations());";
    if main.matches(startup).count() != 1 {
        return Err(invalid("unrecognized full LMS startup").into());
    }
    let mut updated_main = main.replacen(
        startup,
        &format!("{startup}\n    services::supervision_service::initialize().await?;"),
        1,
    );
    let mut edits = vec![
        Edit::replace(manifest_path, original, manifest.to_string()),
        Edit::replace(
            learning_path,
            learning,
            consumer_support::formatted(&updated_learning)?,
        ),
    ];
    let lib = root.join("src/lib.rs");
    if lib.exists() {
        let old = files::read(&lib)?;
        let new = routing::mount(&old)?;
        edits.push(Edit::replace(lib, old, new));
    } else {
        updated_main = routing::mount(&updated_main)?;
    }
    edits.push(Edit::replace(
        main_path,
        main,
        consumer_support::formatted(&updated_main)?,
    ));
    for (path, module) in [
        ("src/controllers/mod.rs", "supervision_controller"),
        ("src/services/mod.rs", "supervision_service"),
    ] {
        let path = root.join(path);
        let old = files::read(&path)?;
        let new = consumer_support::formatted(&format!("{old}\npub mod {module};\n"))?;
        edits.push(Edit::replace(path, old, new));
    }
    for (path, template) in [
        (
            "src/services/supervision_service.rs",
            include_str!("service.rs.template"),
        ),
        (
            "src/services/supervision/config.rs",
            include_str!("config.rs.template"),
        ),
        (
            "src/controllers/supervision_controller.rs",
            include_str!("controller.rs.template"),
        ),
        (
            "src/controllers/supervision/selection.rs",
            include_str!("selection.rs.template"),
        ),
        (
            "src/controllers/supervision/exam.rs",
            include_str!("exam.rs.template"),
        ),
        (
            "src/controllers/supervision/parental.rs",
            include_str!("parental.rs.template"),
        ),
        (
            "src/controllers/supervision/page.rs",
            include_str!("page.rs.template"),
        ),
        (
            "src/controllers/supervision/proof.rs",
            include_str!("proof.rs.template"),
        ),
        (
            "src/bin/supervision-admin.rs",
            include_str!("admin.rs.template"),
        ),
    ] {
        let text = template
            .replace("__POLICY__", &format!("{policy:?}"))
            .replace("__NOTICE__", &format!("{notice:?}"))
            .replace("__RETENTION__", &retention.to_string())
            .replace("__COLLECTION__", &collection.to_string())
            .replace("__LIFETIME__", &lifetime.to_string());
        edits.push(Edit::create(
            root.join(path),
            consumer_support::formatted(&text)?,
        )?);
    }
    edits.push(Edit::create(
        root.join("src/controllers/supervision/visibility.js"),
        include_str!("visibility.js").to_owned(),
    )?);
    edits.push(Edit::create(
        root.join("SUPERVISION.md"),
        include_str!("README.md.template").to_owned(),
    )?);
    Ok(edits)
}

fn browser_collection(value: &str) -> Result<u16, io::Error> {
    if value.is_empty() || value.len() > 64 {
        return Err(invalid("select supported browser observation categories"));
    }
    let mut bits = 0;
    for name in value.split(',') {
        let bit = match name {
            "visibility" => 1,
            "focus" => 2,
            "clipboard" => 4,
            "fullscreen" => 8,
            _ => return Err(invalid("unsupported browser observation category")),
        };
        if bits & bit != 0 {
            return Err(invalid("duplicate browser observation category"));
        }
        bits |= bit;
    }
    Ok(bits)
}
