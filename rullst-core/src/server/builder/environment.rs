//! Process-environment, `.env` and listen-address resolution helpers for the
//! server builder.

use super::ServerError;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

/// The socket address for a `HOST` value: `host:port` as before, a bare IPv6
/// address such as `::`, or `localhost` (`127.0.0.1`). Host names are never
/// resolved.
pub(super) fn listen_address(host: &str, port: u16) -> Option<SocketAddr> {
    if let Ok(address) = format!("{host}:{port}").parse() {
        return Some(address);
    }
    if host.eq_ignore_ascii_case("localhost") {
        return Some(SocketAddr::from(([127, 0, 0, 1], port)));
    }
    host.parse::<std::net::IpAddr>()
        .ok()
        .map(|ip| SocketAddr::new(ip, port))
}

/// Reads an optional process environment variable for database URL resolution.
///
/// An absent variable is `None`; a non-Unicode value is a configuration error
/// that names the variable but never contains its value. Hidden support API for
/// first-party tools such as Rullst Studio, not a stable extension point.
#[doc(hidden)]
pub fn read_optional_environment_variable(name: &str) -> Result<Option<String>, ServerError> {
    match std::env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => Err(ServerError::Configuration(format!(
            "{name} is not valid Unicode"
        ))),
    }
}

/// The panic console and `/_rullst/explain`/`/_rullst/autofix` are mounted only
/// in debug builds running in Development, like hot reload and the generation
/// probe. An unset environment resolves to Development, so a release binary must
/// not rely on the environment alone.
pub(in crate::server) fn development_console_enabled(
    debug_build: bool,
    environment: crate::config::Environment,
) -> bool {
    debug_build && environment.allows_development_tools()
}

pub(super) fn resolve_hot_reload_token() -> Result<Arc<str>, ServerError> {
    let token = read_optional_environment_variable("RULLST_HMR_TOKEN")?.ok_or_else(|| {
        ServerError::HotReloadConfiguration(
            "RULLST_HMR_TOKEN is missing; start hot reload through `cargo rullst dev` or `cargo rullst dash`"
                .to_string(),
        )
    })?;
    if token.len() != 64 || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ServerError::HotReloadConfiguration(
            "RULLST_HMR_TOKEN must contain exactly 64 hexadecimal characters".to_string(),
        ));
    }
    Ok(Arc::from(token))
}

pub(super) fn resolve_environment(
    config: &crate::config::RullstConfig,
    dotenv: &HashMap<String, String>,
) -> Result<crate::config::Environment, ServerError> {
    crate::server::project_settings::resolve_environment(dotenv, config.app.env.as_deref())
}

/// Parses dotenv content with errors that never contain file content: dotenvy's
/// own parse error embeds the unparsed remainder, which can include secrets.
pub(in crate::server) fn parse_dotenv(
    content: &str,
) -> Result<HashMap<String, String>, ServerError> {
    crate::config::parse_dotenv_entries(content).map_err(ServerError::Configuration)
}
