// src/generators/foundry/deploy.rs — SSH deployment pipeline steps.

use super::config::FoundryConfig;
use colored::*;
use std::fs;
use std::process::Command;

pub fn get_ssh_base_args(cfg: &FoundryConfig) -> Vec<String> {
    let mut args = Vec::new();
    let ssh_port_num = if cfg.ssh_port.is_empty() {
        "22"
    } else {
        &cfg.ssh_port
    };
    args.push("-p".to_string());
    args.push(ssh_port_num.to_string());

    let ssh_key_expanded = cfg.ssh_key.replace(
        "~",
        &std::env::var("HOME").unwrap_or_else(|_| std::env::var("USERPROFILE").unwrap_or_default()),
    );
    if !ssh_key_expanded.is_empty() {
        args.push("-i".to_string());
        args.push(ssh_key_expanded);
    }
    args.push("-o".to_string());
    args.push("StrictHostKeyChecking=accept-new".to_string());
    args.push("--".to_string());
    args.push(format!("{}@{}", cfg.user, cfg.host));
    args
}

pub fn run_ssh(cmd: &str, base_args: &[String]) -> Result<bool, Box<dyn std::error::Error>> {
    let mut full_args = base_args.to_vec();
    full_args.push(cmd.to_string());
    let status = Command::new("ssh").args(&full_args).status()?;
    Ok(status.success())
}

pub fn execute_build_step(cfg: &FoundryConfig) -> Result<String, Box<dyn std::error::Error>> {
    println!(
        "{}",
        "📦 [1/5] Building production binary...".bold().yellow()
    );
    let mut build_args = vec!["build".to_string()];
    if cfg.profile != "debug" {
        build_args.push("--release".to_string());
    }
    if !cfg.target_triple.is_empty() {
        build_args.push("--target".to_string());
        build_args.push(cfg.target_triple.clone());
    }

    if !Command::new("cargo").args(&build_args).status()?.success() {
        println!("{}", "❌ Build failed. Aborting deployment.".red().bold());
        std::process::exit(1);
    }
    println!("{}", "  ✅ Build successful.".green());

    let bin_subdir = if cfg.target_triple.is_empty() {
        if cfg.profile == "debug" {
            "debug".to_string()
        } else {
            "release".to_string()
        }
    } else {
        format!(
            "{}/{}",
            cfg.target_triple,
            if cfg.profile == "debug" {
                "debug"
            } else {
                "release"
            }
        )
    };

    let cargo_toml_content = fs::read_to_string("Cargo.toml").unwrap_or_default();
    let bin_name = cargo_toml_content
        .lines()
        .find(|l| l.trim_start().starts_with("name"))
        .and_then(|l| l.split('=').nth(1))
        .map(|s| s.trim().trim_matches('"').to_string())
        .unwrap_or_else(|| cfg.app_name.clone());

    Ok(format!("target/{}/{}", bin_subdir, bin_name))
}

pub fn execute_provision_step(ssh_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        "🖥️  [2/5] Provisioning server environment..."
            .bold()
            .yellow()
    );
    let provision_cmd = r#"set -e
apt-get update -qq
apt-get install -y -qq docker.io curl wget || yum install -y docker curl wget || true
systemctl enable docker --now || true
mkdir -p /app/data /app/bin /app/config
echo "✅ Server environment ready.""#
        .to_string();

    if !run_ssh(&provision_cmd, ssh_args)? {
        println!(
            "{}",
            "⚠️  Server provisioning had warnings (continuing anyway)...".yellow()
        );
    } else {
        println!("{}", "  ✅ Server provisioned.".green());
    }
    Ok(())
}

#[cfg_attr(mutants, mutants::skip)]
pub fn execute_upload_step(
    cfg: &FoundryConfig,
    local_bin: &str,
    _ssh_args: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        "📤 [3/5] Uploading application binary...".bold().yellow()
    );
    let mut scp_args = Vec::new();
    let ssh_port_num = if cfg.ssh_port.is_empty() {
        "22"
    } else {
        &cfg.ssh_port
    };
    scp_args.push("-P".to_string());
    scp_args.push(ssh_port_num.to_string());

    let ssh_key_expanded = cfg.ssh_key.replace(
        "~",
        &std::env::var("HOME").unwrap_or_else(|_| std::env::var("USERPROFILE").unwrap_or_default()),
    );
    if !ssh_key_expanded.is_empty() {
        scp_args.push("-i".to_string());
        scp_args.push(ssh_key_expanded);
    }
    scp_args.push("-o".to_string());
    scp_args.push("StrictHostKeyChecking=accept-new".to_string());
    let bin_name = std::path::Path::new(local_bin)
        .file_name()
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "deployment binary path has no file name",
            )
        })?
        .to_string_lossy();
    if bin_name.is_empty()
        || bin_name.starts_with('-')
        || !bin_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "invalid deployment binary name: use only ASCII alphanumeric characters, dots, underscores, and dashes",
        )
        .into());
    }
    scp_args.push("--".to_string());
    scp_args.push(local_bin.to_string());
    scp_args.push(format!("{}@{}:/app/bin/{}", cfg.user, cfg.host, bin_name));

    if !Command::new("scp").args(&scp_args).status()?.success() {
        println!(
            "{}",
            "❌ Failed to upload binary via SCP. Check SSH access and try again."
                .red()
                .bold()
        );
        std::process::exit(1);
    }
    println!("{}", "  ✅ Binary uploaded to /app/bin/.".green());
    Ok(())
}

#[cfg_attr(mutants, mutants::skip)]
pub fn execute_configure_step(
    cfg: &FoundryConfig,
    bin_name: &str,
    ssh_args: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        "⚙️  [4/5] Configuring services (env, Caddy, container)..."
            .bold()
            .yellow()
    );
    let app_port = if cfg.port.is_empty() {
        "3000"
    } else {
        &cfg.port
    };
    let caddy_site = if cfg.auto_https == "true" || cfg.auto_https.is_empty() {
        format!(
            r#"{domain} {{
    reverse_proxy localhost:{app_port}
    encode gzip zstd
    header {{
        Strict-Transport-Security "max-age=31536000; includeSubDomains; preload"
        X-Frame-Options DENY
        X-Content-Type-Options nosniff
        Referrer-Policy strict-origin-when-cross-origin
    }}
    log {{
        output file /var/log/caddy/{app_name}.log
    }}
}}"#,
            domain = cfg.domain,
            app_port = app_port,
            app_name = cfg.app_name
        )
    } else {
        format!(
            ":{app_port} {{\n    reverse_proxy localhost:{app_port}\n}}",
            app_port = app_port
        )
    };

    let env_lines = cfg
        .env_vars
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("\n");
    let configure_cmd = format!(
        r#"set -e
cat > /app/config/.env << 'ENVEOF'
{env_lines}
ENVEOF
cat > /etc/caddy/Caddyfile << 'CADDYEOF'
{caddy_site}
CADDYEOF
if ! command -v caddy &> /dev/null; then
    curl -fsSL https://caddyserver.com/install.sh | bash -s -- --
fi
chmod +x /app/bin/{bin_name}
docker rm -f rullst_{app_name} 2>/dev/null || true
pkill -f "/app/bin/{bin_name}" 2>/dev/null || true
cat > /etc/systemd/system/rullst_{app_name}.service << 'SVCEOF'
[Unit]
Description=Rullst App: {app_name}
After=network.target

[Service]
Type=simple
ExecStart=/app/bin/{bin_name}
WorkingDirectory=/app/data
EnvironmentFile=/app/config/.env
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
SVCEOF

systemctl daemon-reload
systemctl enable rullst_{app_name}
systemctl restart rullst_{app_name}
systemctl enable caddy 2>/dev/null || true
systemctl reload caddy 2>/dev/null || systemctl restart caddy 2>/dev/null || caddy reload 2>/dev/null || true
echo "✅ Services configured and started."
"#,
        env_lines = env_lines,
        caddy_site = caddy_site,
        bin_name = bin_name,
        app_name = cfg.app_name
    );

    if !run_ssh(&configure_cmd, ssh_args)? {
        println!(
            "{}",
            "⚠️  Service configuration had warnings. Verify on the server.".yellow()
        );
    } else {
        println!("{}", "  ✅ Services configured and started.".green());
    }
    Ok(())
}

#[cfg_attr(mutants, mutants::skip)]
pub fn print_deployment_summary(cfg: &FoundryConfig) {
    let app_port = if cfg.port.is_empty() {
        "3000"
    } else {
        &cfg.port
    };
    println!(
        "\n{}",
        "┌────────────────────────────────────────────────────────────┐"
            .cyan()
            .bold()
    );
    println!(
        "{}",
        format!(
            "│  🏭  Rullst Foundry — Deploying to {:>24} │",
            cfg.provider.to_uppercase()
        )
        .cyan()
        .bold()
    );
    println!(
        "{}",
        "└────────────────────────────────────────────────────────────┘"
            .cyan()
            .bold()
    );
    println!("\n  {} {}", "→ App:".bold(), cfg.app_name.cyan());
    println!("  {} {}", "→ Domain:".bold(), cfg.domain.cyan());
    println!(
        "  {} {}",
        "→ Server:".bold(),
        format!("{}@{}", cfg.user, cfg.host).cyan()
    );
    println!("  {} {}", "→ Port:".bold(), app_port.cyan());
    println!("  {} {}", "→ DB:".bold(), cfg.db_type.cyan());
    println!(
        "  {} {}",
        "→ Profile:".bold(),
        if cfg.profile.is_empty() {
            "release"
        } else {
            &cfg.profile
        }
        .cyan()
    );
    println!();
}

pub fn print_deployment_success(cfg: &FoundryConfig) {
    println!(
        "\n{}",
        "┌────────────────────────────────────────────────────────────┐"
            .green()
            .bold()
    );
    println!(
        "{}",
        "│  🎉  Rullst Foundry — Deployment Complete!                  │"
            .green()
            .bold()
    );
    println!(
        "{}",
        "└────────────────────────────────────────────────────────────┘"
            .green()
            .bold()
    );
    println!();

    let url_protocol = if cfg.auto_https == "true" || cfg.auto_https.is_empty() {
        "https"
    } else {
        "http"
    };
    println!(
        "  {} {}://{}",
        "🌐 Your app is live at:".bold(),
        url_protocol,
        cfg.domain.cyan().bold()
    );
    println!(
        "  {}",
        "📋 To check logs: ssh into your server and run:".bold()
    );
    println!(
        "     {}",
        format!("journalctl -u rullst_{} -f", cfg.app_name).magenta()
    );
    println!();
}
