// src/cli/choices.rs — Value enums accepted by deterministic CLI generation.
#![cfg_attr(mutants, mutants::skip)]

use clap::ValueEnum;

/// Stable blueprint names accepted by non-interactive project generation.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BlueprintChoice {
    Blank,
    Lms,
    Saas,
    Blog,
    Portfolio,
    Erp,
}

impl BlueprintChoice {
    pub const fn id(self) -> usize {
        match self {
            Self::Blank => crate::blueprints::BLANK_BLUEPRINT_ID,
            Self::Lms => crate::blueprints::LMS_BLUEPRINT_ID,
            Self::Saas => crate::blueprints::SAAS_BLUEPRINT_ID,
            Self::Blog => crate::blueprints::BLOG_BLUEPRINT_ID,
            Self::Portfolio => crate::blueprints::PORTFOLIO_BLUEPRINT_ID,
            Self::Erp => crate::blueprints::ERP_BLUEPRINT_ID,
        }
    }
}

/// Primary relational databases accepted by non-interactive generation.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DatabaseChoice {
    Sqlite,
    Postgres,
    Mysql,
    Mariadb,
    Turso,
}

/// Platforms accepted by deterministic Omni scaffolding.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum OmniPlatformChoice {
    Desktop,
    Android,
    Ios,
}

impl OmniPlatformChoice {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Desktop => "desktop",
            Self::Android => "android",
            Self::Ios => "ios",
        }
    }
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
