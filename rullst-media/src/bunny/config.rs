use crate::{LibraryId, MediaError as Error, ProviderMode, Reference};
use zeroize::Zeroizing;

/// Separate write/read-only/embed/CDN keys. Debug and errors never expose them.
pub struct BunnyCredentials {
    pub(super) api: Zeroizing<String>,
    pub(super) webhook: Zeroizing<String>,
    pub(super) embed: Zeroizing<String>,
    pub(super) cdn: Zeroizing<String>,
    pub(super) mode: ProviderMode,
}
impl BunnyCredentials {
    pub fn new(
        api: impl Into<String>,
        webhook: impl Into<String>,
        embed: impl Into<String>,
        cdn: impl Into<String>,
    ) -> Result<Self, Error> {
        let values = [api.into(), webhook.into(), embed.into(), cdn.into()].map(Zeroizing::new);
        let modes = values.each_ref().map(|v| {
            if v.is_empty() || v.starts_with("mock_") {
                ProviderMode::Offline
            } else if v.starts_with("fixture_") {
                ProviderMode::ProtocolFixture
            } else {
                ProviderMode::RemoteUnvalidated
            }
        });
        if modes.iter().any(|m| m != &modes[0])
            || values.iter().any(|v| {
                v.len() > 512
                    || (modes[0] != ProviderMode::Offline
                        && (v.len() < 16 || !v.bytes().all(|b| b.is_ascii_graphic())))
            })
        {
            return Err(Error::Configuration);
        }
        let [api, webhook, embed, cdn] = values;
        Ok(Self {
            api,
            webhook,
            embed,
            cdn,
            mode: modes[0],
        })
    }
}
impl std::fmt::Debug for BunnyCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BunnyCredentials([REDACTED])")
    }
}

/// Explicit operator assertion, NOT a live verification of library/CDN settings.
/// Review original-file, thumbnail, preview and alternative rendition exposure.
#[derive(Debug, Clone, Copy)]
pub struct PrivateDelivery {
    _private: (),
}
impl PrivateDelivery {
    pub fn configured(
        embed_tokens: bool,
        cdn_tokens: bool,
        direct_files_protected: bool,
    ) -> Result<Self, Error> {
        if !embed_tokens || !cdn_tokens || !direct_files_protected {
            return Err(Error::Configuration);
        }
        Ok(Self { _private: () })
    }
}

#[derive(Debug)]
pub struct BunnyConfig {
    pub(super) library: LibraryId,
    pub(super) environment: Reference,
    pub(super) cdn_host: String,
    pub(super) credentials: BunnyCredentials,
}
impl BunnyConfig {
    pub fn new(
        library: LibraryId,
        environment: Reference,
        cdn_host: impl Into<String>,
        credentials: BunnyCredentials,
        _delivery: PrivateDelivery,
    ) -> Result<Self, Error> {
        let cdn_host = cdn_host.into();
        let label = cdn_host
            .strip_suffix(".b-cdn.net")
            .ok_or(Error::Configuration)?;
        if label.is_empty()
            || label.len() > 63
            || label.starts_with('-')
            || label.ends_with('-')
            || !label
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            return Err(Error::Configuration);
        }
        Ok(Self {
            library,
            environment,
            cdn_host,
            credentials,
        })
    }
}
