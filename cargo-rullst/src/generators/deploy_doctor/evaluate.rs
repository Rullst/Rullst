use super::{Report, input::Snapshot};
use rullst_core::config::{Environment, SecurityConfig};

pub(super) fn inspect(snapshot: &Snapshot, report: &mut Report) {
    report.add(
        "configuration_source",
        "PASS",
        if snapshot.config_present {
            "Selected TOML parsed; this is a local snapshot, not running application configuration."
        } else {
            "No local Rullst.toml; inspecting Core configuration defaults."
        },
    );
    if snapshot.unknown_config {
        report.add("unrecognized_configuration", "REVIEW", "Some fields are not consumed by Core configuration. Check for typos or application-specific settings; their values are not inspected.");
    }
    if snapshot.config.validate().is_err() {
        report.add("security_configuration", "FAIL", "Core rejected the browser security configuration. Review CSP/header syntax, COEP, SameSite, exact CORS origins and exact signed-webhook paths.");
    } else {
        report.add("security_configuration", "PASS", "Core accepted configuration syntax. Mounted middleware and effective browser behavior require runtime tests.");
    }
    if snapshot.config.app.port == Some(0) {
        report.add("configured_port", "FAIL", "The TOML port is zero. Configure a stable listening port; runtime port overrides are not inspected.");
    }
    let security = &snapshot.config.security;
    if security.csp != SecurityConfig::default().csp {
        report.add("custom_csp", "REVIEW", "Review the custom CSP against application resources and checkout origins; syntax validation does not establish policy strength.");
    }
    if security.coep == "unsafe-none"
        || security.csrf_same_site == "None"
        || security.cors_allow_credentials
    {
        report.add("browser_policy_exceptions", "REVIEW", "Review the explicit browser-policy exceptions and credentialed cross-origin behavior in a browser test.");
    }
    if !security.csrf_signed_webhook_paths.is_empty() {
        report.add("signed_webhook_exemptions", "REVIEW", "Every configured CSRF exemption needs mandatory signature verification on that exact route; this command does not inspect routing.");
    }
    if !snapshot.environment_selected {
        report.add("environment_source", "NOT_INSPECTED", "Choose --env-file for a literal snapshot or --process-env for the three allowlisted process variables. No dotenv or process values were read.");
        return;
    }
    report.inspection_complete = true;
    let get = |key| snapshot.environment.get(key).map(String::as_str);
    let env = Environment::resolve(
        get("RULLST_ENV"),
        get("APP_ENV"),
        snapshot.config.app.env.as_deref(),
    );
    let expected = if snapshot.staging {
        Environment::Staging
    } else {
        Environment::Production
    };
    if env.is_ok_and(|env| env == expected) {
        report.add("environment_target", "PASS", "Selected snapshot resolves to the requested target using Core precedence; runtime overrides are not inspected.");
    } else {
        report.add("environment_target", "FAIL", "Set RULLST_ENV to the requested --target (production or staging). It takes precedence over APP_ENV and TOML app.env; blank or invalid values are rejected.");
    }
    if let (Some(first), Some(second)) = (get("RULLST_ENV"), get("APP_ENV"))
        && Environment::resolve(Some(first), None, None).ok()
            != Environment::resolve(Some(second), None, None).ok()
    {
        report.add("legacy_environment_conflict", "REVIEW", "RULLST_ENV and legacy APP_ENV disagree. Remove the stale variable to avoid different application components selecting different modes.");
    }
    // Deliberately an obvious-mistake check, not a duplicate implementation of
    // Auth's full key policy or a measurement of cryptographic randomness.
    if get("APP_KEY").is_none_or(obviously_bad_key) {
        report.add("application_key_obvious_errors", "FAIL", "Provide an explicit APP_KEY with at least 32 bytes from a cryptographic generator. Empty, example, mock, control-character or single-symbol keys are rejected. Legacy TOML key fallback is not inspected.");
    } else {
        report.add("application_key_obvious_errors", "PASS", "No checked APP_KEY placeholder/length/control/single-symbol mistake was found. Auth validation, cryptographic generation, custody and rotation remain required.");
    }
}

fn obviously_bad_key(key: &str) -> bool {
    let normalized = key.trim().to_ascii_lowercase();
    key.len() < 32
        || key.chars().any(char::is_control)
        || [
            "mock_",
            "change_me",
            "changeme",
            "replace_",
            "your_",
            "example_",
        ]
        .iter()
        .any(|prefix| normalized.starts_with(prefix))
        || key.bytes().all(|byte| Some(byte) == key.bytes().next())
}
