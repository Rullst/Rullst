// src/cli/choices.rs — Value enums accepted by deterministic CLI generation.
#![cfg_attr(mutants, mutants::skip)]

use clap::ValueEnum;

/// Starter names accepted by `--blueprint`. The variant comments are the value
/// help that `--help` lists. `lms`, `blog`, `portfolio` and `erp` moved to the
/// repository's `examples/` in 13.0.0-alpha.4: they still parse, hidden, so the
/// command can point to the example before it writes anything.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BlueprintChoice {
    /// A server-rendered web page with HTMX (the default)
    Blank,
    /// The Blank starter as a headless JSON API, the same as --api
    Api,
    /// Authentication, Stripe billing and a pricing page
    Saas,
    /// Moved to examples/lms
    #[value(hide = true)]
    Lms,
    /// Moved to examples/blog
    #[value(hide = true)]
    Blog,
    /// Moved to examples/portfolio
    #[value(hide = true)]
    Portfolio,
    /// Moved to examples/erp
    #[value(hide = true)]
    Erp,
}

impl BlueprintChoice {
    /// The starter's stable blueprint ID, or the repository example that
    /// replaced a removed blueprint.
    pub const fn starter(self) -> Result<usize, crate::blueprints::MovedBlueprint> {
        use crate::blueprints::MovedBlueprint;
        match self {
            Self::Blank | Self::Api => Ok(crate::blueprints::BLANK_BLUEPRINT_ID),
            Self::Saas => Ok(crate::blueprints::SAAS_BLUEPRINT_ID),
            Self::Lms => Err(MovedBlueprint::LMS),
            Self::Blog => Err(MovedBlueprint::BLOG),
            Self::Portfolio => Err(MovedBlueprint::PORTFOLIO),
            Self::Erp => Err(MovedBlueprint::ERP),
        }
    }

    /// Whether the choice is the JSON API starter (`--api`).
    pub const fn is_api(self) -> bool {
        matches!(self, Self::Api)
    }
}

/// Primary relational databases accepted by non-interactive generation.
/// The variant comments are the `--database` value help that `--help` lists.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DatabaseChoice {
    /// SQLite through SQLx; every starter (the default)
    Sqlite,
    /// PostgreSQL through SQLx; every starter
    Postgres,
    /// MySQL through SQLx; every starter
    Mysql,
    /// MariaDB through SQLx's MySQL driver; every starter
    Mariadb,
    /// Turso/libSQL (Blank/API only; no Nexus, Studio, Schema or SQLx ORM)
    Turso,
}

impl DatabaseChoice {
    pub(super) const fn provider(self) -> &'static str {
        match self {
            Self::Sqlite => "Sqlite",
            Self::Postgres => "Postgres",
            Self::Mysql => "MySQL",
            Self::Mariadb => "MariaDB",
            Self::Turso => "Turso",
        }
    }
}
