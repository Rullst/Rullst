//! The complete answer set for one `cargo rullst new` run.

use super::catalog::{FEATURES, Feature, storage_order};
use super::{PolyglotIntegration, ProjectWizardOptions, V12_FRONTEND_ENGINE, V12_ORM_PATTERN};
use crate::blueprints::BLANK_BLUEPRINT_ID;

/// The primary relational database, or none (Blank starter only).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Database {
    None,
    Provider(&'static str),
}

/// Everything that decides the generated files and the commands `new` runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProjectPlan {
    /// The destination as typed: a name or a path ending in the package name.
    pub(crate) name: String,
    pub(crate) blueprint: usize,
    pub(crate) api: bool,
    pub(crate) database: Database,
    pub(crate) ai: bool,
    pub(crate) redis: bool,
    pub(crate) docker: bool,
    pub(crate) nix: bool,
    pub(crate) buildah: bool,
    /// Storage adapters requested by flags, in flag order.
    pub(crate) requested: Vec<PolyglotIntegration>,
    /// Storage add-ons chosen in the wizard.
    pub(crate) add_ons: Vec<PolyglotIntegration>,
    pub(crate) skip_initial_migration: bool,
}

impl ProjectPlan {
    pub(crate) fn db_needed(&self) -> bool {
        self.database != Database::None
    }

    /// The generator's provider name; database-free projects keep `Sqlite`.
    pub(crate) fn provider(&self) -> &'static str {
        match self.database {
            Database::Provider(provider) => provider,
            Database::None => "Sqlite",
        }
    }

    /// Requested adapters first, then Turso for a Turso primary, then the
    /// chosen add-ons in catalogue order: the order the manifest lists them.
    pub(crate) fn integrations(&self) -> Vec<PolyglotIntegration> {
        let mut integrations = self.requested.clone();
        if self.database == Database::Provider("Turso")
            && !integrations.contains(&PolyglotIntegration::Turso)
        {
            integrations.push(PolyglotIntegration::Turso);
        }
        for integration in storage_order() {
            if self.add_ons.contains(&integration) && !integrations.contains(&integration) {
                integrations.push(integration);
            }
        }
        integrations
    }

    /// Whether `feature` is part of the plan.
    pub(crate) fn has(&self, feature: Feature) -> bool {
        match feature {
            Feature::Ai => self.ai,
            Feature::Redis => self.redis,
            Feature::Docker => self.docker || self.buildah,
            Feature::Nix => self.nix,
            Feature::Storage(integration) => self.integrations().contains(&integration),
        }
    }

    pub(crate) fn set(&mut self, feature: Feature, enabled: bool) {
        match feature {
            Feature::Ai => self.ai = enabled,
            Feature::Redis => self.redis = enabled,
            Feature::Docker => self.docker = enabled,
            Feature::Nix => self.nix = enabled,
            Feature::Storage(integration) => {
                self.add_ons.retain(|candidate| *candidate != integration);
                if enabled {
                    self.add_ons.push(integration);
                }
            }
        }
    }

    /// The optional features in the plan, in catalogue order.
    pub(crate) fn features(&self) -> Vec<Feature> {
        // A Turso primary is the database, not an add-on.
        let turso_primary = self.database == Database::Provider("Turso");
        FEATURES
            .iter()
            .map(|(feature, ..)| *feature)
            .filter(|feature| self.has(*feature))
            .filter(|feature| {
                !(turso_primary && *feature == Feature::Storage(PolyglotIntegration::Turso))
            })
            .collect()
    }

    /// Adjusts answers a newly chosen blueprint does not support.
    pub(crate) fn fit_blueprint(&mut self) {
        if self.blueprint != BLANK_BLUEPRINT_ID {
            self.api = false;
            if matches!(self.database, Database::None | Database::Provider("Turso")) {
                self.database = Database::Provider("Sqlite");
            }
        }
    }

    /// The generator options for this plan.
    pub(crate) fn wizard_options(&self) -> ProjectWizardOptions {
        let integrations = self.integrations();
        let provider = self.provider();
        ProjectWizardOptions {
            name: self.name.clone(),
            api: self.api,
            db_provider: provider.to_string(),
            db_needed: self.db_needed(),
            hot_reload: false,
            blueprint_selection: self.blueprint,
            wants_ai: self.ai,
            wants_redis: self.redis,
            turso: integrations.contains(&PolyglotIntegration::Turso),
            polyglot_integrations: integrations,
            orm_pattern: if provider == "Turso" {
                "Turso Active Record"
            } else {
                V12_ORM_PATTERN
            }
            .to_string(),
            frontend_engine: V12_FRONTEND_ENGINE.to_string(),
        }
    }
}
