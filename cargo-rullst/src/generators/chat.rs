//! Conversational-memory scaffold for relational and Turso-primary projects.

use crate::generators::migration::regenerate_migrations_mod;
use crate::generators::{
    ProjectOrmBackend, is_rullst_project, project_orm_backend, register_mod_ast,
};
use std::fs;
use std::io::{Error as IoError, ErrorKind};
use std::path::{Path, PathBuf};
use toml_edit::{Array, DocumentMut, InlineTable, Item, Value};

#[cfg(test)]
mod tests;

const MODEL_PATH: &str = "src/models/chat.rs";
const SERVICE_PATH: &str = "src/ai/chat_service.rs";

/// Scaffolds durable, bounded chat memory and a reversible schema migration.
pub fn scaffold_chat_session() -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        return Err(IoError::new(
            ErrorKind::InvalidInput,
            "make:chat-session must be run inside a Rullst project",
        )
        .into());
    }

    reject_existing_outputs()?;
    let backend = project_orm_backend();
    let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S");
    let migration_stem = format!("m{timestamp}_create_chat_memory_tables");
    let migration_path = Path::new("src/migrations").join(format!("{migration_stem}.rs"));
    if migration_path.exists() {
        return Err(IoError::new(
            ErrorKind::AlreadyExists,
            format!("refusing to overwrite {}", migration_path.display()),
        )
        .into());
    }

    let manifest_path = Path::new("Cargo.toml");
    let manifest = fs::read_to_string(manifest_path)?;
    let updated_manifest = ensure_rullst_features(&manifest, &["orm", "ai"])?;

    fs::create_dir_all("src/models")?;
    fs::create_dir_all("src/ai")?;
    fs::create_dir_all("src/migrations")?;
    fs::write(MODEL_PATH, render_chat_models(backend))?;
    fs::write(SERVICE_PATH, render_chat_service(backend))?;
    fs::write(
        &migration_path,
        render_chat_migration(&migration_stem, backend),
    )?;
    fs::write(manifest_path, updated_manifest)?;

    register_mod_ast(Path::new("src/models/mod.rs"), "chat")?;
    register_mod_ast(Path::new("src/ai/mod.rs"), "chat_service")?;
    register_root_modules()?;
    regenerate_migrations_mod()?;

    println!("✅ Durable ChatSession and ChatMessage memory scaffolded.");
    println!("👉 Generated {MODEL_PATH}, {SERVICE_PATH}, and a reversible migration.");
    Ok(())
}

fn reject_existing_outputs() -> Result<(), IoError> {
    let collisions = [MODEL_PATH, SERVICE_PATH]
        .into_iter()
        .filter(|path| Path::new(path).exists())
        .collect::<Vec<_>>();
    if collisions.is_empty() {
        return Ok(());
    }
    Err(IoError::new(
        ErrorKind::AlreadyExists,
        format!(
            "refusing to overwrite existing chat scaffold: {}",
            collisions.join(", ")
        ),
    ))
}

fn register_root_modules() -> Result<(), Box<dyn std::error::Error>> {
    let root = ["src/lib.rs", "src/main.rs"]
        .into_iter()
        .map(PathBuf::from)
        .find(|path| path.exists())
        .ok_or_else(|| {
            IoError::new(
                ErrorKind::NotFound,
                "Rullst project has neither src/lib.rs nor src/main.rs",
            )
        })?;
    register_mod_ast(&root, "models")?;
    register_mod_ast(&root, "ai")
}

/// Adds required umbrella features while preserving the rest of the manifest.
#[doc(hidden)]
pub fn ensure_rullst_features(
    manifest: &str,
    required: &[&str],
) -> Result<String, Box<dyn std::error::Error>> {
    let mut document = manifest.parse::<DocumentMut>().map_err(|error| {
        IoError::new(
            ErrorKind::InvalidData,
            format!("Cargo.toml is not valid TOML: {error}"),
        )
    })?;
    let dependency = document
        .get_mut("dependencies")
        .and_then(Item::as_table_mut)
        .and_then(|dependencies| dependencies.get_mut("rullst"))
        .ok_or_else(|| {
            IoError::new(
                ErrorKind::InvalidData,
                "Cargo.toml must declare rullst under [dependencies]",
            )
        })?;

    match dependency {
        Item::Value(Value::String(version)) => {
            let mut table = InlineTable::new();
            table.insert("version", Value::String(version.clone()));
            table.insert(
                "features",
                Value::Array(required_feature_array(None, required)),
            );
            *dependency = Item::Value(Value::InlineTable(table));
        }
        Item::Value(Value::InlineTable(table)) => {
            let current = table.get("features").and_then(Value::as_array);
            let features = required_feature_array(current, required);
            table.insert("features", Value::Array(features));
        }
        Item::Table(table) => {
            let current = table.get("features").and_then(Item::as_array);
            let features = required_feature_array(current, required);
            table.insert("features", Item::Value(Value::Array(features)));
        }
        _ => {
            return Err(IoError::new(
                ErrorKind::InvalidData,
                "rullst dependency must be a version string, inline table, or dependency table",
            )
            .into());
        }
    }
    Ok(document.to_string())
}

fn required_feature_array(current: Option<&Array>, required: &[&str]) -> Array {
    let mut features = current.cloned().unwrap_or_default();
    for required_feature in required {
        if !features
            .iter()
            .any(|feature| feature.as_str() == Some(required_feature))
        {
            features.push(*required_feature);
        }
    }
    features
}

/// Returns the backend-specific models emitted by `make:chat-session`.
pub(crate) fn render_chat_models(backend: ProjectOrmBackend) -> &'static str {
    match backend {
        ProjectOrmBackend::Sqlx => SQLX_MODELS,
        ProjectOrmBackend::Turso => TURSO_MODELS,
    }
}

/// Returns the backend-specific service emitted by `make:chat-session`.
pub(crate) fn render_chat_service(backend: ProjectOrmBackend) -> &'static str {
    match backend {
        ProjectOrmBackend::Sqlx => SQLX_SERVICE,
        ProjectOrmBackend::Turso => TURSO_SERVICE,
    }
}

/// Returns the reversible backend-specific migration.
pub(crate) fn render_chat_migration(stem: &str, backend: ProjectOrmBackend) -> String {
    match backend {
        ProjectOrmBackend::Sqlx => SQLX_MIGRATION.replace("__MIGRATION_NAME__", stem),
        ProjectOrmBackend::Turso => TURSO_MIGRATION.replace("__MIGRATION_NAME__", stem),
    }
}

const SQLX_MODELS: &str = r#"use rullst::db::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "chat_sessions")]
pub struct ChatSession {
    pub id: i32,
    pub title: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "chat_messages")]
pub struct ChatMessage {
    pub id: i32,
    pub session_id: i32,
    pub role: String,
    pub content: String,
    pub created_at: i64,
}

impl ChatSession {
    pub fn new(title: impl Into<String>) -> Self {
        Self { id: 0, title: title.into(), created_at: unix_timestamp() }
    }

    /// Returns at most the newest 100 messages in chronological order.
    pub async fn history(&self) -> Result<Vec<ChatMessage>, rullst::orm::Error> {
        let mut messages = ChatMessage::query()
            .where_eq("session_id", self.id)
            .order_by_desc("id")
            .limit(100)
            .get()
            .await?;
        messages.reverse();
        Ok(messages)
    }
}

pub fn unix_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| i64::try_from(duration.as_secs()).unwrap_or(i64::MAX))
}
"#;

const TURSO_MODELS: &str = r#"use rullst::orm::polyglot::TursoOrder;

#[derive(Debug, Clone, rullst::orm::Orm)]
#[orm(table = "chat_sessions", backend = "turso")]
pub struct ChatSession {
    pub id: i64,
    pub title: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, rullst::orm::Orm)]
#[orm(table = "chat_messages", backend = "turso")]
pub struct ChatMessage {
    pub id: i64,
    pub session_id: i64,
    pub role: String,
    pub content: String,
    pub created_at: i64,
}

impl ChatSession {
    pub fn new(title: impl Into<String>) -> Self {
        Self { id: 0, title: title.into(), created_at: unix_timestamp() }
    }

    /// Returns at most the newest 100 messages in chronological order.
    pub async fn history(
        &self,
    ) -> Result<Vec<ChatMessage>, rullst::orm::polyglot::PolyglotError> {
        let mut messages = ChatMessage::query()?
            .where_eq("session_id", &self.id)?
            .order_by("id", TursoOrder::Desc)?
            .limit(100)?
            .get()
            .await?;
        messages.reverse();
        Ok(messages)
    }
}

pub fn unix_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| i64::try_from(duration.as_secs()).unwrap_or(i64::MAX))
}
"#;

const SQLX_SERVICE: &str = r#"use crate::models::chat::{ChatMessage, ChatSession, unix_timestamp};
use rullst::ai::{AiClient, AiError};

pub struct StatefulChat {
    session: ChatSession,
    client: AiClient,
    send_lock: tokio::sync::Mutex<()>,
}

#[derive(Debug)]
pub enum StatefulChatError {
    Database(rullst::orm::Error),
    Ai(AiError),
    InvalidHistoryRole(String),
    UnsavedSession,
}

impl std::fmt::Display for StatefulChatError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(error) => write!(formatter, "chat database error: {error}"),
            Self::Ai(error) => write!(formatter, "chat provider error: {error}"),
            Self::InvalidHistoryRole(role) => write!(formatter, "invalid stored chat role: {role}"),
            Self::UnsavedSession => formatter.write_str("save ChatSession before sending messages"),
        }
    }
}

impl std::error::Error for StatefulChatError {}
impl From<rullst::orm::Error> for StatefulChatError {
    fn from(error: rullst::orm::Error) -> Self { Self::Database(error) }
}
impl From<AiError> for StatefulChatError {
    fn from(error: AiError) -> Self { Self::Ai(error) }
}

impl StatefulChat {
    pub fn new(session: ChatSession, client: AiClient) -> Self {
        Self { session, client, send_lock: tokio::sync::Mutex::new(()) }
    }

    pub async fn send(&self, text: &str) -> Result<String, StatefulChatError> {
        let _send_guard = self.send_lock.lock().await;
        if self.session.id == 0 { return Err(StatefulChatError::UnsavedSession); }
        let history = self.session.history().await?;
        let mut builder = self.client.chat();
        for message in history {
            builder = match message.role.as_str() {
                "system" => builder.system(message.content),
                "user" => builder.user(message.content),
                "assistant" => builder.assistant(message.content),
                role => return Err(StatefulChatError::InvalidHistoryRole(role.to_string())),
            };
        }
        builder = builder.user(text);
        let mut user_message = ChatMessage {
            id: 0,
            session_id: self.session.id,
            role: "user".to_string(),
            content: text.to_string(),
            created_at: unix_timestamp(),
        };
        user_message.save().await?;
        let response = builder.send().await?;
        let mut assistant_message = ChatMessage {
            id: 0,
            session_id: self.session.id,
            role: "assistant".to_string(),
            content: response.clone(),
            created_at: unix_timestamp(),
        };
        assistant_message.save().await?;
        Ok(response)
    }
}
"#;

const TURSO_SERVICE: &str = r#"use crate::models::chat::{ChatMessage, ChatSession, unix_timestamp};
use rullst::ai::{AiClient, AiError};

pub struct StatefulChat {
    session: ChatSession,
    client: AiClient,
    send_lock: tokio::sync::Mutex<()>,
}

#[derive(Debug)]
pub enum StatefulChatError {
    Database(rullst::orm::polyglot::PolyglotError),
    Ai(AiError),
    InvalidHistoryRole(String),
    UnsavedSession,
}

impl std::fmt::Display for StatefulChatError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(error) => write!(formatter, "chat database error: {error}"),
            Self::Ai(error) => write!(formatter, "chat provider error: {error}"),
            Self::InvalidHistoryRole(role) => write!(formatter, "invalid stored chat role: {role}"),
            Self::UnsavedSession => formatter.write_str("save ChatSession before sending messages"),
        }
    }
}

impl std::error::Error for StatefulChatError {}
impl From<rullst::orm::polyglot::PolyglotError> for StatefulChatError {
    fn from(error: rullst::orm::polyglot::PolyglotError) -> Self { Self::Database(error) }
}
impl From<AiError> for StatefulChatError {
    fn from(error: AiError) -> Self { Self::Ai(error) }
}

impl StatefulChat {
    pub fn new(session: ChatSession, client: AiClient) -> Self {
        Self { session, client, send_lock: tokio::sync::Mutex::new(()) }
    }

    pub async fn send(&self, text: &str) -> Result<String, StatefulChatError> {
        let _send_guard = self.send_lock.lock().await;
        if self.session.id == 0 { return Err(StatefulChatError::UnsavedSession); }
        let history = self.session.history().await?;
        let mut builder = self.client.chat();
        for message in history {
            builder = match message.role.as_str() {
                "system" => builder.system(message.content),
                "user" => builder.user(message.content),
                "assistant" => builder.assistant(message.content),
                role => return Err(StatefulChatError::InvalidHistoryRole(role.to_string())),
            };
        }
        builder = builder.user(text);
        let mut user_message = ChatMessage {
            id: 0,
            session_id: self.session.id,
            role: "user".to_string(),
            content: text.to_string(),
            created_at: unix_timestamp(),
        };
        user_message.save().await?;
        let response = builder.send().await?;
        let mut assistant_message = ChatMessage {
            id: 0,
            session_id: self.session.id,
            role: "assistant".to_string(),
            content: response.clone(),
            created_at: unix_timestamp(),
        };
        assistant_message.save().await?;
        Ok(response)
    }
}
"#;

const SQLX_MIGRATION: &str = r#"use rullst::db::async_trait;
use rullst::db::schema::{Migration, Schema};

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str { "__MIGRATION_NAME__" }

    async fn up(&self) -> Result<(), rullst::orm::Error> {
        Schema::create("chat_sessions", |table| {
            table.id();
            table.string("title").not_null();
            table.big_integer("created_at").not_null();
        }).await?;
        Schema::create("chat_messages", |table| {
            table.id();
            table.integer("session_id").not_null();
            table.enum_col("role", vec!["system", "user", "assistant"]).not_null();
            table.string("content").not_null();
            table.big_integer("created_at").not_null();
        }).await
    }

    async fn down(&self) -> Result<(), rullst::orm::Error> {
        Schema::drop_if_exists("chat_messages").await?;
        Schema::drop_if_exists("chat_sessions").await
    }
}
"#;

const TURSO_MIGRATION: &str = r#"use rullst::orm::polyglot::{PolyglotError, TursoMigration, TursoStatement};

pub fn migration() -> Result<TursoMigration, PolyglotError> {
    TursoMigration::new(
        "__MIGRATION_NAME__",
        vec![
            TursoStatement::new(
                "CREATE TABLE chat_sessions (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT NOT NULL, created_at INTEGER NOT NULL)",
                vec![],
            )?,
            TursoStatement::new(
                "CREATE TABLE chat_messages (id INTEGER PRIMARY KEY AUTOINCREMENT, session_id INTEGER NOT NULL, role TEXT NOT NULL CHECK(role IN ('system', 'user', 'assistant')), content TEXT NOT NULL, created_at INTEGER NOT NULL)",
                vec![],
            )?,
        ],
    )?
    .with_down(vec![
        TursoStatement::new("DROP TABLE chat_messages", vec![])?,
        TursoStatement::new("DROP TABLE chat_sessions", vec![])?,
    ])
}
"#;
