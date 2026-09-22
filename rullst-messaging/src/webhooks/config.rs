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
        serde_json::to_vec(&serde_json::json!({"version":1,"namespace":self.namespace.as_str(),
            "destination":self.destination.url.as_str(),"loopback":self.destination.loopback,
            "retained":self.retained,"window_ms":self.window_ms,"key_id":key.id(),
            "key_binding":key.binding(),"production":self.production,"offline":key.offline,"test_root":self.destination.test_root}))
            .map_err(|_|WebhookError::Configuration)
    }
}
impl std::fmt::Debug for WebhookConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WebhookConfig([REDACTED])")
    }
}
