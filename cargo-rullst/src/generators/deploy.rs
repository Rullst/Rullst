use colored::*;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::blueprints::deploy::{
    CADDYFILE, DOCKER_COMPOSE_PROD, FLY_TOML, RAILWAY_JSON, RENDER_YAML, VPS_PROXY_ADDRESS,
};
use crate::generators::platform_name::{dns_label, package_name};

/// A supported deployment target, parsed before anything is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Platform {
    Fly,
    Railway,
    Render,
    Vps,
}

impl Platform {
    fn parse(value: &str) -> Result<Self, String> {
        match value.to_lowercase().as_str() {
            "fly" | "fly.io" => Ok(Self::Fly),
            "railway" => Ok(Self::Railway),
            "render" => Ok(Self::Render),
            "vps" => Ok(Self::Vps),
            other => Err(format!(
                "Unknown platform '{other}'. Supported: fly, railway, render, vps"
            )),
        }
    }
}

pub fn run_deploy(platform_arg: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        "🚀 Rullst Guided Cloud Deployment Wizard".bold().cyan()
    );

    // Reject an unknown platform before the Dockerfile scaffold mutates the project.
    let platform = match platform_arg {
        Some(p) => Platform::parse(p)?,
        None => {
            let theme = dialoguer::theme::ColorfulTheme::default();
            let options = &[
                "Fly.io (Global Edge Containers, automatic SSL & health probes)",
                "Railway (Zero-config deployment, PostgreSQL/Redis plugins)",
                "Render (Managed Cloud Services & Docker)",
                "VPS Production (Docker Compose + Caddy Reverse Proxy with Automatic SSL)",
            ];
            let selection = dialoguer::Select::with_theme(&theme)
                .with_prompt("☁️ Select your Target Deployment Platform")
                .default(0)
                .items(&options[..])
                .interact()?;

            match selection {
                1 => Platform::Railway,
                2 => Platform::Render,
                3 => Platform::Vps,
                _ => Platform::Fly,
            }
        }
    };

    // The binary name (Dockerfile, Railway start command) is the package name.
    let project_name =
        package_name(Path::new("Cargo.toml")).unwrap_or_else(|| "rullst_app".to_string());

    // Before the Dockerfile scaffold, so a new Dockerfile copies Rullst.toml.
    if platform == Platform::Vps {
        trust_vps_proxy(Path::new("Rullst.toml"))?;
    }

    // Ensure Dockerfile exists
    if !Path::new("Dockerfile").exists() {
        println!(
            "{}",
            "🐳 Dockerfile missing. Scaffolding optimized multi-stage build...".yellow()
        );
        crate::generators::project::generate_docker_files(
            Path::new("."),
            &project_name,
            None,
            None,
        )?;
    }

    match platform {
        Platform::Fly => deploy_fly(&project_name)?,
        Platform::Railway => deploy_railway(&project_name)?,
        Platform::Render => deploy_render(&project_name)?,
        Platform::Vps => deploy_vps(&project_name)?,
    }
    if let Some(notice) = platform_proxy_notice(platform, Path::new("Rullst.toml")) {
        println!("{}", notice.yellow());
    }

    Ok(())
}

/// How `Rullst.toml` came to trust the generated Caddy proxy.
#[derive(Debug, PartialEq, Eq)]
enum ProxyTrust {
    /// `trusted_proxies` was absent and now lists the proxy.
    Added,
    /// The proxy was already listed.
    AlreadyTrusted,
    /// An existing list without the proxy was left unchanged.
    OtherList,
}

/// Adds the generated Caddy proxy to `[security] trusted_proxies`, returning
/// the updated document when it changed. Without it every request reaches
/// the app from Caddy, so all clients share per-client limits such as the
/// starters' login and registration budget.
fn with_trusted_vps_proxy(config: &str) -> Result<(ProxyTrust, Option<String>), String> {
    let mut document = config.parse::<toml_edit::DocumentMut>().map_err(|error| {
        // The parser's message quotes the offending line, which may hold a
        // secret such as a database URL.
        let line = error.span().map(|span| {
            config
                .get(..span.start)
                .unwrap_or(config)
                .matches('\n')
                .count()
                + 1
        });
        format!(
            "Rullst.toml is not valid TOML{}; fix it before running `deploy --platform vps`",
            line.map_or_else(String::new, |line| format!(" (line {line})"))
        )
    })?;
    let security = document.entry("security").or_insert_with(toml_edit::table);
    // Comments cannot go inside an inline `security = { ... }` table.
    let standard_table = security.is_table();
    let security = security
        .as_table_like_mut()
        .ok_or("Rullst.toml `security` must be a table")?;
    if let Some(existing) = security.get("trusted_proxies") {
        let listed = existing.as_array().is_some_and(|networks| {
            networks
                .iter()
                .any(|network| network.as_str() == Some(VPS_PROXY_ADDRESS))
        });
        let trust = if listed {
            ProxyTrust::AlreadyTrusted
        } else {
            ProxyTrust::OtherList
        };
        return Ok((trust, None));
    }
    let networks = toml_edit::Array::from_iter([VPS_PROXY_ADDRESS]);
    security.insert("trusted_proxies", toml_edit::value(networks));
    if let Some(mut key) = security
        .key_mut("trusted_proxies")
        .filter(|_| standard_table)
    {
        key.leaf_decor_mut().set_prefix(
            "# Caddy in docker-compose.prod.yml (`deploy --platform vps`) reports the client.\n",
        );
    }
    Ok((ProxyTrust::Added, Some(document.to_string())))
}

fn trust_vps_proxy(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let config = match fs::read_to_string(path) {
        Ok(config) => config,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error.into()),
    };
    let (trust, updated) = with_trusted_vps_proxy(&config).map_err(std::io::Error::other)?;
    if let Some(updated) = updated {
        fs::write(path, updated)?;
    }
    let message = match trust {
        ProxyTrust::Added => format!(
            "  ✅ Trusted the Caddy proxy ({VPS_PROXY_ADDRESS}) in Rullst.toml [security] trusted_proxies"
        )
        .green(),
        ProxyTrust::AlreadyTrusted => {
            format!("  ℹ️ Rullst.toml already trusts the Caddy proxy ({VPS_PROXY_ADDRESS}).").blue()
        }
        ProxyTrust::OtherList => format!(
            "  ⚠️ Rullst.toml [security] trusted_proxies does not list the Caddy proxy ({VPS_PROXY_ADDRESS}); add it, or every client shares the proxy's rate limits."
        )
        .yellow(),
    };
    println!("{message}");
    Ok(())
}

/// Advice for a managed platform whose proxy forwards every request while
/// `Rullst.toml` trusts no proxy network. Their proxy addresses are
/// provider-specific, so nothing is guessed.
fn platform_proxy_notice(platform: Platform, config_path: &Path) -> Option<String> {
    let name = match platform {
        Platform::Fly => "Fly.io",
        Platform::Railway => "Railway",
        Platform::Render => "Render",
        Platform::Vps => return None,
    };
    let configured = fs::read_to_string(config_path)
        .ok()
        .and_then(|config| rullst_core::config::RullstConfig::from_toml(&config).ok())
        .is_some_and(|config| !config.security.trusted_proxies.is_empty());
    (!configured).then(|| {
        format!(
            "⚠️ {name} forwards every request through its proxy. Until Rullst.toml [security] trusted_proxies lists the networks that proxy connects from (see {name}'s documentation or the peer address in your access log), all clients share per-client limits such as the login and registration budget."
        )
    })
}

/// Runs a provider CLI and reports whether it was available.
///
/// A missing executable stays advisory because the manifest was still
/// written; a provider command that ran and failed is returned as an error so
/// scripts and CI do not record a failed deployment as successful.
fn run_provider_cli(program: &str, args: &[&str]) -> Result<bool, Box<dyn std::error::Error>> {
    match Command::new(program).args(args).status() {
        Ok(status) if status.success() => Ok(true),
        Ok(status) => Err(std::io::Error::other(format!(
            "`{program} {}` failed ({status}); the deployment did not complete",
            args.join(" ")
        ))
        .into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn deploy_fly(project_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let fly_file = "fly.toml";
    // Fly app names allow only lowercase letters, digits and dashes.
    let content = FLY_TOML.replace("APP_NAME", &dns_label(project_name));

    if !Path::new(fly_file).exists() {
        fs::write(fly_file, content)?;
        println!("{}", format!("  ✅ Created {}", fly_file).green());
    } else {
        println!("{}", format!("  ℹ️ Existing {} retained.", fly_file).blue());
    }

    println!("{}", "\n🚀 Deploying to Fly.io...".bold().magenta());

    if run_provider_cli("flyctl", &["deploy"])? {
        println!(
            "{}",
            "🎉 Application successfully deployed to Fly.io!"
                .bold()
                .green()
        );
    } else {
        println!(
            "{}",
            "💡 Fly CLI ('flyctl') not found. Execute manually:".yellow()
        );
        println!("   {}", "fly launch".cyan());
        println!("   {}", "fly deploy".cyan());
    }

    Ok(())
}

fn deploy_railway(project_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let railway_file = "railway.json";
    let content = RAILWAY_JSON.replace("APP_NAME", project_name);

    if !Path::new(railway_file).exists() {
        fs::write(railway_file, content)?;
        println!("{}", format!("  ✅ Created {}", railway_file).green());
    } else {
        println!(
            "{}",
            format!("  ℹ️ Existing {} retained.", railway_file).blue()
        );
    }

    println!("{}", "\n🚀 Deploying to Railway...".bold().magenta());

    if run_provider_cli("railway", &["up"])? {
        println!(
            "{}",
            "🎉 Application successfully deployed to Railway!"
                .bold()
                .green()
        );
    } else {
        println!(
            "{}",
            "💡 Railway CLI ('railway') not found. Execute manually:".yellow()
        );
        println!("   {}", "railway login".cyan());
        println!("   {}", "railway up".cyan());
    }

    Ok(())
}

fn deploy_render(project_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let render_file = "render.yaml";
    let content = RENDER_YAML.replace("APP_NAME", project_name);

    if !Path::new(render_file).exists() {
        fs::write(render_file, content)?;
        println!("{}", format!("  ✅ Created {}", render_file).green());
    } else {
        println!(
            "{}",
            format!("  ℹ️ Existing {} retained.", render_file).blue()
        );
    }

    println!(
        "{}",
        "\n✨ Render Blueprint generated successfully!"
            .bold()
            .green()
    );
    println!("  Connect your GitHub repository to Render and select 'New Blueprint Instance'.");

    Ok(())
}

fn deploy_vps(_project_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let compose_file = "docker-compose.prod.yml";
    let caddy_file = "Caddyfile";

    if !Path::new(compose_file).exists() {
        fs::write(compose_file, DOCKER_COMPOSE_PROD)?;
        println!("{}", format!("  ✅ Created {}", compose_file).green());
    } else if fs::read_to_string(compose_file)
        .is_ok_and(|compose| !compose.contains(&format!("ipv4_address: {VPS_PROXY_ADDRESS}")))
    {
        println!(
            "{}",
            format!("  ⚠️ The existing {compose_file} does not pin Caddy to {VPS_PROXY_ADDRESS}, the trusted proxy address. Add the `edge` network from a newly generated file, or move it aside and rerun this command.")
                .yellow()
        );
    }

    if !Path::new(caddy_file).exists() {
        fs::write(caddy_file, CADDYFILE)?;
        println!("{}", format!("  ✅ Created {}", caddy_file).green());
    }

    println!(
        "{}",
        "\n🔒 VPS Production Infrastructure Provisioned!"
            .bold()
            .green()
    );
    println!("  To launch on your VPS server:");
    println!("   DOMAIN=yourdomain.com docker compose -f docker-compose.prod.yml up -d --build");
    if fs::read_to_string("Dockerfile").is_ok_and(|dockerfile| !dockerfile.contains("Rullst.toml"))
    {
        println!(
            "{}",
            "  ⚠️ The existing Dockerfile does not copy Rullst.toml, so the image lacks the trusted proxy. Add `COPY --chown=10001:10001 Rullst.toml /app/Rullst.toml` to its runtime stage."
                .yellow()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rullst_core::config::RullstConfig;
    use rullst_core::security::TrustedProxyConfig;
    use std::net::IpAddr;

    fn trusts_caddy(config: &str) -> bool {
        let parsed = RullstConfig::from_toml(config).expect("valid Rullst.toml");
        let policy = TrustedProxyConfig::from_security_config(&parsed.security)
            .expect("valid trusted-proxy policy");
        let caddy: IpAddr = VPS_PROXY_ADDRESS.parse().expect("proxy address");
        policy.is_trusted(caddy) && !policy.is_trusted("172.31.250.11".parse().expect("peer"))
    }

    #[test]
    fn vps_compose_pins_the_caddy_address_inside_its_subnet() {
        // Behind Caddy every request reached the app from one dynamic bridge
        // address, so all clients shared the 10-per-minute login budget.
        assert!(DOCKER_COMPOSE_PROD.contains(&format!("ipv4_address: {VPS_PROXY_ADDRESS}\n")));
        assert!(DOCKER_COMPOSE_PROD.contains("- subnet: 172.31.250.0/24\n"));
        assert!(VPS_PROXY_ADDRESS.starts_with("172.31.250."));
        assert_eq!(
            DOCKER_COMPOSE_PROD
                .matches("networks:\n      - edge\n")
                .count(),
            1
        );
        assert!(CADDYFILE.contains("reverse_proxy app:3000"));
    }

    #[test]
    fn vps_deploy_trusts_only_the_generated_proxy() {
        let (trust, created) = with_trusted_vps_proxy("").expect("empty configuration");
        assert_eq!(trust, ProxyTrust::Added);
        let created = created.expect("new configuration");
        assert!(created.starts_with("[security]\n# Caddy in docker-compose.prod.yml"));
        assert!(trusts_caddy(&created));

        let saas = "[database]\nurl = \"sqlite://db.sqlite\"\n\n[security]\ncsrf_signed_webhook_paths = [\"/billing/webhook\"]\n";
        let (trust, updated) = with_trusted_vps_proxy(saas).expect("SaaS configuration");
        assert_eq!(trust, ProxyTrust::Added);
        let updated = updated.expect("updated configuration");
        assert!(updated.starts_with(saas));
        assert!(trusts_caddy(&updated));
        let parsed = RullstConfig::from_toml(&updated).expect("valid Rullst.toml");
        assert_eq!(
            parsed.security.csrf_signed_webhook_paths,
            ["/billing/webhook"]
        );
        assert_eq!(
            with_trusted_vps_proxy(&updated).expect("second run"),
            (ProxyTrust::AlreadyTrusted, None)
        );

        let inline = "security = { coep = \"require-corp\" }\n";
        let (trust, updated) = with_trusted_vps_proxy(inline).expect("inline table");
        assert_eq!(trust, ProxyTrust::Added);
        assert!(trusts_caddy(&updated.expect("updated inline table")));

        // An operator's own list is never rewritten.
        let custom = "[security]\ntrusted_proxies = [\"10.0.0.0/8\"]\n";
        assert_eq!(
            with_trusted_vps_proxy(custom).expect("custom list"),
            (ProxyTrust::OtherList, None)
        );
    }

    #[test]
    fn unparsable_configuration_is_reported_without_its_contents() {
        let error =
            with_trusted_vps_proxy("[database]\nurl = \"postgres://user:hunter2@db\nbroken")
                .expect_err("invalid TOML");
        assert!(error.contains("Rullst.toml is not valid TOML (line "));
        assert!(!error.contains("hunter2"));
    }

    #[test]
    fn managed_platforms_explain_an_untrusted_proxy() {
        let root = tempfile::tempdir().expect("temporary project");
        let config = root.path().join("Rullst.toml");
        for platform in [Platform::Fly, Platform::Railway, Platform::Render] {
            let notice = platform_proxy_notice(platform, &config).expect("no trusted proxies");
            assert!(notice.contains("[security] trusted_proxies"));
        }
        assert_eq!(platform_proxy_notice(Platform::Vps, &config), None);
        fs::write(&config, "[security]\ntrusted_proxies = [\"10.0.0.0/8\"]\n").expect("config");
        assert_eq!(platform_proxy_notice(Platform::Fly, &config), None);
    }

    #[test]
    fn platforms_are_parsed_case_insensitively_and_unknown_ones_rejected() {
        assert_eq!(Platform::parse("Fly.io"), Ok(Platform::Fly));
        assert_eq!(Platform::parse("RAILWAY"), Ok(Platform::Railway));
        assert_eq!(Platform::parse("render"), Ok(Platform::Render));
        assert_eq!(Platform::parse("vps"), Ok(Platform::Vps));
        let error = Platform::parse("flyio").unwrap_err();
        assert!(error.contains("Unknown platform 'flyio'"));
    }
}
