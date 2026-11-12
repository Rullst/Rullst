//! Constructors for application-owned adapters: an [`UploadGrant`] or a
//! [`PlaybackGrant`] minted outside this crate (13.0.0-alpha.5).

use crate::contracts::checked_time;
use crate::{
    MediaError, PlaybackGrant, ProviderBinding, ProviderMode, UploadDeclaration, UploadGrant,
    UploadProtocol, VideoId,
};

/// Longest grant URL or endpoint accepted, in bytes.
const MAX_URL_BYTES: usize = 2_048;
/// Longest upload signature accepted, in bytes.
const MAX_SIGNATURE_BYTES: usize = 4_096;

impl UploadGrant {
    /// An upload grant created by an application-owned [`VideoProvider`](crate::VideoProvider).
    /// Since 13.0.0-alpha.5.
    ///
    /// The grant takes the library and mode of the adapter's `binding`.
    /// `endpoint` is an absolute `https://` URL (plain `http://` only in
    /// [`ProviderMode::ProtocolFixture`]) of at most 2 KiB, without
    /// credentials, fragment, spaces or control characters. `signature` is the
    /// adapter's opaque upload credential: 1 to 4096 visible ASCII
    /// characters, redacted from `Debug`. `expires_at` is a UTC Unix second
    /// between 1970 and 9999. Anything else is [`MediaError::InvalidInput`].
    /// Add a host-validated declaration with
    /// [`with_declaration`](Self::with_declaration).
    pub fn new(
        binding: &ProviderBinding,
        video: VideoId,
        protocol: UploadProtocol,
        endpoint: impl Into<String>,
        signature: impl Into<String>,
        expires_at: i64,
    ) -> Result<Self, MediaError> {
        let (endpoint, signature) = (endpoint.into(), signature.into());
        let signed = (1..=MAX_SIGNATURE_BYTES).contains(&signature.len())
            && signature.bytes().all(|byte| byte.is_ascii_graphic());
        if !url_valid(&endpoint, binding.mode) || !signed {
            return Err(MediaError::InvalidInput);
        }
        Ok(Self {
            endpoint,
            library: binding.library,
            video,
            expires_at: grant_expiry(expires_at)?,
            signature,
            mode: binding.mode,
            protocol,
            content_type: None,
            content_length: None,
        })
    }

    /// Records the declared content type and exact length the grant's
    /// signature binds, as [`VideoProvider::upload_declared`](crate::VideoProvider::upload_declared)
    /// requires. Since 13.0.0-alpha.5.
    pub fn with_declaration(mut self, declaration: &UploadDeclaration) -> Self {
        self.content_type = Some(declaration.content_type().to_owned());
        self.content_length = Some(declaration.length());
        self
    }
}

impl PlaybackGrant {
    /// A playback grant created by an application-owned
    /// [`VideoProvider`](crate::VideoProvider). Since 13.0.0-alpha.5.
    ///
    /// The grant takes the mode of the adapter's `binding`. `url` is the
    /// bearer playback URL, with the rules of an [`UploadGrant::new`]
    /// endpoint; it is redacted from `Debug` and read with
    /// [`expose_url`](Self::expose_url). `expires_at` is a UTC Unix second
    /// between 1970 and 9999. Anything else is [`MediaError::InvalidInput`].
    pub fn new(
        binding: &ProviderBinding,
        url: impl Into<String>,
        expires_at: i64,
    ) -> Result<Self, MediaError> {
        let url = url.into();
        if !url_valid(&url, binding.mode) {
            return Err(MediaError::InvalidInput);
        }
        Ok(Self {
            url,
            expires_at: grant_expiry(expires_at)?,
            mode: binding.mode,
        })
    }
}

fn grant_expiry(expires_at: i64) -> Result<i64, MediaError> {
    checked_time(expires_at).map_err(|_| MediaError::InvalidInput)
}

/// HTTPS (HTTP only for the loopback protocol fixture), a host, no
/// credentials or fragment, visible ASCII only, at most 2 KiB.
pub(crate) fn url_valid(url: &str, mode: ProviderMode) -> bool {
    let rest = url.strip_prefix("https://").or_else(|| {
        url.strip_prefix("http://")
            .filter(|_| mode == ProviderMode::ProtocolFixture)
    });
    let Some(rest) = rest else {
        return false;
    };
    let authority = rest.split(['/', '?']).next().unwrap_or_default();
    url.len() <= MAX_URL_BYTES
        && !authority.is_empty()
        && !authority.contains('@')
        && !url.contains('#')
        && url.bytes().all(|byte| byte.is_ascii_graphic())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::{LibraryId, Reference};

    fn binding(mode: ProviderMode) -> ProviderBinding {
        ProviderBinding {
            library: LibraryId::new(7).unwrap(),
            mode,
            environment: Reference::new("grants").unwrap(),
        }
    }

    fn video() -> VideoId {
        VideoId::new("12345678-1234-4234-8234-123456789abc").unwrap()
    }

    fn upload(mode: ProviderMode, endpoint: &str, signature: &str, at: i64) -> Option<UploadGrant> {
        UploadGrant::new(
            &binding(mode),
            video(),
            UploadProtocol::Tus,
            endpoint,
            signature,
            at,
        )
        .ok()
    }

    #[test]
    fn upload_grants_take_the_binding_and_their_inputs() {
        let grant = upload(
            ProviderMode::Offline,
            "https://upload.example/tus",
            "sig-1",
            1_800_000_000,
        )
        .unwrap();
        assert_eq!(grant.endpoint, "https://upload.example/tus");
        assert_eq!(grant.library, LibraryId::new(7).unwrap());
        assert_eq!(grant.video, video());
        assert_eq!(grant.expires_at, 1_800_000_000);
        assert_eq!(grant.expose_signature(), "sig-1");
        assert_eq!(grant.mode, ProviderMode::Offline);
        assert_eq!(grant.protocol, UploadProtocol::Tus);
        assert_eq!(
            (grant.content_type.as_deref(), grant.content_length),
            (None, None)
        );
        assert!(!format!("{grant:?}").contains("sig-1"));

        let declared = grant.with_declaration(&UploadDeclaration::new("video/mp4", 42).unwrap());
        assert_eq!(declared.content_type.as_deref(), Some("video/mp4"));
        assert_eq!(declared.content_length, Some(42));
    }

    #[test]
    fn upload_grants_refuse_unsafe_endpoints_signatures_and_expiry() {
        let at = 1_800_000_000;
        let longest = format!("https://upload.example/{}", "a".repeat(MAX_URL_BYTES - 23));
        assert_eq!(longest.len(), MAX_URL_BYTES);
        assert!(upload(ProviderMode::RemoteUnvalidated, &longest, "s", at).is_some());
        for endpoint in [
            format!("{longest}a"),
            "upload.example/tus".into(),
            "ftp://upload.example/tus".into(),
            "http://upload.example/tus".into(),
            "https:///tus".into(),
            "https://user:pass@upload.example/tus".into(),
            "https://upload.example/tus#part".into(),
            "https://upload.example/a b".into(),
            "https://upload.example/\n".into(),
        ] {
            assert!(
                upload(ProviderMode::Offline, &endpoint, "s", at).is_none(),
                "{endpoint}"
            );
        }
        // Credentials are found before the path, query or end of the URL.
        assert!(
            upload(
                ProviderMode::Offline,
                "https://upload.example?x=a@b",
                "s",
                at
            )
            .is_some()
        );
        assert!(upload(ProviderMode::Offline, "https://upload.example/a@b", "s", at).is_some());
        assert!(upload(ProviderMode::Offline, "https://u@upload.example?x", "s", at).is_none());
        // Plain HTTP is the loopback protocol fixture's alone.
        assert!(
            upload(
                ProviderMode::ProtocolFixture,
                "http://127.0.0.1:9/tus",
                "s",
                at
            )
            .is_some()
        );

        let longest = "s".repeat(MAX_SIGNATURE_BYTES);
        assert!(upload(ProviderMode::Offline, "https://u.example", &longest, at).is_some());
        for signature in [
            String::new(),
            format!("{longest}s"),
            "a b".into(),
            "a\u{e9}".into(),
        ] {
            assert!(upload(ProviderMode::Offline, "https://u.example", &signature, at).is_none());
        }
        for at in [0, -1, 253_402_300_800] {
            assert!(upload(ProviderMode::Offline, "https://u.example", "s", at).is_none());
        }
        assert!(
            upload(
                ProviderMode::Offline,
                "https://u.example",
                "s",
                253_402_300_799
            )
            .is_some()
        );
    }

    #[test]
    fn playback_grants_validate_their_url_and_expiry() {
        let offline = binding(ProviderMode::Offline);
        let grant = PlaybackGrant::new(
            &offline,
            "https://cdn.example/v/playlist.m3u8?t=1",
            1_800_000_000,
        )
        .unwrap();
        assert_eq!(
            grant.expose_url(),
            "https://cdn.example/v/playlist.m3u8?t=1"
        );
        assert_eq!(
            (grant.expires_at, grant.mode),
            (1_800_000_000, ProviderMode::Offline)
        );
        assert!(!format!("{grant:?}").contains("cdn.example"));
        assert!(PlaybackGrant::new(&offline, "http://cdn.example/v", 1_800_000_000).is_err());
        assert!(PlaybackGrant::new(&offline, "https://cdn.example/v", 0).is_err());
        let fixture = binding(ProviderMode::ProtocolFixture);
        assert!(PlaybackGrant::new(&fixture, "http://127.0.0.1:9/v", 1_800_000_000).is_ok());
    }
}
