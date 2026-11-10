use crate::{LibraryId, MediaError as Error, ProviderMode, Reference};
use zeroize::Zeroizing;

/// Separate write/read-only/embed/CDN keys. Debug and errors never expose them.
pub struct BunnyCredentials {
    pub(super) api: Zeroizing<String>,
    pub(super) webhook: Zeroizing<String>,
    pub(super) embed: Zeroizing<String>,
    pub(super) cdn: Zeroizing<String>,
    pub(super) mode: ProviderMode,
    /// False for [`BunnyCredentials::upload_only`]: no webhook/embed/CDN keys.
    pub(super) delivery_keys: bool,
}
impl BunnyCredentials {
    pub fn new(
        api: impl Into<String>,
        webhook: impl Into<String>,
        embed: impl Into<String>,
        cdn: impl Into<String>,
    ) -> Result<Self, Error> {
        let values = [api.into(), webhook.into(), embed.into(), cdn.into()].map(Zeroizing::new);
        let mode = checked_mode(&values)?;
        let [api, webhook, embed, cdn] = values;
        Ok(Self {
            api,
            webhook,
            embed,
            cdn,
            mode,
            delivery_keys: true,
        })
    }

    /// v13: only the library API key, for an application that creates,
    /// uploads, reads, lists, renames and deletes videos but signs no playback
    /// links and receives no webhooks through this crate. The key follows the
    /// [`new`](Self::new) rules (`mock_`/empty keys select offline mode). Use
    /// it with [`BunnyConfig::upload_only`]; [`BunnyConfig::new`] refuses it.
    pub fn upload_only(api: impl Into<String>) -> Result<Self, Error> {
        let values = [Zeroizing::new(api.into())];
        let mode = checked_mode(&values)?;
        let [api] = values;
        Ok(Self {
            api,
            webhook: Zeroizing::new(String::new()),
            embed: Zeroizing::new(String::new()),
            cdn: Zeroizing::new(String::new()),
            mode,
            delivery_keys: false,
        })
    }
}

/// The common mode of `values`; remote keys need 16–512 printable bytes.
fn checked_mode(values: &[Zeroizing<String>]) -> Result<ProviderMode, Error> {
    let modes: Vec<ProviderMode> = values
        .iter()
        .map(|v| {
            if v.is_empty() || v.starts_with("mock_") {
                ProviderMode::Offline
            } else if v.starts_with("fixture_") {
                ProviderMode::ProtocolFixture
            } else {
                ProviderMode::RemoteUnvalidated
            }
        })
        .collect();
    let mode = *modes.first().ok_or(Error::Configuration)?;
    if modes.iter().any(|m| m != &mode)
        || values.iter().any(|v| {
            v.len() > 512
                || (mode != ProviderMode::Offline
                    && (v.len() < 16 || !v.bytes().all(|b| b.is_ascii_graphic())))
        })
    {
        return Err(Error::Configuration);
    }
    Ok(mode)
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
    /// False for [`BunnyConfig::upload_only`]: playback and notifications are
    /// `Unsupported`.
    pub(super) delivery: bool,
}
impl BunnyConfig {
    /// A complete configuration: playback grants and webhook verification
    /// need all four keys of [`BunnyCredentials::new`].
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
        if !credentials.delivery_keys
            || label.is_empty()
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
            delivery: true,
        })
    }

    /// v13: video management without delivery. Needs no CDN host and no
    /// [`PrivateDelivery`] assertion; `BunnyStream` then signs upload grants
    /// and calls the library API with the API key only, while `playback` and
    /// `verify_notification` return [`MediaError::Unsupported`](Error::Unsupported).
    /// Accepts upload-only or complete credentials; only the API key is used.
    /// Its provider binding differs from a complete configuration's, so a
    /// `sqlite` store created with one refuses the other.
    pub fn upload_only(
        library: LibraryId,
        environment: Reference,
        credentials: BunnyCredentials,
    ) -> Result<Self, Error> {
        Ok(Self {
            library,
            environment,
            cdn_host: String::new(),
            credentials,
            delivery: false,
        })
    }
}
