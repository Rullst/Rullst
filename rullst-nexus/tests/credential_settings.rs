//! `NexusAuthPolicy::basic_from_env` resolves each credential from the process
//! environment, then the working directory's `.env`, without loading `.env`
//! into the process environment. This binary holds a single test because it
//! changes the working directory.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use rullst_nexus::{
    NEXUS_ADMIN_PASSWORD_ENV as PASSWORD, NEXUS_ADMIN_USERNAME_ENV as USERNAME, NexusAuthPolicy,
    NexusBuildError,
};

/// A fresh project directory, removed when dropped.
struct Project(std::path::PathBuf);

impl Project {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rullst-nexus-credential-settings-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn write_dotenv(&self, content: &str) {
        std::fs::write(self.0.join(".env"), content).unwrap();
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn basic_credentials_come_from_the_process_then_dotenv_without_loading_it() {
    let project = Project::new();
    std::env::set_current_dir(&project.0).unwrap();
    unsafe {
        std::env::remove_var(USERNAME);
        std::env::remove_var(PASSWORD);
    }

    assert_eq!(
        NexusAuthPolicy::basic_from_env().unwrap_err(),
        NexusBuildError::MissingCredential { variable: USERNAME }
    );

    project.write_dotenv(
        "NEXUS_ADMIN_USERNAME=dotenv_operator\nNEXUS_ADMIN_PASSWORD=dotenv-credential-0123456789\n",
    );
    assert!(matches!(
        NexusAuthPolicy::basic_from_env(),
        Ok(NexusAuthPolicy::Basic(_))
    ));
    assert!(
        std::env::var_os(USERNAME).is_none() && std::env::var_os(PASSWORD).is_none(),
        "Nexus must not load .env into the process environment"
    );

    // `.env` supplies the value only when the process environment lacks it.
    project.write_dotenv(
        "NEXUS_ADMIN_USERNAME=dotenv:operator\nNEXUS_ADMIN_PASSWORD=dotenv-credential-0123456789\n",
    );
    assert_eq!(
        NexusAuthPolicy::basic_from_env().unwrap_err(),
        NexusBuildError::UsernameContainsSeparator
    );
    unsafe { std::env::set_var(USERNAME, "process_operator") };
    assert!(matches!(
        NexusAuthPolicy::basic_from_env(),
        Ok(NexusAuthPolicy::Basic(_))
    ));
    unsafe { std::env::remove_var(USERNAME) };

    // A malformed `.env` fails closed and names only the variable.
    project.write_dotenv(
        "NEXUS_ADMIN_USERNAME=dotenv_operator\nNEXUS_ADMIN_PASSWORD='unterminated-private-marker\n",
    );
    let error = NexusAuthPolicy::basic_from_env().unwrap_err();
    assert_eq!(error, NexusBuildError::InvalidDotenv { variable: USERNAME });
    assert!(!error.to_string().contains("private-marker"));
}
