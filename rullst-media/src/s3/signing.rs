//! SigV4 through `aws-sigv4`, the signer Core's `storage-s3` feature uses.
use super::config::S3Config;
use crate::MediaError as Error;
use aws_credential_types::Credentials;
use aws_sigv4::{
    http_request::{
        PayloadChecksumKind, PercentEncodingMode, SignableBody, SignableRequest, SignatureLocation,
        SigningSettings, UriPathNormalizationMode, sign,
    },
    sign::v4,
};
use reqwest::Url;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// The object URL whose path is already the SigV4 canonical URI.
pub(super) fn object_url(config: &S3Config, key: &str) -> Result<Url, Error> {
    let mut url = config.endpoint.clone();
    let mut path = String::new();
    if config.path_style {
        path.push('/');
        uri_encode(&mut path, &config.bucket);
    } else {
        let host = url.host_str().ok_or(Error::Configuration)?;
        let host = format!("{}.{host}", config.bucket);
        url.set_host(Some(&host))
            .map_err(|_| Error::Configuration)?;
    }
    for segment in key.split('/') {
        if matches!(segment, "" | "." | "..") {
            return Err(Error::InvalidInput);
        }
        path.push('/');
        uri_encode(&mut path, segment);
    }
    url.set_path(&path);
    // The encoded path holds only unreserved characters, `%XX` and `/`.
    if url.path() != path {
        return Err(Error::InvalidInput);
    }
    Ok(url)
}

/// AWS SigV4 `UriEncode`: every byte except `A-Z a-z 0-9 - . _ ~` becomes
/// `%XX` with upper-case hex.
pub(super) fn uri_encode(output: &mut String, value: &str) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for &byte in value.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            output.push(char::from(byte));
        } else {
            output.push('%');
            output.push(char::from(HEX[usize::from(byte >> 4)]));
            output.push(char::from(HEX[usize::from(byte & 0x0F)]));
        }
    }
}

pub(super) fn system_time(unix_seconds: i64) -> Result<SystemTime, Error> {
    let seconds = u64::try_from(unix_seconds).map_err(|_| Error::Clock)?;
    UNIX_EPOCH
        .checked_add(Duration::from_secs(seconds))
        .ok_or(Error::Clock)
}

fn settings() -> SigningSettings {
    let mut settings = SigningSettings::default();
    settings.percent_encoding_mode = PercentEncodingMode::Single;
    settings.uri_path_normalization_mode = UriPathNormalizationMode::Disabled;
    settings
}

fn credentials(config: &S3Config) -> Credentials {
    let keys = &config.credentials;
    Credentials::new(
        keys.access_key.as_str(),
        keys.secret_key.as_str(),
        keys.session_token.as_ref().map(|token| token.to_string()),
        None,
        "rullst-media",
    )
}

/// Query-string signature (`X-Amz-*` pairs, without a leading `?`) for a
/// presigned request. Every supplied header is signed and must be sent as is.
pub(super) fn presign(
    config: &S3Config,
    method: &str,
    url: &Url,
    headers: &[(&str, &str)],
    now: SystemTime,
    lifetime: Duration,
) -> Result<String, Error> {
    let identity = credentials(config).into();
    let mut settings = settings();
    settings.signature_location = SignatureLocation::QueryParams;
    settings.expires_in = Some(lifetime);
    let params = v4::SigningParams::builder()
        .identity(&identity)
        .region(&config.region)
        .name("s3")
        .time(now)
        .settings(settings)
        .build()
        .map_err(|_| Error::Configuration)?
        .into();
    let request = SignableRequest::new(
        method,
        url.as_str(),
        headers.iter().copied(),
        SignableBody::UnsignedPayload,
    )
    .map_err(|_| Error::InvalidInput)?;
    // The signer's trace events include the request URI; keys stay private.
    let instructions =
        tracing::dispatcher::with_default(&tracing::Dispatch::none(), || sign(request, &params))
            .map_err(|_| Error::Configuration)?
            .into_parts()
            .0;
    if instructions.headers().next().is_some() {
        return Err(Error::Configuration);
    }
    let mut query = String::new();
    for (key, value) in instructions.params() {
        if !query.is_empty() {
            query.push('&');
        }
        uri_encode(&mut query, key);
        query.push('=');
        uri_encode(&mut query, value);
    }
    Ok(query)
}

/// `Authorization`, `x-amz-date`, `x-amz-content-sha256` and, for temporary
/// credentials, `x-amz-security-token` for one direct API request.
pub(super) fn authorize(
    config: &S3Config,
    method: &str,
    url: &Url,
    headers: &[(&str, &str)],
    body_sha256: &str,
    now: SystemTime,
) -> Result<Vec<(String, String)>, Error> {
    let identity = credentials(config).into();
    let mut settings = settings();
    settings.payload_checksum_kind = PayloadChecksumKind::XAmzSha256;
    let params = v4::SigningParams::builder()
        .identity(&identity)
        .region(&config.region)
        .name("s3")
        .time(now)
        .settings(settings)
        .build()
        .map_err(|_| Error::Configuration)?
        .into();
    let request = SignableRequest::new(
        method,
        url.as_str(),
        headers.iter().copied(),
        SignableBody::Precomputed(body_sha256.into()),
    )
    .map_err(|_| Error::InvalidInput)?;
    let instructions =
        tracing::dispatcher::with_default(&tracing::Dispatch::none(), || sign(request, &params))
            .map_err(|_| Error::Configuration)?
            .into_parts()
            .0;
    Ok(instructions
        .headers()
        .map(|(name, value)| (name.to_owned(), value.to_owned()))
        .collect())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::{LibraryId, s3::S3Credentials};

    /// AWS's documented presigned GET example ("Authenticating Requests: Using
    /// Query Parameters", Signature Version 4).
    #[test]
    fn presigned_get_matches_the_aws_documented_example() {
        let credentials = S3Credentials::new(
            "AKIAIOSFODNN7EXAMPLE",
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
        )
        .unwrap();
        let config = S3Config::custom(
            LibraryId::new(1).unwrap(),
            "https://s3.amazonaws.com",
            "us-east-1",
            "examplebucket",
            credentials,
        )
        .unwrap()
        .with_path_style(false)
        .unwrap();
        let url = object_url(&config, "test.txt").unwrap();
        assert_eq!(
            url.as_str(),
            "https://examplebucket.s3.amazonaws.com/test.txt"
        );
        let query = presign(
            &config,
            "GET",
            &url,
            &[],
            system_time(1_369_353_600).unwrap(),
            Duration::from_secs(86_400),
        )
        .unwrap();
        assert_eq!(
            query,
            "X-Amz-Algorithm=AWS4-HMAC-SHA256\
             &X-Amz-Credential=AKIAIOSFODNN7EXAMPLE%2F20130524%2Fus-east-1%2Fs3%2Faws4_request\
             &X-Amz-Date=20130524T000000Z&X-Amz-Expires=86400&X-Amz-SignedHeaders=host\
             &X-Amz-Signature=aeeed9bbccd4d02ee5c0109b86d86835f995330da4c265957d157751f604d404"
        );
    }

    #[test]
    fn object_paths_are_canonical_and_reject_ambiguous_segments() {
        let config = S3Config::r2(
            LibraryId::new(1).unwrap(),
            "abc123",
            "videos",
            S3Credentials::new("", "").unwrap(),
        )
        .unwrap();
        let url = object_url(&config, "a b/original").unwrap();
        assert_eq!(
            url.as_str(),
            "https://abc123.r2.cloudflarestorage.com/videos/a%20b/original"
        );
        for key in ["", "a//b", "../x", "a/./b", "a/"] {
            assert_eq!(object_url(&config, key).unwrap_err(), Error::InvalidInput);
        }
    }
}
