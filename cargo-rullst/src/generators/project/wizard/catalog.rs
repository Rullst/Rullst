//! The choices `cargo rullst new` offers, as data: starters with a one-line
//! description, primary databases and optional features.

use super::PolyglotIntegration;
use super::plan::Database;
use crate::blueprints::{BLANK_BLUEPRINT_ID, SAAS_BLUEPRINT_ID};

/// One starter: a blueprint, and for the Blank blueprint the application kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Starter {
    pub(crate) blueprint: usize,
    /// The headless JSON API form of the Blank blueprint (`--api`).
    pub(crate) api: bool,
    pub(crate) name: &'static str,
    /// The `--blueprint` value.
    pub(crate) flag: &'static str,
    pub(crate) summary: &'static str,
}

/// Every starter, in the order the wizard lists them.
pub(crate) const STARTERS: [Starter; 3] = [
    Starter {
        blueprint: BLANK_BLUEPRINT_ID,
        api: false,
        name: "Blank",
        flag: "blank",
        summary: "A server-rendered web page with HTMX; add what you need",
    },
    Starter {
        blueprint: BLANK_BLUEPRINT_ID,
        api: true,
        name: "API",
        flag: "api",
        summary: "A headless JSON API with token-protected writes; no HTML",
    },
    Starter {
        blueprint: SAAS_BLUEPRINT_ID,
        api: false,
        name: "SaaS",
        flag: "saas",
        summary: "Authentication, Stripe billing and a pricing page",
    },
];

/// The starter a blueprint and application kind make up.
pub(crate) fn starter(blueprint: usize, api: bool) -> Option<&'static Starter> {
    STARTERS
        .iter()
        .find(|starter| starter.blueprint == blueprint && starter.api == api)
}

/// One primary database answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DatabaseOption {
    pub(crate) database: Database,
    pub(crate) name: &'static str,
    pub(crate) hint: &'static str,
}

const fn provider(
    provider: &'static str,
    name: &'static str,
    hint: &'static str,
) -> DatabaseOption {
    DatabaseOption {
        database: Database::Provider(provider),
        name,
        hint,
    }
}

const SQLX_DATABASES: [DatabaseOption; 4] = [
    provider(
        "Sqlite",
        "SQLite",
        "zero setup; recommended for a first run",
    ),
    provider(
        "Postgres",
        "PostgreSQL",
        "needs a running server on localhost:5432",
    ),
    provider("MySQL", "MySQL", "needs a running server on localhost:3306"),
    provider(
        "MariaDB",
        "MariaDB",
        "MySQL protocol; separately contract-tested",
    ),
];

/// What a Turso primary does not integrate with, shown next to its choice
/// and on the review screen (`docs/src/database-support.md` has the table).
pub(crate) const TURSO_PRIMARY_LIMITS: &str =
    "Blank/API only; no Nexus, Studio, Schema or SQLx ORM";

const BLANK_ONLY_DATABASES: [DatabaseOption; 2] = [
    provider("Turso", "Turso / libSQL", TURSO_PRIMARY_LIMITS),
    DatabaseOption {
        database: Database::None,
        name: "No database",
        hint: "add persistence later",
    },
];

/// The primary databases a blueprint supports. Turso-primary and the
/// database-free profile exist only for the Blank starter.
pub(crate) fn database_options(blueprint: usize) -> Vec<DatabaseOption> {
    let mut options = SQLX_DATABASES.to_vec();
    if blueprint == BLANK_BLUEPRINT_ID {
        options.extend(BLANK_ONLY_DATABASES);
    }
    options
}

/// The display name of a primary database answer.
pub(crate) fn database_name(database: Database) -> &'static str {
    SQLX_DATABASES
        .iter()
        .chain(BLANK_ONLY_DATABASES.iter())
        .find(|option| option.database == database)
        .map_or("SQLite", |option| option.name)
}

/// An optional capability chosen on the features screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Feature {
    Ai,
    Redis,
    Docker,
    Storage(PolyglotIntegration),
}

/// Every optional feature, in the order the wizard lists them.
pub(crate) const FEATURES: [(Feature, &str, &str); 8] = [
    (
        Feature::Ai,
        "AI features",
        "rullst-ai LLM client with offline mocks",
    ),
    (
        Feature::Redis,
        "Redis adapters",
        "queues, cache and a Redis-structures side store",
    ),
    (
        Feature::Docker,
        "Dockerfile",
        "multi-stage container image and .dockerignore",
    ),
    (
        Feature::Storage(PolyglotIntegration::Turso),
        "Turso / libSQL add-on",
        "side store: edge SQL; not the primary database",
    ),
    (
        Feature::Storage(PolyglotIntegration::MongoDb),
        "MongoDB",
        "side store: document CRUD",
    ),
    (
        Feature::Storage(PolyglotIntegration::DuckDb),
        "DuckDB",
        "side store: analytics; longer first build",
    ),
    (
        Feature::Storage(PolyglotIntegration::SurrealDb),
        "SurrealDB",
        "side store: documents, read-only graph queries",
    ),
    (
        Feature::Storage(PolyglotIntegration::Qdrant),
        "Qdrant",
        "side store: dense-vector search",
    ),
];

/// The display name of a feature.
pub(crate) fn feature_name(feature: Feature) -> &'static str {
    FEATURES
        .iter()
        .find(|(candidate, ..)| *candidate == feature)
        .map_or("", |(_, name, _)| name)
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod tests;

/// Optional storage adapters in the order generation lists them.
pub(crate) fn storage_order() -> impl Iterator<Item = PolyglotIntegration> {
    FEATURES
        .into_iter()
        .filter_map(|(feature, ..)| match feature {
            Feature::Storage(integration) => Some(integration),
            _ => None,
        })
}
