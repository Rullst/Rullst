//! Where the standalone Studio keeps its files.
//!
//! The project list lives in the user's configuration directory and the key
//! that encrypts its tokens in the user's data directory, so a copy of one
//! file alone (a dotfile repository, a backup of `~/.config`) reveals no
//! token. Both are outside every project.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

const DIRECTORY: [&str; 2] = ["rullst", "studio"];

/// The files of the standalone Studio.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudioPaths {
    /// The project list (`projects.json`).
    pub projects: PathBuf,
    /// The key that encrypts the tokens (`projects.key`).
    pub key: PathBuf,
    /// The bounded local history of monitored values (`history.sqlite`).
    pub history: PathBuf,
}

impl StudioPaths {
    /// The standard locations: `%APPDATA%` and `%LOCALAPPDATA%` on Windows,
    /// otherwise `$XDG_CONFIG_HOME` (or `~/.config`) and `$XDG_DATA_HOME` (or
    /// `~/.local/share`), each followed by `rullst/studio`. `None` when the
    /// variables give no absolute directory.
    pub fn from_env() -> Option<Self> {
        Self::resolve(|name| std::env::var_os(name))
    }

    pub(crate) fn resolve(var: impl Fn(&str) -> Option<OsString>) -> Option<Self> {
        let absolute = |value: OsString| {
            let path = PathBuf::from(value);
            path.is_absolute().then_some(path)
        };
        let (config, data) = if cfg!(windows) {
            (
                var("APPDATA").and_then(absolute)?,
                var("LOCALAPPDATA").and_then(absolute)?,
            )
        } else {
            let home = var("HOME").and_then(absolute);
            let xdg = |name: &str, fallback: &[&str]| {
                var(name)
                    .filter(|value| !value.is_empty())
                    .and_then(absolute)
                    .or_else(|| {
                        home.as_ref().map(|home| {
                            fallback
                                .iter()
                                .fold(home.clone(), |path, part| path.join(part))
                        })
                    })
            };
            (
                xdg("XDG_CONFIG_HOME", &[".config"])?,
                xdg("XDG_DATA_HOME", &[".local", "share"])?,
            )
        };
        Some(Self::in_directories(&config, &data))
    }

    /// The files under explicit configuration and data directories.
    pub fn in_directories(config: &Path, data: &Path) -> Self {
        let studio = |base: &Path| {
            DIRECTORY
                .iter()
                .fold(base.to_path_buf(), |path, part| path.join(part))
        };
        Self {
            projects: studio(config).join("projects.json"),
            key: studio(data).join("projects.key"),
            history: studio(data).join("history.sqlite"),
        }
    }
}

#[cfg(all(test, not(windows)))]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn resolve(vars: &[(&str, &str)]) -> Option<StudioPaths> {
        StudioPaths::resolve(|name| {
            vars.iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| OsString::from(value))
        })
    }

    #[test]
    fn xdg_directories_win_over_home_and_must_be_absolute() {
        let home = resolve(&[("HOME", "/home/u")]).unwrap();
        assert_eq!(
            home.projects,
            Path::new("/home/u/.config/rullst/studio/projects.json")
        );
        assert_eq!(
            home.key,
            Path::new("/home/u/.local/share/rullst/studio/projects.key")
        );
        let xdg = resolve(&[
            ("HOME", "/home/u"),
            ("XDG_CONFIG_HOME", "/cfg"),
            ("XDG_DATA_HOME", ""),
        ])
        .unwrap();
        assert_eq!(xdg.projects, Path::new("/cfg/rullst/studio/projects.json"));
        assert_eq!(
            xdg.history,
            Path::new("/home/u/.local/share/rullst/studio/history.sqlite")
        );
        assert_eq!(resolve(&[("HOME", "relative")]), None);
        assert_eq!(resolve(&[]), None);
        assert_eq!(
            StudioPaths::from_env(),
            StudioPaths::resolve(|name| std::env::var_os(name))
        );
    }
}
