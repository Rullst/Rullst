// src/generators/foundry/deploy.rs — SSH deployment pipeline steps.

use super::config::FoundryConfig;
use colored::*;
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

fn expand_ssh_key(path: &str) -> String {
    let Some(relative) = path.strip_prefix("~/") else {
        return path.to_string();
    };
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(|home| format!("{home}/{relative}"))
        .unwrap_or_else(|_| path.to_string())
}

pub fn get_ssh_base_args(cfg: &FoundryConfig) -> Vec<String> {
    let mut args = Vec::new();
    let ssh_port_num = if cfg.ssh_port.is_empty() {
        "22"
    } else {
        &cfg.ssh_port
    };
    args.push("-p".to_string());
    args.push(ssh_port_num.to_string());

    let ssh_key_expanded = expand_ssh_key(&cfg.ssh_key);
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

fn run_ssh_script(script: &str, base_args: &[String]) -> Result<bool, Box<dyn std::error::Error>> {
    let mut child = Command::new("ssh")
        .args(base_args)
        .arg(
            "if [ \"$(id -u)\" -eq 0 ]; then exec sh -s; elif command -v sudo >/dev/null 2>&1; then exec sudo -n sh -s; else echo 'root or passwordless sudo is required' >&2; exit 1; fi",
        )
        .stdin(Stdio::piped())
        .spawn()?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| std::io::Error::other("could not open SSH script input"))?;
    stdin.write_all(script.as_bytes())?;
    drop(stdin);
    Ok(child.wait()?.success())
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
        return Err(std::io::Error::other("release build failed; deployment aborted").into());
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

    let cargo_toml_content = fs::read_to_string("Cargo.toml")?;
    let cargo_manifest = toml::from_str::<toml::Value>(&cargo_toml_content)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    let bin_name = cargo_manifest
        .get("package")
        .and_then(|package| package.get("name"))
        .and_then(toml::Value::as_str)
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Cargo.toml must contain a string package.name",
            )
        })?
        .to_string();

    Ok(format!("target/{}/{}", bin_subdir, bin_name))
}

pub fn execute_provision_step(
    cfg: &FoundryConfig,
    ssh_args: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        "🖥️  [2/5] Provisioning server environment..."
            .bold()
            .yellow()
    );
    let provision_cmd = render_provision_command(cfg);

    if !run_ssh_script(&provision_cmd, ssh_args)? {
        return Err(std::io::Error::other(
            "remote provisioning failed; deployment state must be inspected before retrying",
        )
        .into());
    }
    println!("{}", "  ✅ Server provisioned.".green());
    Ok(())
}

fn render_provision_command(cfg: &FoundryConfig) -> String {
    format!(
        r#"set -e
command -v curl > /dev/null 2>&1
command -v systemctl > /dev/null 2>&1
command -v caddy > /dev/null 2>&1
install -d -m 0755 /opt/rullst /opt/rullst/{app_name} /opt/rullst/{app_name}/data /opt/rullst/{app_name}/bin /var/log/caddy
install -d -m 0700 /opt/rullst/{app_name}/config
echo "✅ Server environment ready.""#,
        app_name = cfg.app_name
    )
}

#[cfg_attr(mutants, mutants::skip)]
pub fn execute_upload_step(
    cfg: &FoundryConfig,
    local_bin: &str,
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

    let ssh_key_expanded = expand_ssh_key(&cfg.ssh_key);
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
    scp_args.push(format!(
        "{}@{}:/tmp/rullst_{}.upload",
        cfg.user, cfg.host, cfg.app_name
    ));

    if !Command::new("scp").args(&scp_args).status()?.success() {
        return Err(std::io::Error::other(
            "binary upload failed; check SSH access and remote partial state",
        )
        .into());
    }
    println!(
        "{}",
        "  ✅ Binary uploaded to a remote staging path.".green()
    );
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
        "⚙️  [4/5] Configuring the systemd service and Caddy..."
            .bold()
            .yellow()
    );
    let configure_cmd = render_configure_command(cfg, bin_name);

    if !run_ssh_script(&configure_cmd, ssh_args)? {
        return Err(std::io::Error::other(
            "remote service configuration failed; deployment is incomplete",
        )
        .into());
    }
    println!("{}", "  ✅ Services configured and started.".green());
    Ok(())
}

fn render_configure_command(cfg: &FoundryConfig, bin_name: &str) -> String {
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
            ":80 {{\n    reverse_proxy localhost:{app_port}\n}}",
            app_port = app_port
        )
    };

    let env_lines = cfg
        .env_vars
        .iter()
        .map(|(key, value)| format!("{key}=\"{}\"", escape_systemd_env_value(value)))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        r#"set -e
umask 077
staged_binary=/tmp/rullst_{app_name}.upload
test -f "$staged_binary"
command -v systemctl > /dev/null 2>&1
if ! command -v caddy > /dev/null 2>&1; then
    echo "Caddy is required but was not found; install it from a reviewed package source before retrying" >&2
    exit 1
fi
cat > /opt/rullst/{app_name}/config/.env.next << 'ENVEOF'
{env_lines}
ENVEOF
chmod 600 /opt/rullst/{app_name}/config/.env.next
cat > /etc/caddy/Caddyfile.rullst-next << 'CADDYEOF'
{caddy_site}
CADDYEOF
caddy validate --config /etc/caddy/Caddyfile.rullst-next
cat > /etc/systemd/system/rullst_{app_name}.service.next << 'SVCEOF'
[Unit]
Description=Rullst App: {app_name}
After=network.target

[Service]
Type=simple
ExecStart=/opt/rullst/{app_name}/bin/{bin_name}
WorkingDirectory=/opt/rullst/{app_name}/data
EnvironmentFile=/opt/rullst/{app_name}/config/.env
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
SVCEOF

install -m 0755 "$staged_binary" /opt/rullst/{app_name}/bin/{bin_name}.next
rm -f "$staged_binary"
if [ -f /opt/rullst/{app_name}/bin/{bin_name} ]; then
    mv -f /opt/rullst/{app_name}/bin/{bin_name} /opt/rullst/{app_name}/bin/{bin_name}.previous
fi
if [ -f /opt/rullst/{app_name}/config/.env ]; then
    cp -p /opt/rullst/{app_name}/config/.env /opt/rullst/{app_name}/config/.env.previous
fi
if [ -f /etc/caddy/Caddyfile ]; then
    cp -p /etc/caddy/Caddyfile /etc/caddy/Caddyfile.previous
fi
if [ -f /etc/systemd/system/rullst_{app_name}.service ]; then
    cp -p /etc/systemd/system/rullst_{app_name}.service /etc/systemd/system/rullst_{app_name}.service.previous
fi
mv -f /opt/rullst/{app_name}/bin/{bin_name}.next /opt/rullst/{app_name}/bin/{bin_name}
mv -f /opt/rullst/{app_name}/config/.env.next /opt/rullst/{app_name}/config/.env
mv -f /etc/caddy/Caddyfile.rullst-next /etc/caddy/Caddyfile
mv -f /etc/systemd/system/rullst_{app_name}.service.next /etc/systemd/system/rullst_{app_name}.service
systemctl daemon-reload
systemctl enable rullst_{app_name}
systemctl restart rullst_{app_name}
systemctl enable caddy
systemctl reload caddy || systemctl restart caddy
echo "✅ Services configured and started."
"#,
        env_lines = env_lines,
        caddy_site = caddy_site,
        bin_name = bin_name,
        app_name = cfg.app_name
    )
}

fn escape_systemd_env_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
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
        "│  Rullst Foundry — Local health check passed                 │"
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
        "Candidate public URL (verify DNS, TLS, and external reachability):".bold(),
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

#[cfg(test)]
mod tests;
