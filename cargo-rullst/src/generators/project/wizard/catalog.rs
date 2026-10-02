//! The choices `cargo rullst new` offers, as data: blueprints with a one-line
//! description, primary databases and optional features.

use super::PolyglotIntegration;
use super::plan::Database;
use crate::blueprints::{
    BLANK_BLUEPRINT_ID, BLOG_BLUEPRINT_ID, ERP_BLUEPRINT_ID, LMS_BLUEPRINT_ID,
    PORTFOLIO_BLUEPRINT_ID, SAAS_BLUEPRINT_ID,
};

/// One starter blueprint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Blueprint {
    pub(crate) id: usize,
    pub(crate) name: &'static str,
    /// The `--blueprint` value.
    pub(crate) flag: &'static str,
    pub(crate) summary: &'static str,
}

/// Every public blueprint, in stable ID order.
pub(crate) const BLUEPRINTS: [Blueprint; 6] = [
    Blueprint {
        id: BLANK_BLUEPRINT_ID,
        name: "Blank",
        flag: "blank",
        summary: "Minimal HTMX page or JSON API; Nexus CMS not included",
    },
    Blueprint {
        id: LMS_BLUEPRINT_ID,
        name: "LMS Platform",
        flag: "lms",
        summary: "Courses, lessons, video player and learner accounts",
    },
    Blueprint {
        id: SAAS_BLUEPRINT_ID,
        name: "SaaS Starter",
        flag: "saas",
        summary: "Authentication, Stripe billing and a pricing page",
    },
    Blueprint {
        id: BLOG_BLUEPRINT_ID,
        name: "Blog / Press",
        flag: "blog",
        summary: "Posts and feed, pre-wired with Nexus CMS",
    },
    Blueprint {
        id: PORTFOLIO_BLUEPRINT_ID,
        name: "Portfolio",
        flag: "portfolio",
        summary: "Developer showcase with a profile edited in Nexus CMS",
    },
    Blueprint {
        id: ERP_BLUEPRINT_ID,
        name: "ERP Pocket",
        flag: "erp",
        summary: "Inventory, stock movements and orders with an auto-CMS",
    },
];

pub(crate) fn blueprint(id: usize) -> Option<&'static Blueprint> {
    BLUEPRINTS.iter().find(|blueprint| blueprint.id == id)
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

const BLANK_ONLY_DATABASES: [DatabaseOption; 2] = [
    provider(
        "Turso",
        "Turso / libSQL",
        "edge SQL; Blank and API starters only",
    ),
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
    Nix,
    Storage(PolyglotIntegration),
}

/// Every optional feature, in the order the wizard lists them.
pub(crate) const FEATURES: [(Feature, &str, &str); 9] = [
    (
        Feature::Ai,
        "AI features",
        "rullst-ai LLM client with offline mocks",
    ),
    (
        Feature::Redis,
        "Redis adapters",
        "queues, cache and ORM; needs explicit configuration",
    ),
    (
        Feature::Docker,
        "Dockerfile",
        "multi-stage container image and .dockerignore",
    ),
    (
        Feature::Nix,
        "Nix flake",
        "reproducible dev shell with direnv",
    ),
    (
        Feature::Storage(PolyglotIntegration::Turso),
        "Turso / libSQL add-on",
        "edge SQL beside the primary database",
    ),
    (
        Feature::Storage(PolyglotIntegration::MongoDb),
        "MongoDB",
        "document CRUD",
    ),
    (
        Feature::Storage(PolyglotIntegration::DuckDb),
        "DuckDB",
        "in-process analytics; longer first build",
    ),
    (
        Feature::Storage(PolyglotIntegration::SurrealDb),
        "SurrealDB",
        "documents and read-only graph queries",
    ),
    (
        Feature::Storage(PolyglotIntegration::Qdrant),
        "Qdrant",
        "dense-vector search",
    ),
];

/// The display name of a feature.
pub(crate) fn feature_name(feature: Feature) -> &'static str {
    FEATURES
        .iter()
        .find(|(candidate, ..)| *candidate == feature)
        .map_or("", |(_, name, _)| name)
}

/// Optional storage adapters in the order generation lists them.
pub(crate) fn storage_order() -> impl Iterator<Item = PolyglotIntegration> {
    FEATURES
        .into_iter()
        .filter_map(|(feature, ..)| match feature {
            Feature::Storage(integration) => Some(integration),
            _ => None,
        })
}
