// src/ui/update_check.rs — Background version check and update banner.

use colored::*;
use semver::Version;

fn enabled_env_flag(value: Option<&std::ffi::OsStr>) -> bool {
    value
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes"
            )
        })
}

fn update_check_disabled() -> bool {
    enabled_env_flag(std::env::var_os("RULLST_DISABLE_UPDATE_CHECK").as_deref())
        || enabled_env_flag(std::env::var_os("CARGO_NET_OFFLINE").as_deref())
}

fn get_cache_path() -> std::path::PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push("cargo_rullst_version_cache.txt");
    dir
}

fn is_version_newer(current: &str, latest: &str) -> bool {
    match (Version::parse(current), Version::parse(latest)) {
        (Ok(current), Ok(latest)) => latest > current,
        _ => false,
    }
}

pub fn check_update_available() -> Option<String> {
    let cache_path = get_cache_path();
    if cache_path.exists()
        && let Ok(cached_version) = std::fs::read_to_string(&cache_path)
    {
        let cached_version = cached_version.trim().to_string();
        let current_version = env!("CARGO_PKG_VERSION");
        if is_version_newer(current_version, &cached_version) {
            return Some(cached_version);
        }
    }
    None
}

pub fn trigger_background_update_check() {
    if update_check_disabled() {
        return;
    }

    std::thread::spawn(|| {
        let cache_path = get_cache_path();
        let needs_refresh = if cache_path.exists() {
            if let Ok(metadata) = std::fs::metadata(&cache_path) {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(elapsed) = modified.elapsed() {
                        elapsed.as_secs() > 86400 // 24 hours
                    } else {
                        true
                    }
                } else {
                    true
                }
            } else {
                true
            }
        } else {
            true
        };

        if needs_refresh {
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(4))
                .build();
            if let Ok(client) = client {
                let response = client
                    .get("https://crates.io/api/v1/crates/cargo-rullst")
                    .header("User-Agent", "cargo-rullst-update-check/12")
                    .send();
                if let Ok(res) = response {
                    #[derive(serde::Deserialize)]
                    struct CrateInfo {
                        max_version: String,
                    }
                    #[derive(serde::Deserialize)]
                    struct CratesIoResponse {
                        #[serde(rename = "crate")]
                        krate: CrateInfo,
                    }
                    if let Ok(data) = res.json::<CratesIoResponse>() {
                        let _ = std::fs::write(&cache_path, &data.krate.max_version);
                    }
                }
            }
        }
    });
}

pub fn print_update_banner(latest_version: &str) {
    let current_version = env!("CARGO_PKG_VERSION");
    println!();
    println!(
        "{}",
        "┌────────────────────────────────────────────────────────────┐"
            .cyan()
            .bold()
    );
    println!(
        "{}  🚀 {} {:<19} {}",
        "│".cyan().bold(),
        "New cargo-rullst available:".bold().yellow(),
        format!("{} → {}", current_version, latest_version)
            .green()
            .bold(),
        "│".cyan().bold()
    );
    println!(
        "{}  Install that exact CLI, then run                     {}",
        "│".cyan().bold(),
        "│".cyan().bold()
    );
    println!(
        "{}  {} to migrate the project transactionally. {}",
        "│".cyan().bold(),
        "'cargo rullst upgrade'".magenta().bold(),
        "│".cyan().bold()
    );
    println!(
        "{}",
        "└────────────────────────────────────────────────────────────┘"
            .cyan()
            .bold()
    );
    println!();
}

#[cfg(test)]
mod tests {
    use super::{enabled_env_flag, is_version_newer};
    use std::ffi::OsStr;

    #[test]
    fn offline_flag_parser_is_explicit() {
        assert!(enabled_env_flag(Some(OsStr::new("true"))));
        assert!(enabled_env_flag(Some(OsStr::new("1"))));
        assert!(enabled_env_flag(Some(OsStr::new("YES"))));
        assert!(!enabled_env_flag(Some(OsStr::new("false"))));
        assert!(!enabled_env_flag(None));
    }

    #[test]
    fn update_comparison_supports_prereleases_without_accepting_invalid_versions() {
        assert!(is_version_newer("12.0.0-rc.1", "12.0.0-rc.2"));
        assert!(is_version_newer("12.0.0-rc.2", "12.0.0"));
        assert!(!is_version_newer("12.0.0", "12.0.0-rc.2"));
        assert!(!is_version_newer("12.0.0", "not-a-version"));
    }
}
