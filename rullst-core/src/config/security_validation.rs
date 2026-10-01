//! Validation of the `[security]` table: browser policy, CSRF webhook
//! exemptions and the trusted-proxy settings.

use super::{ConfigError, SecurityConfig};

impl SecurityConfig {
    /// Validates browser security policy, exact-path CSRF webhook exemptions
    /// and the trusted-proxy settings.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if !matches!(self.csrf_same_site.as_str(), "Lax" | "Strict" | "None") {
            return Err(ConfigError::InvalidSecurityConfiguration(format!(
                "CSRF SameSite policy `{}` must be Lax, Strict, or None",
                self.csrf_same_site
            )));
        }
        let rendered_csp = self.csp.replace("{NONCE}", "validation-nonce");
        if self.csp.trim().is_empty() || rendered_csp.parse::<http::HeaderValue>().is_err() {
            return Err(ConfigError::InvalidSecurityConfiguration(
                "CSP must be a non-empty valid HTTP header value".to_string(),
            ));
        }
        if !matches!(
            self.coep.as_str(),
            "require-corp" | "credentialless" | "unsafe-none"
        ) {
            return Err(ConfigError::InvalidSecurityConfiguration(format!(
                "COEP policy `{}` must be require-corp, credentialless, or unsafe-none",
                self.coep
            )));
        }
        if let Some(position) = self
            .user_agent_blocklist
            .iter()
            .position(|agent| agent.trim().is_empty())
        {
            return Err(ConfigError::InvalidSecurityConfiguration(format!(
                "user_agent_blocklist entry {} is empty; it would block every request",
                position + 1
            )));
        }
        let mut unique_origins = std::collections::HashSet::new();
        for origin in &self.cors_allow_origins {
            let uri = origin.parse::<http::Uri>().map_err(|_| {
                ConfigError::InvalidSecurityConfiguration(format!(
                    "CORS origin `{origin}` must be an exact HTTP(S) origin"
                ))
            })?;
            let valid_scheme = uri
                .scheme_str()
                .is_some_and(|scheme| matches!(scheme, "http" | "https"));
            let valid_origin = valid_scheme
                && uri.authority().is_some()
                && uri.path() == "/"
                && uri.query().is_none()
                && !origin.ends_with('/')
                && !origin.contains(['@', '#', '*'])
                && origin.parse::<http::HeaderValue>().is_ok();
            if !valid_origin {
                return Err(ConfigError::InvalidSecurityConfiguration(format!(
                    "CORS origin `{origin}` must be an exact HTTP(S) origin without path, query, credentials, wildcard, or trailing slash"
                )));
            }
            if !is_serialized_origin(origin, &uri) {
                return Err(ConfigError::InvalidSecurityConfiguration(format!(
                    "CORS origin `{origin}` must be written as browsers send it: lowercase scheme and host, without the default port"
                )));
            }
            if !unique_origins.insert(origin) {
                return Err(ConfigError::InvalidSecurityConfiguration(format!(
                    "duplicate CORS origin `{origin}`"
                )));
            }
        }
        let mut unique_paths = std::collections::HashSet::new();
        for path in &self.csrf_signed_webhook_paths {
            let is_exact_absolute_path = path.starts_with('/')
                && path.len() > 1
                && !path.contains(['?', '#', '*', '{', '}', ':'])
                && !path
                    .split('/')
                    .any(|segment| segment == ".." || segment == ".");
            if !is_exact_absolute_path {
                return Err(ConfigError::InvalidSecurityConfiguration(format!(
                    "CSRF signed-webhook exemption `{path}` must be an exact absolute path"
                )));
            }
            if !unique_paths.insert(path) {
                return Err(ConfigError::InvalidSecurityConfiguration(format!(
                    "duplicate CSRF signed-webhook exemption `{path}`"
                )));
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        crate::security::TrustedProxyConfig::from_security_config(self)
            .map_err(|error| ConfigError::InvalidSecurityConfiguration(error.to_string()))?;
        Ok(())
    }
}

/// Whether `origin` is byte-for-byte the `Origin` a browser sends for `uri`:
/// lowercase scheme and host and no default port. CORS matching compares the
/// configured value with that header exactly, so any other spelling never
/// matches.
fn is_serialized_origin(origin: &str, uri: &http::Uri) -> bool {
    let (Some(scheme), Some(host)) = (uri.scheme_str(), uri.host()) else {
        return false;
    };
    let default_port = match scheme {
        "http" => 80,
        "https" => 443,
        _ => return false,
    };
    let port = match uri.port_u16() {
        Some(port) if port == default_port => return false,
        Some(port) => format!(":{port}"),
        None => String::new(),
    };
    origin
        == format!(
            "{}://{}{port}",
            scheme.to_ascii_lowercase(),
            host.to_ascii_lowercase()
        )
}
