// src/generators/foundry/service.rs — Unprivileged systemd service identity and sandbox.

use super::config::FoundryConfig;

/// Linux user names are limited to 32 bytes; `useradd` expects lowercase.
const MAX_ACCOUNT_LEN: usize = 32;

/// Returns the dedicated system account that runs the application.
///
/// It is `rullst-<app>` when that is already a valid lowercase name. Otherwise
/// a stable FNV-1a digest of the exact application name keeps distinct
/// applications on distinct accounts after lowercasing or truncation.
pub(super) fn service_account(app_name: &str) -> String {
    let lowercase = app_name.to_ascii_lowercase();
    let account = format!("rullst-{lowercase}");
    if lowercase == app_name && account.len() <= MAX_ACCOUNT_LEN {
        return account;
    }
    let digest = app_name
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
        });
    let prefix = lowercase.chars().take(16).collect::<String>();
    format!("rullst-{prefix}-{:08x}", digest & 0xffff_ffff)
}

/// Creates the service account and hands it the only writable application path.
///
/// Binaries and configuration stay root-owned. Existing data written by an
/// earlier root-run deployment is re-owned without following symlinks.
pub(super) fn render_account_setup(cfg: &FoundryConfig) -> String {
    let account = service_account(&cfg.app_name);
    format!(
        r#"command -v useradd > /dev/null 2>&1
if ! id -u {account} > /dev/null 2>&1; then
    useradd --system --user-group --no-create-home --home-dir /opt/rullst/{app_name}/data --shell "$(command -v nologin || echo /bin/false)" {account}
fi
install -d -m 0750 -o {account} -g {account} /opt/rullst/{app_name}/data
chmod 0750 /opt/rullst/{app_name}/data
chown -R -h {account}:{account} /opt/rullst/{app_name}/data"#,
        app_name = cfg.app_name
    )
}

/// Renders the `[Service]` identity and sandbox directives.
pub(super) fn render_unit_hardening(cfg: &FoundryConfig) -> String {
    let account = service_account(&cfg.app_name);
    let privileged_port = cfg
        .app_port()
        .parse::<u16>()
        .is_ok_and(|port| (1..1024).contains(&port));
    let capabilities = if privileged_port {
        "CapabilityBoundingSet=CAP_NET_BIND_SERVICE\nAmbientCapabilities=CAP_NET_BIND_SERVICE"
    } else {
        "CapabilityBoundingSet="
    };
    format!(
        r#"User={account}
Group={account}
UMask=0027
NoNewPrivileges=yes
{capabilities}
PrivateTmp=yes
PrivateDevices=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/opt/rullst/{app_name}/data
ProtectKernelTunables=yes
ProtectKernelModules=yes
ProtectControlGroups=yes
RestrictSUIDSGID=yes
LockPersonality=yes"#,
        app_name = cfg.app_name
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_accounts_are_valid_bounded_and_distinct() {
        assert_eq!(service_account("demo"), "rullst-demo");
        let long = "a".repeat(64);
        for name in [
            "Demo",
            "demo_App-1",
            long.as_str(),
            "x123456789012345678901234",
        ] {
            let account = service_account(name);
            assert!(account.len() <= MAX_ACCOUNT_LEN, "{account}");
            assert!(account.starts_with("rullst-"));
            assert!(
                account.bytes().all(|byte| byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'-' | b'_')),
                "{account}"
            );
        }
        assert_ne!(service_account("Demo"), service_account("demo"));
        assert_ne!(
            service_account(&format!("{long}a")[1..]),
            service_account(&format!("{long}b")[1..])
        );
    }
}
