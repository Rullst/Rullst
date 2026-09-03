#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn run(command: &mut Command, action: &str) -> Output {
    command
        .output()
        .unwrap_or_else(|error| panic!("could not {action}: {error}"))
}

fn assert_success(output: &Output, action: &str) {
    assert!(
        output.status.success(),
        "{action} failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn clean_generated_package(project: &Path, workspace: &Path, package_name: &str) {
    let cleaned = run(
        Command::new("cargo")
            .current_dir(project)
            .args(["clean", "--package", package_name])
            .env("CARGO_TARGET_DIR", workspace.join("target"))
            .env("CARGO_NET_OFFLINE", "true"),
        "clean generated chat package",
    );
    assert_success(&cleaned, "generated chat package cleanup");
}

fn install_workspace_lock(project: &Path, workspace: &Path) {
    fs::copy(workspace.join("Cargo.lock"), project.join("Cargo.lock"))
        .expect("copy workspace lockfile into generated chat project");
}

fn chat_migration(project: &Path) -> PathBuf {
    fs::read_dir(project.join("src/migrations"))
        .expect("migration directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.contains("create_chat_memory_tables"))
        })
        .expect("chat migration")
}

fn contract_source(database: &str) -> &'static str {
    if database == "turso" {
        r#"#[path = "../ai/mod.rs"]
mod ai;
#[path = "../models/mod.rs"]
mod models;

use ai::chat_service::StatefulChat;
use models::chat::ChatSession;
use rullst::ai::{providers::openai::OpenAiProvider, AiClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = rullst::orm::polyglot::TursoConfig::new("mock_chat", "")
        .with_offline_path("turso-development.db")?;
    rullst::orm::polyglot::TursoOrm::init(config).await?;
    let mut session = ChatSession::new("contract");
    session.save().await?;
    let chat = StatefulChat::new(
        session.clone(),
        AiClient::new(OpenAiProvider::new("mock_chat")),
    );
    chat.send("hello from the Turso contract").await?;
    let history = session.history().await?;
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].role, "user");
    assert_eq!(history[1].role, "assistant");
    Ok(())
}
"#
    } else {
        r#"#[path = "../ai/mod.rs"]
mod ai;
#[path = "../models/mod.rs"]
mod models;

use ai::chat_service::StatefulChat;
use models::chat::ChatSession;
use rullst::ai::{providers::openai::OpenAiProvider, AiClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    rullst::orm::Orm::init("sqlite://db.sqlite?mode=rwc").await?;
    let mut session = ChatSession::new("contract");
    session.save().await?;
    let chat = StatefulChat::new(
        session.clone(),
        AiClient::new(OpenAiProvider::new("mock_chat")),
    );
    chat.send("hello from the SQLx contract").await?;
    let history = session.history().await?;
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].role, "user");
    assert_eq!(history[1].role, "assistant");
    Ok(())
}
"#
    }
}

fn verify_backend(database: &str) {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let project = std::env::temp_dir().join(format!(
        "rullst-chat-scaffold-{database}-{}",
        rand::random::<u64>()
    ));
    let cli = env!("CARGO_BIN_EXE_rullst");
    let generated = run(
        Command::new(cli)
            .current_dir(workspace)
            .arg("new")
            .arg(&project)
            .args([
                "--default",
                "--api",
                "--database",
                database,
                "--skip-initial-migration",
            ]),
        "generate base project",
    );
    assert_success(&generated, "base project generation");

    let scaffolded = run(
        Command::new(cli)
            .current_dir(&project)
            .arg("make:chat-session"),
        "scaffold chat memory",
    );
    assert_success(&scaffolded, "chat-memory generation");

    let manifest = fs::read_to_string(project.join("Cargo.toml")).expect("generated manifest");
    let parsed: toml::Value = toml::from_str(&manifest).expect("valid generated manifest");
    let package_name = parsed["package"]["name"]
        .as_str()
        .expect("generated package name")
        .to_string();
    let features = parsed["dependencies"]["rullst"]["features"]
        .as_array()
        .expect("rullst features");
    for required in ["orm", "ai"] {
        assert_eq!(
            features
                .iter()
                .filter(|feature| feature.as_str() == Some(required))
                .count(),
            1,
            "feature {required} must be enabled exactly once"
        );
    }

    let model = fs::read_to_string(project.join("src/models/chat.rs")).expect("chat models");
    let service = fs::read_to_string(project.join("src/ai/chat_service.rs")).expect("chat service");
    let migration_path = chat_migration(&project);
    let migration = fs::read_to_string(&migration_path).expect("chat migration");
    assert!(model.contains("newest 100 messages"));
    assert!(service.contains("InvalidHistoryRole"));
    assert!(!service.contains("let _ = user_message.save"));
    assert!(!service.contains("let _ = assistant_message.save"));
    assert!(migration.contains("chat_sessions"));
    assert!(migration.contains("chat_messages"));
    assert_eq!(model.contains("backend = \"turso\""), database == "turso");
    assert!(
        fs::read_to_string(project.join("src/models/mod.rs"))
            .expect("models registry")
            .contains("pub mod chat;")
    );
    assert!(
        fs::read_to_string(project.join("src/ai/mod.rs"))
            .expect("AI registry")
            .contains("pub mod chat_service;")
    );
    fs::create_dir_all(project.join("src/bin")).expect("contract bin directory");
    fs::write(
        project.join("src/bin/chat_contract.rs"),
        contract_source(database),
    )
    .expect("write generated chat runtime contract");
    install_workspace_lock(&project, workspace);

    let checked = run(
        Command::new("cargo")
            .current_dir(&project)
            .args(["clippy", "--all-targets", "--", "-D", "warnings"])
            .env("CARGO_TARGET_DIR", workspace.join("target"))
            .env("CARGO_NET_OFFLINE", "true"),
        "Clippy generated chat project",
    );
    assert_success(&checked, "generated chat project Clippy");

    let migrated = run(
        Command::new("cargo")
            .current_dir(&project)
            .args(["run", "--quiet", "--bin", &package_name, "--", "db:migrate"])
            .env("CARGO_TARGET_DIR", workspace.join("target"))
            .env("CARGO_NET_OFFLINE", "true"),
        "run generated chat migrations",
    );
    assert_success(&migrated, "generated chat migrations");
    assert!(
        String::from_utf8_lossy(&migrated.stdout).contains("create_chat_memory_tables")
            || (database == "turso"
                && String::from_utf8_lossy(&migrated.stdout)
                    .contains("Applied 2 Turso migration(s)"))
    );

    let runtime = run(
        Command::new("cargo")
            .current_dir(&project)
            .args(["run", "--quiet", "--bin", "chat_contract"])
            .env("CARGO_TARGET_DIR", workspace.join("target"))
            .env("CARGO_NET_OFFLINE", "true"),
        "run generated chat-memory runtime contract",
    );
    assert_success(&runtime, "generated chat-memory runtime contract");

    let model_before = fs::read_to_string(project.join("src/models/chat.rs")).expect("model");
    let duplicate = run(
        Command::new(cli)
            .current_dir(&project)
            .arg("make:chat-session"),
        "rerun chat-memory generator",
    );
    assert!(!duplicate.status.success(), "rerun must fail closed");
    assert_eq!(
        fs::read_to_string(project.join("src/models/chat.rs")).expect("preserved model"),
        model_before
    );
    assert_eq!(
        fs::read_dir(project.join("src/migrations"))
            .expect("migration directory")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains("chat_memory"))
            .count(),
        1
    );

    clean_generated_package(&project, workspace, &package_name);
    fs::remove_dir_all(&project).expect("remove generated project");
}

#[test]
fn chat_memory_scaffold_compiles_migrates_and_fails_closed_on_sqlx_and_turso() {
    for database in ["sqlite", "turso"] {
        verify_backend(database);
    }
}
