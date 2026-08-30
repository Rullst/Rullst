// src/generators/desktop/scaffold.rs — Scaffold Omni (Tauri) packaging for desktop & mobile.

use crate::generators::is_rullst_project;
use colored::*;
use std::fs;
use std::path::Path;

mod templates;

use templates::{generate_icon_source, write_omni_files};

#[cfg_attr(mutants, mutants::skip)]
pub fn scaffold_omni_system(
    requested_platforms: &[&str],
    backend_url: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let theme = dialoguer::theme::ColorfulTheme::default();

    if !is_rullst_project() {
        return Err("make:omni must run in a Cargo project with a rullst dependency".into());
    }

    println!(
        "{}",
        "🖥️ Starting scaffolding of Rullst Omni packaging system..."
            .cyan()
            .bold()
    );

    let platforms = [
        "Desktop (Windows/Mac/Linux)".to_string(),
        format!(
            "Android {}",
            "(Requires Android Studio SDK)".truecolor(255, 165, 0)
        ),
        format!("iOS {}", "(iPhone/iPad - Requires macOS)".red()),
    ];

    let explicit_platforms = !requested_platforms.is_empty();
    let selections = if explicit_platforms {
        requested_platforms
            .iter()
            .map(|platform| match *platform {
                "desktop" => Ok(0),
                "android" => Ok(1),
                "ios" => Ok(2),
                other => Err(format!("unsupported Omni platform `{other}`")),
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        match dialoguer::MultiSelect::with_theme(&theme)
            .with_prompt(format!("{}", "⚠️ Select target platforms for Rullst Omni (Press <Space> to select, <Enter> to confirm)".truecolor(255, 165, 0).bold()))
            .items(&platforms[..])
            .defaults(&[true, false, false])
            .interact()
        {
            Ok(selections) => selections,
            Err(_) => {
                println!("{}", "⚠️ Warning: Non-interactive terminal detected. Defaulting to Desktop target.".yellow());
                vec![0]
            }
        }
    };

    let mut has_desktop = false;
    let mut has_android = false;
    let mut has_ios = false;

    for &selection in &selections {
        match selection {
            0 => has_desktop = true,
            1 => has_android = true,
            2 => has_ios = true,
            _ => {}
        }
    }

    if !has_desktop && !has_android && !has_ios {
        println!("{}", "⚠️ Warning: No platforms selected (remember to press <Space> to select). Defaulting to Desktop."
            .truecolor(255, 165, 0)
            .bold());
        has_desktop = true;
    }
    let _ = has_desktop;
    let backend_url = validated_backend_url(backend_url, has_android || has_ios)?;

    // Create Directories
    let omni_dir = Path::new("omni-app");
    let src_dir = omni_dir.join("src");
    let icons_dir = omni_dir.join("icons");

    fs::create_dir_all(omni_dir)?;
    fs::create_dir_all(&src_dir)?;
    fs::create_dir_all(&icons_dir)?;

    write_omni_files(omni_dir, &src_dir, &backend_url)?;
    generate_icon_source(&icons_dir)?;
    run_npm_install(omni_dir)?;
    generate_platform_icons(omni_dir)?;

    if has_android {
        init_mobile_target(omni_dir, "android")?;
    }

    if has_ios {
        if cfg!(target_os = "macos") {
            init_mobile_target(omni_dir, "ios")?;
        } else if explicit_platforms {
            return Err("explicit iOS initialization requires a macOS host with Xcode".into());
        } else {
            println!(
                "{}",
                "⚠️ Warning: iOS initialization requires a macOS host. Skipping iOS setup."
                    .truecolor(255, 165, 0)
                    .bold()
            );
        }
    }

    println!(
        "{}\n\n{}",
        "✅ Rullst Omni template successfully generated in 'omni-app/'!"
            .green()
            .bold(),
        "To start developing, run:".cyan()
    );

    if has_desktop {
        println!("  {}", "cargo rullst omni desktop".white().bold());
    }
    if has_android {
        println!("  {}", "cargo rullst omni android".white().bold());
    }
    if has_ios {
        println!("  {}", "cargo rullst omni ios".white().bold());
    }

    Ok(())
}

fn validated_backend_url(
    backend_url: Option<&str>,
    mobile_selected: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    let value = match backend_url {
        Some(value) => value.trim(),
        None if mobile_selected => {
            return Err(
                "--backend-url is required for Android/iOS; use HTTPS for distributable apps"
                    .into(),
            );
        }
        None => "http://localhost:3000",
    };
    let parsed = reqwest::Url::parse(value)
        .map_err(|error| format!("invalid Omni backend URL `{value}`: {error}"))?;
    let allowed_http_host = parsed
        .host_str()
        .is_some_and(|host| matches!(host, "localhost" | "127.0.0.1" | "10.0.2.2"));
    if parsed.scheme() != "https" && !(parsed.scheme() == "http" && allowed_http_host) {
        return Err(
            "Omni backend URL must use HTTPS; HTTP is limited to localhost/127.0.0.1/10.0.2.2 development"
                .into(),
        );
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("Omni backend URL must not embed credentials".into());
    }
    Ok(parsed.to_string().trim_end_matches('/').to_string())
}

fn run_npm_install(omni_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let has_npm = if cfg!(windows) {
        std::process::Command::new("npm.cmd")
            .args(["--version"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    } else {
        std::process::Command::new("npm")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    };

    if !has_npm {
        return Err("npm is required to install the pinned Tauri CLI for Omni".into());
    }

    println!("📦 Installing the pinned Tauri CLI via npm...");
    let mut npm_install = if cfg!(windows) {
        let mut command = std::process::Command::new("npm.cmd");
        command.arg("install");
        command
    } else {
        let mut command = std::process::Command::new("npm");
        command.arg("install");
        command
    };
    let status = npm_install.current_dir(omni_dir).status()?;
    if !status.success() {
        return Err("npm install failed while preparing the Omni scaffold".into());
    }
    Ok(())
}

fn generate_platform_icons(omni_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut command = super::runner::get_tauri_command(omni_dir)?;
    let status = command
        .arg("icon")
        .arg("icons/icon.svg")
        .current_dir(omni_dir)
        .status()?;
    if !status.success() {
        return Err("Tauri failed to generate platform icon assets".into());
    }
    Ok(())
}

fn init_mobile_target(omni_dir: &Path, platform: &str) -> Result<(), Box<dyn std::error::Error>> {
    if platform == "android" {
        println!("🤖 Initializing Android support folder inside 'omni-app/'...");
        println!("{}", "💡 Tip: If Omni asks to install Android command line tools or NDK, typing 'y' (yes) is highly recommended!"
            .truecolor(255, 165, 0)
            .bold());
    } else {
        println!("🍎 Initializing iOS support folder inside 'omni-app/'...");
    }

    let mut command = super::runner::get_tauri_command(omni_dir)?;
    let status = command
        .arg(platform)
        .arg("init")
        .arg("--ci")
        .current_dir(omni_dir)
        .status()?;
    if !status.success() {
        return Err(format!("Tauri failed to initialize the {platform} target").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn generated_omni_runtime_is_valid_and_zero_panic() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "rullst-omni-zero-panic-{}-{unique}",
            std::process::id()
        ));
        let src = root.join("src");

        fs::create_dir_all(&src).expect("temporary Omni source directory");
        write_omni_files(&root, &src, "https://api.example.com").expect("Omni scaffold files");
        let generated = fs::read_to_string(src.join("lib.rs")).expect("generated Omni runtime");
        let redirect = fs::read_to_string(src.join("redirect.js")).expect("generated redirect");
        let config = fs::read_to_string(root.join("tauri.conf.json")).expect("Tauri config");
        let package = fs::read_to_string(root.join("package.json")).expect("npm manifest");

        syn::parse_file(&generated).expect("generated Omni runtime must parse as Rust");
        serde_json::from_str::<serde_json::Value>(&config).expect("valid Tauri JSON config");
        serde_json::from_str::<serde_json::Value>(&package).expect("valid npm manifest");
        assert!(!generated.contains(".unwrap("));
        assert!(!generated.contains(".expect("));
        assert!(!generated.contains("panic!("));
        assert!(!generated.contains("todo!("));
        assert!(!generated.contains("unimplemented!("));
        assert!(redirect.contains("https://api.example.com"));
        assert!(!config.contains("\"csp\": null"));
        assert!(config.contains("default-src 'self'"));
        assert!(!config.contains("withGlobalTauri"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn mobile_backend_urls_fail_closed() {
        assert!(validated_backend_url(None, true).is_err());
        assert!(validated_backend_url(Some("ftp://example.com"), true).is_err());
        assert!(validated_backend_url(Some("http://example.com"), true).is_err());
        assert!(validated_backend_url(Some("https://user:secret@example.com"), true).is_err());
        assert_eq!(
            validated_backend_url(Some("https://api.example.com/"), true)
                .expect("secure backend URL"),
            "https://api.example.com"
        );
        assert_eq!(
            validated_backend_url(Some("http://10.0.2.2:3000"), true)
                .expect("Android emulator development URL"),
            "http://10.0.2.2:3000"
        );
    }
}
