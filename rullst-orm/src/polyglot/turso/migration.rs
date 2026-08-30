use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};

use super::{PolyglotError, TursoQueryLimit, TursoStatement, TursoStore, TursoValue};
use crate::polyglot::CollectionName;

const CREATE_MIGRATION_TABLE: &str = "CREATE TABLE IF NOT EXISTS rullst_turso_migrations (name TEXT PRIMARY KEY, digest TEXT NOT NULL, applied_at INTEGER NOT NULL DEFAULT (unixepoch()))";

/// One ordered, checksummed Turso/libSQL migration.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct TursoMigration {
    name: CollectionName,
    statements: Vec<TursoStatement>,
    down_statements: Vec<TursoStatement>,
    digest: String,
}

impl TursoMigration {
    /// Creates a named migration containing at least one parameterized statement.
    pub fn new(
        name: impl Into<String>,
        statements: Vec<TursoStatement>,
    ) -> Result<Self, PolyglotError> {
        if statements.is_empty() {
            return Err(PolyglotError::InvalidIdentifier {
                kind: "Turso migration",
                reason: "at least one statement is required",
            });
        }
        let name = CollectionName::new(name)?;
        let digest = migration_digest(&name, &statements, &[]);
        Ok(Self {
            name,
            statements,
            down_statements: Vec::new(),
            digest,
        })
    }

    /// Adds the ordered statements used to roll this migration back.
    pub fn with_down(mut self, statements: Vec<TursoStatement>) -> Result<Self, PolyglotError> {
        if statements.is_empty() {
            return Err(PolyglotError::InvalidIdentifier {
                kind: "Turso rollback migration",
                reason: "at least one down statement is required",
            });
        }
        self.down_statements = statements;
        self.digest = migration_digest(&self.name, &self.statements, &self.down_statements);
        Ok(self)
    }

    /// Returns the stable migration name.
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Returns the content digest used for drift detection.
    pub fn digest(&self) -> &str {
        &self.digest
    }
}

/// Outcome of an ordered Turso/libSQL migration run.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TursoMigrationReport {
    /// Newly committed migration names.
    pub applied: Vec<String>,
    /// Previously committed migrations whose digests still match.
    pub skipped: Vec<String>,
}

/// Outcome of one Turso/libSQL migration rollback.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TursoRollbackReport {
    /// Migration removed from history, or `None` when history was empty.
    pub rolled_back: Option<String>,
}

impl TursoStore {
    /// Applies migrations atomically one at a time and rejects changed history.
    pub async fn migrate(
        &self,
        migrations: Vec<TursoMigration>,
    ) -> Result<TursoMigrationReport, PolyglotError> {
        self.execute(TursoStatement::new(CREATE_MIGRATION_TABLE, vec![])?)
            .await?;
        let limit = TursoQueryLimit::new(1)?;
        let mut report = TursoMigrationReport {
            applied: Vec::new(),
            skipped: Vec::new(),
        };

        for migration in migrations {
            let existing = self
                .query(
                    TursoStatement::new(
                        "SELECT digest FROM rullst_turso_migrations WHERE name = ?1",
                        vec![TursoValue::Text(migration.name().to_owned())],
                    )?,
                    limit,
                )
                .await?;
            if let Some(row) = existing.first() {
                let Some(TursoValue::Text(existing_digest)) = row.get("digest") else {
                    return Err(PolyglotError::Driver {
                        backend: "Turso",
                        message: "migration history returned a non-text digest".to_owned(),
                    });
                };
                if existing_digest != migration.digest() {
                    return Err(PolyglotError::Driver {
                        backend: "Turso",
                        message: format!(
                            "migration history drift detected for {}",
                            migration.name()
                        ),
                    });
                }
                report.skipped.push(migration.name().to_owned());
                continue;
            }

            let mut statements = migration.statements;
            statements.push(TursoStatement::new(
                "INSERT INTO rullst_turso_migrations (name, digest) VALUES (?1, ?2)",
                vec![
                    TursoValue::Text(migration.name.as_str().to_owned()),
                    TursoValue::Text(migration.digest),
                ],
            )?);
            let name = migration.name.as_str().to_owned();
            self.transaction(statements).await?;
            report.applied.push(name);
        }
        Ok(report)
    }

    /// Returns applied migration names in deterministic application order.
    pub async fn migration_status(&self) -> Result<Vec<String>, PolyglotError> {
        self.execute(TursoStatement::new(CREATE_MIGRATION_TABLE, vec![])?)
            .await?;
        let rows = self
            .query(
                TursoStatement::new(
                    "SELECT name FROM rullst_turso_migrations ORDER BY applied_at, rowid",
                    vec![],
                )?,
                TursoQueryLimit::new(10_000)?,
            )
            .await?;
        rows.into_iter()
            .map(|row| match row.get("name") {
                Some(TursoValue::Text(name)) => Ok(name.clone()),
                _ => Err(PolyglotError::Driver {
                    backend: "Turso",
                    message: "migration history returned a non-text name".to_owned(),
                }),
            })
            .collect()
    }

    /// Rolls back the most recently applied migration using its declared down statements.
    pub async fn rollback_last(
        &self,
        migrations: Vec<TursoMigration>,
    ) -> Result<TursoRollbackReport, PolyglotError> {
        self.execute(TursoStatement::new(CREATE_MIGRATION_TABLE, vec![])?)
            .await?;
        let rows = self
            .query(
                TursoStatement::new(
                    "SELECT name FROM rullst_turso_migrations ORDER BY applied_at DESC, rowid DESC LIMIT 1",
                    vec![],
                )?,
                TursoQueryLimit::new(1)?,
            )
            .await?;
        let Some(row) = rows.first() else {
            return Ok(TursoRollbackReport { rolled_back: None });
        };
        let Some(TursoValue::Text(name)) = row.get("name") else {
            return Err(PolyglotError::Driver {
                backend: "Turso",
                message: "migration history returned a non-text name".to_owned(),
            });
        };
        let migration = migrations
            .into_iter()
            .find(|migration| migration.name() == name)
            .ok_or_else(|| PolyglotError::Driver {
                backend: "Turso",
                message: format!("applied migration {name} is missing from the application"),
            })?;
        if migration.down_statements.is_empty() {
            return Err(PolyglotError::InvalidIdentifier {
                kind: "Turso rollback migration",
                reason: "the latest migration does not declare down statements",
            });
        }
        let mut statements = migration.down_statements;
        statements.push(TursoStatement::new(
            "DELETE FROM rullst_turso_migrations WHERE name = ?1",
            vec![TursoValue::Text(name.clone())],
        )?);
        self.transaction(statements).await?;
        Ok(TursoRollbackReport {
            rolled_back: Some(name.clone()),
        })
    }
}

fn migration_digest(
    name: &CollectionName,
    statements: &[TursoStatement],
    down_statements: &[TursoStatement],
) -> String {
    let mut digest = Sha256::new();
    digest.update(name.as_str().as_bytes());
    digest.update((statements.len() as u64).to_be_bytes());
    for statement in statements {
        update_statement_digest(&mut digest, statement);
    }
    digest.update((down_statements.len() as u64).to_be_bytes());
    for statement in down_statements {
        update_statement_digest(&mut digest, statement);
    }
    URL_SAFE_NO_PAD.encode(digest.finalize())
}

fn update_statement_digest(digest: &mut Sha256, statement: &TursoStatement) {
    digest.update((statement.sql.len() as u64).to_be_bytes());
    digest.update(statement.sql.as_bytes());
    digest.update((statement.parameters.len() as u64).to_be_bytes());
    for parameter in &statement.parameters {
        match parameter {
            TursoValue::Null => digest.update([0]),
            TursoValue::Integer(value) => {
                digest.update([1]);
                digest.update(value.to_be_bytes());
            }
            TursoValue::Real(value) => {
                digest.update([2]);
                digest.update(value.to_bits().to_be_bytes());
            }
            TursoValue::Text(value) => {
                digest.update([3]);
                digest.update((value.len() as u64).to_be_bytes());
                digest.update(value.as_bytes());
            }
            TursoValue::Blob(value) => {
                digest.update([4]);
                digest.update((value.len() as u64).to_be_bytes());
                digest.update(value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polyglot::TursoConfig;

    #[tokio::test]
    async fn migrations_are_atomic_idempotent_and_drift_checked() {
        let store = TursoStore::connect(TursoConfig::new("mock_local", ""))
            .await
            .unwrap();
        let migration = TursoMigration::new(
            "m20260829_create_events",
            vec![
                TursoStatement::new("CREATE TABLE events (id INTEGER PRIMARY KEY)", vec![])
                    .unwrap(),
            ],
        )
        .unwrap();
        let first = store.migrate(vec![migration.clone()]).await.unwrap();
        assert_eq!(first.applied, vec!["m20260829_create_events"]);
        let second = store.migrate(vec![migration]).await.unwrap();
        assert_eq!(second.skipped, vec!["m20260829_create_events"]);

        let changed = TursoMigration::new(
            "m20260829_create_events",
            vec![TursoStatement::new("CREATE TABLE changed (id INTEGER)", vec![]).unwrap()],
        )
        .unwrap();
        assert!(store.migrate(vec![changed]).await.is_err());
    }

    #[tokio::test]
    async fn migration_status_and_rollback_use_declared_down_statements() {
        let store = TursoStore::connect(TursoConfig::new("mock_local", ""))
            .await
            .unwrap();
        let migration = TursoMigration::new(
            "m20260829_reversible",
            vec![
                TursoStatement::new("CREATE TABLE reversible (id INTEGER PRIMARY KEY)", vec![])
                    .unwrap(),
            ],
        )
        .unwrap()
        .with_down(vec![
            TursoStatement::new("DROP TABLE reversible", vec![]).unwrap(),
        ])
        .unwrap();
        store.migrate(vec![migration.clone()]).await.unwrap();
        assert_eq!(
            store.migration_status().await.unwrap(),
            vec!["m20260829_reversible"]
        );
        let report = store.rollback_last(vec![migration]).await.unwrap();
        assert_eq!(report.rolled_back.as_deref(), Some("m20260829_reversible"));
        assert!(store.migration_status().await.unwrap().is_empty());
    }
}
