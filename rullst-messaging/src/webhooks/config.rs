use super::*;
use crate::Namespace;
use reqwest::Url;
use std::net::IpAddr;

/// Exact server-approved endpoint. Never derive approval from a request URL.
#[derive(Clone)]
pub struct WebhookDestination {
    pub(super) url: Url,
    pub(super) loopback: bool,
    pub(super) test_root: Option<Vec<u8>>,
}
impl WebhookDestination {
    pub fn approved_https(value: impl Into<String>) -> Result<Self> {
        Self::parse(value.into(), false)
    }
    /// Explicit test-only policy; accepts literal loopback HTTP(S), not DNS names.
    pub fn loopback_test(value: impl Into<String>) -> Result<Self> {
        Self::parse(value.into(), true)
    }
    fn parse(value: String, loopback: bool) -> Result<Self> {
        if value.len() > 2048
            || value
                .bytes()
                .any(|byte| byte.is_ascii_control() || matches!(byte, b' ' | b'\\'))
        {
            return Err(WebhookError::DestinationDenied);
        }
        let url = Url::parse(&value).map_err(|_| WebhookError::DestinationDenied)?;
        if url.port() == Some(0)
            || !url.username().is_empty()
            || url.password().is_some()
            || url.fragment().is_some()
            || (url.scheme() != "https" && !(loopback && url.scheme() == "http"))
        {
            return Err(WebhookError::DestinationDenied);
        }
        let host = url
            .host_str()
            .ok_or(WebhookError::DestinationDenied)?
            .trim_matches(['[', ']']);
        if host.ends_with('.')
            || host.eq_ignore_ascii_case("localhost")
            || host.ends_with(".localhost")
        {
            return Err(WebhookError::DestinationDenied);
        }
        if loopback && !host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback()) {
            return Err(WebhookError::DestinationDenied);
        }
        Ok(Self {
            url,
            loopback,
            test_root: None,
        })
    }
    /// Adds an owned fixture CA. Only explicit loopback development endpoints permit it.
    pub fn with_test_root_pem(mut self, pem: impl Into<Vec<u8>>) -> Result<Self> {
        let pem = pem.into();
        if !self.loopback || self.url.scheme() != "https" || pem.len() > 8192 {
            return Err(WebhookError::InvalidInput("test TLS root"));
        }
        reqwest::Certificate::from_pem(&pem)
            .map_err(|_| WebhookError::InvalidInput("test TLS root"))?;
        self.test_root = Some(pem);
        Ok(self)
    }
}
impl std::fmt::Debug for WebhookDestination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebhookDestination")
            .field("loopback_test", &self.loopback)
            .finish_non_exhaustive()
    }
}
#[derive(Clone)]
pub struct WebhookConfig {
    pub(super) namespace: Namespace,
    pub(super) destination: WebhookDestination,
    pub(super) retained: usize,
    pub(super) window_ms: i64,
    pub(super) production: bool,
}
impl WebhookConfig {
    /// Reserves one extra internal control record. Delivery is limited to ten attempts
    /// over one day; at most 99,999 application events can be retained.
    pub fn new(
        namespace: impl Into<String>,
        destination: WebhookDestination,
        retained: usize,
    ) -> Result<Self> {
        let namespace =
            Namespace::try_new(namespace).map_err(|_| WebhookError::InvalidInput("namespace"))?;
        if !(1..=99_999).contains(&retained) {
            return Err(WebhookError::InvalidInput("retained events"));
        }
        Ok(Self {
            namespace,
            destination,
            retained,
            window_ms: 86_400_000,
            production: false,
        })
    }
    pub fn with_delivery_window(mut self, window: Duration) -> Result<Self> {
        let ms = i64::try_from(window.as_millis())
            .map_err(|_| WebhookError::InvalidInput("delivery window"))?;
        if !(60_000..=7 * 86_400_000).contains(&ms) {
            return Err(WebhookError::InvalidInput("delivery window"));
        }
        self.window_ms = ms;
        Ok(self)
    }
    pub fn namespace(&self) -> &str {
        self.namespace.as_str()
    }
    /// Rejects development destinations now and offline keys again when opening.
    pub fn require_production(mut self) -> Result<Self> {
        if self.destination.loopback {
            return Err(WebhookError::Configuration);
        }
        self.production = true;
        Ok(self)
    }
    pub(super) fn binding(&self, key: &WebhookSigningKey) -> Result<Vec<u8>> {
        let key_binding = key.binding();
        serde_json::to_vec(&ConfigurationBinding {
            destination: self.destination.url.as_str(),
            key_binding: &key_binding,
            key_id: key.id(),
            loopback: self.destination.loopback,
            namespace: self.namespace.as_str(),
            offline: key.offline,
            production: self.production,
            retained: self.retained,
            test_root: self.destination.test_root.as_deref(),
            version: 1,
            window_ms: self.window_ms,
        })
        .map_err(|_| WebhookError::Configuration)
    }
}

/// Exact bytes of the immutable control record, compared by fingerprint on
/// every reopen. Fields are declared in sorted order, the layout that
/// `serde_json::json!` produced without `preserve_order`, so the bytes do not
/// depend on serde_json features enabled elsewhere in the dependency graph.
#[derive(serde::Serialize)]
struct ConfigurationBinding<'value> {
    destination: &'value str,
    key_binding: &'value [u8],
    key_id: &'value str,
    loopback: bool,
    namespace: &'value str,
    offline: bool,
    production: bool,
    retained: usize,
    test_root: Option<&'value [u8]>,
    version: u8,
    window_ms: i64,
}
impl std::fmt::Debug for WebhookConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WebhookConfig([REDACTED])")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configuration_binding_bytes_have_a_fixed_layout() {
        let destination = WebhookDestination::loopback_test("http://127.0.0.1:8080/hook").unwrap();
        let config = WebhookConfig::new("layout", destination, 16).unwrap();
        let key = WebhookSigningKey::new("fixture", "mock_layout").unwrap();
        let binding = String::from_utf8(config.binding(&key).unwrap()).unwrap();
        let key_binding = key
            .binding()
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(
            binding,
            format!(
                "{{\"destination\":\"http://127.0.0.1:8080/hook\",\"key_binding\":[{key_binding}],\
                \"key_id\":\"fixture\",\"loopback\":true,\"namespace\":\"layout\",\"offline\":true,\
                \"production\":false,\"retained\":16,\"test_root\":null,\"version\":1,\
                \"window_ms\":86400000}}"
            )
        );
    }
}
