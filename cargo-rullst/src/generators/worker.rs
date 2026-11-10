// src/generators/worker.rs — Background Worker generator.

use crate::generators::{is_rullst_project, is_valid_rust_identifier};
use colored::*;
use std::fs;
use std::io::{Error as IoError, ErrorKind};
use std::path::Path;

const WORKER_START_HELPER: &str = r#"
/// Registers every generated processor and starts the queue worker.
///
/// Keep the returned handle alive for as long as the application should
/// continue processing jobs.
pub fn start_workers(
    mut worker: rullst::queue::Worker,
) -> Result<rullst::queue::WorkerHandle, rullst::queue::QueueError> {
    register_workers(&mut worker);
    let handle = worker.run()?;
    Ok(handle)
}
"#;

/// Returns the lifecycle-safe worker startup helper emitted by this generator.
///
/// This is public so scaffold smoke tests can compile the exact generated API.
#[doc(hidden)]
pub fn worker_start_helper() -> &'static str {
    WORKER_START_HELPER
}

/// Renders the exact job-handler module emitted by this generator.
#[doc(hidden)]
pub fn render_worker_source(job_name: &str) -> String {
    format!(
        r#"use rullst::queue::Worker;
use serde_json::Value;

/// Registers this worker's job processor.
pub fn register(worker: &mut Worker) {{
    worker.register("{job_name}", |payload: Value| async move {{
        // Never log the raw payload: jobs often carry e-mail addresses, reset
        // tokens and other personal data. Log only explicitly redacted fields.
        let field_count = payload.as_object().map_or(0, |fields| fields.len());
        println!("🚀 [Worker] Processing '{job_name}' ({{field_count}} payload field(s))");

        // Add background task logic here (for example, email or image processing).

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }});
}}
"#
    )
}

/// The queue backend the project's `rullst` features select.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QueueBackend {
    Sqlite,
    Redis,
}

/// What the project's `rullst` features do for the queue the startup lines
/// create.
#[derive(Debug, PartialEq, Eq)]
enum QueueSetup {
    /// `queue-sqlite`, `queue-redis` or `redis` is enabled.
    Enabled(QueueBackend),
    /// No backend was enabled: this manifest enables `queue-sqlite`.
    Added(String),
    /// No backend is enabled and the `rullst` dependency cannot be edited.
    Missing,
}

fn queue_setup(manifest: &str) -> QueueSetup {
    let features = toml::from_str::<toml::Value>(manifest)
        .ok()
        .and_then(|manifest| {
            manifest
                .get("dependencies")?
                .get("rullst")?
                .get("features")?
                .as_array()
                .cloned()
        })
        .unwrap_or_default();
    let enabled = |name: &str| {
        features
            .iter()
            .any(|feature| feature.as_str() == Some(name))
    };
    if enabled("queue-sqlite") {
        QueueSetup::Enabled(QueueBackend::Sqlite)
    } else if enabled("queue-redis") || enabled("redis") {
        QueueSetup::Enabled(QueueBackend::Redis)
    } else {
        crate::generators::chat::ensure_rullst_features(manifest, &["queue-sqlite"])
            .map_or(QueueSetup::Missing, QueueSetup::Added)
    }
}

/// The startup lines `make:worker` prints for `backend`.
fn startup_guidance(backend: QueueBackend) -> Vec<&'static str> {
    let queue: &[&str] = match backend {
        QueueBackend::Sqlite => &[
            "  1. Create the queue in the application's database, and the worker:",
            "     let database_url = rullst::config::project_setting(\"DATABASE_URL\")?",
            "         .ok_or(\"DATABASE_URL is not set\")?;",
            "     let queue = rullst::Queue::sqlite(database_url).await?;",
            "     (Jobs live in its rullst_jobs table. A PostgreSQL or MySQL application gives",
            "     the queue a SQLite URL of its own, on a writable volume.)",
        ],
        QueueBackend::Redis => &[
            "  1. Create the queue on the application's Redis, and the worker:",
            "     let redis_url = rullst::config::project_setting(\"REDIS_URL\")?",
            "         .ok_or(\"REDIS_URL is not set\")?;",
            "     let queue = rullst::Queue::redis(redis_url)?;",
        ],
    };
    let mut lines = vec!["How to start the background worker in src/main.rs:"];
    lines.extend_from_slice(queue);
    lines.extend([
        "     let worker = rullst::queue::Worker::new(&queue);",
        "  2. Register the processors and start the processing loop:",
        "     let _worker_handle = workers::start_workers(worker)?; // Keep this handle alive for the application lifetime",
    ]);
    lines
}

fn ensure_worker_dependencies(manifest: &str) -> Result<String, IoError> {
    let has_serde_json = manifest.lines().any(|line| {
        let declaration = line.split_once('#').map_or(line, |(value, _)| value).trim();
        declaration
            .split_once('=')
            .is_some_and(|(key, _)| key.trim().trim_matches(['\'', '"']) == "serde_json")
    });
    if has_serde_json {
        return Ok(manifest.to_string());
    }
    let Some(dependencies) = manifest.find("[dependencies]") else {
        return Err(IoError::new(
            ErrorKind::InvalidData,
            "Cargo.toml does not contain a [dependencies] table",
        ));
    };
    let mut updated = manifest.to_string();
    updated.insert_str(
        dependencies + "[dependencies]".len(),
        "\nserde_json = \"1\"",
    );
    Ok(updated)
}

pub fn worker_to_snake_case(s: &str) -> String {
    let mut base = s.to_string();
    if base.to_lowercase().ends_with("worker") {
        let len = base.len();
        base.truncate(len - 6);
    }

    let mut result = String::new();
    let mut prev_is_lower = false;
    for c in base.chars() {
        if c == '_' || c == '-' {
            result.push('_');
            prev_is_lower = false;
        } else if c.is_uppercase() {
            if prev_is_lower {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
            prev_is_lower = false;
        } else {
            result.push(c);
            prev_is_lower = true;
        }
    }

    result.push_str("_worker");

    // Clean duplicate underscores
    let mut clean_result = String::new();
    let mut prev_is_underscore = false;
    for c in result.chars() {
        if c == '_' {
            if !prev_is_underscore {
                clean_result.push(c);
            }
            prev_is_underscore = true;
        } else {
            clean_result.push(c);
            prev_is_underscore = false;
        }
    }
    clean_result.trim_matches('_').to_string()
}

pub fn create_new_worker(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        return Err(crate::ui::error_report::ProjectRequired.into());
    }

    let snake_name = worker_to_snake_case(name);
    if name.trim().is_empty() || !is_valid_rust_identifier(&snake_name) {
        return Err(IoError::new(
            ErrorKind::InvalidInput,
            "worker name must produce a valid non-keyword Rust module identifier",
        )
        .into());
    }
    let job_name = snake_name.strip_suffix("_worker").unwrap_or(&snake_name);

    let cargo_toml_path = Path::new("Cargo.toml");
    let cargo_toml = fs::read_to_string(cargo_toml_path)?;
    let setup = queue_setup(&cargo_toml);
    let (backend, with_queue) = match &setup {
        QueueSetup::Enabled(backend) => (*backend, cargo_toml.clone()),
        QueueSetup::Added(updated) => (QueueBackend::Sqlite, updated.clone()),
        QueueSetup::Missing => (QueueBackend::Sqlite, cargo_toml.clone()),
    };
    let updated_cargo_toml = ensure_worker_dependencies(&with_queue)?;
    if updated_cargo_toml != cargo_toml {
        fs::write(cargo_toml_path, updated_cargo_toml)?;
    }

    println!(
        "{}",
        format!("🛠️ Generating background worker Rullst: {}...", snake_name)
            .cyan()
            .bold()
    );

    let workers_dir = Path::new("src/workers");
    if !workers_dir.exists() {
        fs::create_dir_all(workers_dir)?;
    }

    let mod_path = workers_dir.join("mod.rs");
    if !mod_path.exists() {
        fs::write(&mod_path, "")?;
    }

    // Add module declaration to mod.rs
    let mut mod_content = fs::read_to_string(&mod_path)?;
    let mod_declaration = format!("pub mod {};", snake_name);
    if !mod_content.contains(&mod_declaration) {
        if !mod_content.is_empty() && !mod_content.ends_with('\n') {
            mod_content.push('\n');
        }
        mod_content.push_str(&mod_declaration);
        mod_content.push('\n');
    }

    // Ensure register_workers function exists in mod.rs
    if !mod_content.contains("pub fn register_workers") {
        mod_content.push_str("\npub fn register_workers(worker: &mut rullst::queue::Worker) {\n");
        mod_content.push_str(&format!("    {}::register(worker);\n", snake_name));
        mod_content.push_str("}\n");
    } else {
        // Inject registration inside register_workers
        let search_str = "pub fn register_workers(worker: &mut rullst::queue::Worker) {";
        if let Some(pos) = mod_content.find(search_str) {
            let insert_pos = pos + search_str.len() + 1;
            mod_content.insert_str(
                insert_pos,
                &format!("    {}::register(worker);\n", snake_name),
            );
        }
    }
    if !mod_content.contains("pub fn start_workers") {
        mod_content.push_str(worker_start_helper());
    }
    fs::write(&mod_path, mod_content)?;

    let worker_path = workers_dir.join(format!("{}.rs", snake_name));
    if worker_path.exists() {
        println!(
            "{}",
            format!(
                "⚠️ Warning: Worker '{}.rs' already exists. Skipping creation.",
                snake_name
            )
            .yellow()
        );
    } else {
        let template = render_worker_source(job_name);
        fs::write(&worker_path, template)?;
    }

    // Add module declaration to src/main.rs
    let main_path = Path::new("src/main.rs");
    if main_path.exists() {
        let mut main_content = fs::read_to_string(main_path)?;
        if !main_content.contains("pub mod workers;") && !main_content.contains("mod workers;") {
            if main_content.contains("pub mod controllers;") {
                main_content = main_content.replace(
                    "pub mod controllers;",
                    "pub mod controllers;\npub mod workers;",
                );
            } else if main_content.contains("pub mod models;") {
                main_content =
                    main_content.replace("pub mod models;", "pub mod models;\npub mod workers;");
            } else {
                main_content = format!("pub mod workers;\n{}", main_content);
            }
            fs::write(main_path, main_content)?;
            println!(
                "{}",
                "ℹ️ Automatically added 'pub mod workers;' to src/main.rs.".cyan()
            );
        }
    }

    println!(
        "{}",
        format!(
            "✨ Worker '{}' successfully created at '{}'!",
            snake_name,
            worker_path.display()
        )
        .green()
        .bold()
    );
    match setup {
        QueueSetup::Added(_) => println!(
            "{}",
            "ℹ️ Enabled the `queue-sqlite` feature of rullst in Cargo.toml (rullst::Queue::sqlite).".cyan()
        ),
        QueueSetup::Missing => println!(
            "{}",
            "⚠️ Enable the `queue-sqlite` feature of the rullst dependency in Cargo.toml: rullst::Queue::sqlite needs it.".yellow()
        ),
        QueueSetup::Enabled(_) => {}
    }
    for line in startup_guidance(backend) {
        println!("{}", line.cyan());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        QueueBackend, QueueSetup, WORKER_START_HELPER, ensure_worker_dependencies, queue_setup,
        render_worker_source, startup_guidance,
    };

    #[test]
    fn the_queue_feature_is_enabled_and_the_guidance_uses_the_app_database() {
        let plain = "[package]\nname = \"demo\"\n\n[dependencies]\nrullst = { version = \"13\", features = [\"orm\"] }\n";
        let QueueSetup::Added(updated) = queue_setup(plain) else {
            panic!("queue-sqlite is added");
        };
        assert!(
            updated.contains("features = [\"orm\", \"queue-sqlite\"]"),
            "{updated}"
        );
        assert_eq!(
            queue_setup(&updated),
            QueueSetup::Enabled(QueueBackend::Sqlite)
        );
        let redis = plain.replace("[\"orm\"]", "[\"orm\", \"queue-redis\"]");
        assert_eq!(
            queue_setup(&redis),
            QueueSetup::Enabled(QueueBackend::Redis)
        );
        assert_eq!(
            queue_setup("[package]\nname = \"x\"\n"),
            QueueSetup::Missing
        );

        let sqlite = startup_guidance(QueueBackend::Sqlite).join("\n");
        assert!(sqlite.contains("rullst::config::project_setting(\"DATABASE_URL\")?"));
        assert!(sqlite.contains("rullst::Queue::sqlite(database_url).await?"));
        assert!(!sqlite.contains("sqlite://rullst.db"), "{sqlite}");
        let redis = startup_guidance(QueueBackend::Redis).join("\n");
        assert!(redis.contains("rullst::Queue::redis(redis_url)?"));
        assert!(!redis.contains("Queue::sqlite"));
    }

    #[test]
    fn generated_startup_guidance_uses_the_fallible_worker_api() {
        let guidance = startup_guidance(QueueBackend::Sqlite).join("\n");
        assert!(guidance.contains("start_workers(worker)?"));
        assert!(guidance.contains("_worker_handle"));
        assert!(!guidance.contains("worker.run();"));
        assert!(WORKER_START_HELPER.contains("worker.run()?"));
        assert!(WORKER_START_HELPER.contains("Ok(handle)"));
        assert!(!WORKER_START_HELPER.contains("worker.run();"));
    }

    #[test]
    fn generated_handler_matches_the_fallible_queue_contract() {
        let source = render_worker_source("send_email");
        syn::parse_file(&source).expect("generated worker must parse");
        assert!(source.contains("Box<dyn std::error::Error + Send + Sync>"));
        assert!(!source.contains("worker.run("));
        assert!(!source.contains(".unwrap("));
        assert!(!source.contains(".expect("));
        assert!(!source.contains("panic!("));
    }

    #[test]
    fn generated_handler_never_logs_the_job_payload() {
        let source = render_worker_source("send_password_reset");
        assert!(
            !source.contains("{:?}"),
            "payload must not be Debug-formatted"
        );
        assert!(!source.contains("payload);"));
        assert!(source.contains("payload field(s)"));
    }

    #[test]
    fn worker_dependency_injection_is_idempotent() {
        let original = "[package]\nname = \"demo\"\n\n[dependencies]\nrullst = \"12\"\n";
        let updated = ensure_worker_dependencies(original).expect("worker dependency");
        let repeated = ensure_worker_dependencies(&updated).expect("idempotent dependency");
        assert_eq!(updated, repeated);
        assert_eq!(updated.matches("serde_json =").count(), 1);
    }
}
