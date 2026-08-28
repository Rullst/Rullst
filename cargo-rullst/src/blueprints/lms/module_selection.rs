//! Explicit module-selection contract for the LMS blueprint.

/// Public LMS capabilities accepted by the scaffold selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum LmsModule {
    Auth,
    Learning,
    Assessment,
    Gamification,
    Automation,
    Realtime,
    Billing,
}

impl LmsModule {
    /// Stable CLI/configuration name for this module.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auth => "auth",
            Self::Learning => "learning",
            Self::Assessment => "assessment",
            Self::Gamification => "gamification",
            Self::Automation => "automation",
            Self::Realtime => "realtime",
            Self::Billing => "billing",
        }
    }
}

/// Invalid or not-yet-detached LMS module selections.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum LmsModuleError {
    #[error("LMS module `{0}` was selected more than once")]
    Duplicate(&'static str),
    #[error(
        "unsupported LMS module combination `{0}`; currently use `auth`, `auth,learning`, `auth,learning,assessment`, `auth,learning,gamification`, or omit --lms-modules for the complete starter"
    )]
    UnsupportedCombination(String),
    #[error("detached LMS module profiles do not yet support hot reload")]
    HotReloadUnsupported,
}

pub(super) fn validate_foundation(modules: &[LmsModule]) -> Result<(), LmsModuleError> {
    let mut selected = modules.to_vec();
    selected.sort_unstable();
    if let Some(duplicate) = selected.windows(2).find(|pair| pair[0] == pair[1]) {
        return Err(LmsModuleError::Duplicate(duplicate[0].as_str()));
    }
    if selected == [LmsModule::Auth]
        || selected == [LmsModule::Auth, LmsModule::Learning]
        || selected == [LmsModule::Auth, LmsModule::Learning, LmsModule::Assessment]
        || selected
            == [
                LmsModule::Auth,
                LmsModule::Learning,
                LmsModule::Gamification,
            ]
    {
        return Ok(());
    }
    Err(LmsModuleError::UnsupportedCombination(
        selected
            .into_iter()
            .map(LmsModule::as_str)
            .collect::<Vec<_>>()
            .join(","),
    ))
}

/// Validates a detached LMS selection before the project directory is created.
pub fn validate_module_selection(
    modules: &[LmsModule],
    hot_reload: bool,
) -> Result<(), LmsModuleError> {
    validate_foundation(modules)?;
    if hot_reload {
        return Err(LmsModuleError::HotReloadUnsupported);
    }
    Ok(())
}

/// Generates the bounded detached LMS profile selected by `modules`.
pub fn file_manifest_for_modules(
    project_name_safe: &str,
    hot_reload: bool,
    orm_pattern: &str,
    frontend_engine: &str,
    modules: &[LmsModule],
) -> Result<Vec<(&'static str, String)>, LmsModuleError> {
    validate_module_selection(modules, hot_reload)?;
    super::foundation::select(
        super::file_manifest(project_name_safe, hot_reload, orm_pattern, frontend_engine),
        hot_reload,
        modules,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selector_accepts_detached_auth_profiles_without_duplicates() {
        assert!(validate_foundation(&[LmsModule::Auth]).is_ok());
        assert!(validate_foundation(&[LmsModule::Learning, LmsModule::Auth]).is_ok());
        assert!(
            validate_foundation(&[LmsModule::Assessment, LmsModule::Auth, LmsModule::Learning,])
                .is_ok()
        );
        assert!(
            validate_foundation(&[
                LmsModule::Gamification,
                LmsModule::Auth,
                LmsModule::Learning,
            ])
            .is_ok()
        );
        assert_eq!(
            validate_foundation(&[LmsModule::Auth, LmsModule::Auth]),
            Err(LmsModuleError::Duplicate("auth"))
        );
        assert!(matches!(
            validate_foundation(&[LmsModule::Assessment]),
            Err(LmsModuleError::UnsupportedCombination(_))
        ));
    }
}
