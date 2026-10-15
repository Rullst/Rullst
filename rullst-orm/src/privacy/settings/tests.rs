#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

/// A directory with `.env` holding `dotenv`, removed when dropped.
struct Project(std::path::PathBuf);

impl Project {
    fn new(dotenv: Option<&str>) -> Self {
        let directory = std::env::temp_dir().join(format!(
            "rullst-orm-settings-{}-{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        if let Some(dotenv) = dotenv {
            std::fs::write(directory.join(".env"), dotenv).unwrap();
        }
        Self(directory)
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A variable name no other test reads, so setting it races with nothing.
fn unique_name() -> String {
    format!("RULLST_ORM_SETTING_TEST_{}", rand::random::<u64>())
}

#[test]
fn dotenv_supplies_a_setting_the_process_lacks_without_exporting_it() {
    let name = unique_name();
    let project = Project::new(Some(&format!(
        "OTHER=first\n{name}=from-dotenv\nLAST=\"quoted\"\n"
    )));
    assert_eq!(
        setting_in(&project.0, &name).unwrap().as_deref(),
        Some("from-dotenv")
    );
    assert_eq!(
        setting_in(&project.0, "LAST").unwrap().as_deref(),
        Some("quoted")
    );
    assert_eq!(setting_in(&project.0, &unique_name()).unwrap(), None);
    assert!(std::env::var(&name).is_err(), ".env must not be exported");
}

#[test]
fn the_last_dotenv_entry_wins() {
    let name = unique_name();
    let project = Project::new(Some(&format!("{name}=first\n{name}=second\nOTHER=x\n")));
    assert_eq!(
        setting_in(&project.0, &name).unwrap().as_deref(),
        Some("second")
    );
}

#[test]
fn the_process_environment_wins_over_dotenv() {
    let name = unique_name();
    let project = Project::new(Some(&format!("{name}=from-dotenv\n")));
    // SAFETY: no other test reads or writes this randomly named variable.
    unsafe { std::env::set_var(&name, "from-process") };
    let value = setting_in(&project.0, &name);
    // SAFETY: as above.
    unsafe { std::env::remove_var(&name) };
    assert_eq!(value.unwrap().as_deref(), Some("from-process"));
}

#[test]
fn a_missing_dotenv_is_an_absent_setting() {
    let project = Project::new(None);
    assert_eq!(setting_in(&project.0, &unique_name()).unwrap(), None);
}

#[test]
fn an_unreadable_dotenv_fails_without_its_content() {
    let project = Project::new(None);
    std::fs::create_dir(project.0.join(".env")).unwrap();
    let error = setting_in(&project.0, &unique_name()).unwrap_err();
    assert!(
        matches!(&error, PrivacyError::EnvError(message) if message.starts_with(".env could not be read: ")),
        "{error:?}"
    );
}

#[test]
fn malformed_dotenv_entries_are_reported_by_number_only() {
    let name = unique_name();
    let project = Project::new(Some(&format!("{name}=fine\nBROKEN='secret-canary\n")));
    let error = setting_in(&project.0, &name).unwrap_err();
    assert_eq!(
        error,
        PrivacyError::EnvError("invalid .env syntax in entry 2".to_string())
    );
    assert!(!format!("{error} {error:?}").contains("secret-canary"));
}

#[test]
fn a_required_setting_names_where_it_was_looked_up() {
    let name = unique_name();
    assert_eq!(
        required_setting(&name).unwrap_err(),
        PrivacyError::EnvError(format!(
            "{name} is set neither in the process environment nor in ./.env"
        ))
    );
}
