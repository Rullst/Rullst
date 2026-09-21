use super::{CloudError, CloudStorageConfig, SignedDownload};
use aws_sigv4::{
    http_request::{
        PayloadChecksumKind, PercentEncodingMode, SignableBody, SignableRequest, SignatureLocation,
        SigningSettings, UriPathNormalizationMode, sign,
    },
    sign::v4,
};
use reqwest::{
    Method, Url,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use std::fmt::Write;
use std::time::{Duration, SystemTime};

pub(crate) fn validate_key(key: &str) -> Result<(), CloudError> {
    if key.is_empty()
        || key.len() > 1024
        || key.chars().any(char::is_control)
        || key.contains('\\')
        || key.split('/').any(|part| matches!(part, "" | "." | ".."))
    {
        return Err(CloudError::InvalidKey);
    }
    Ok(())
}

pub(super) fn object_url(endpoint: &Url, bucket: &str, key: &str) -> Result<Url, CloudError> {
    validate_key(key)?;
    let mut url = endpoint.clone();
    url.path_segments_mut()
        .map_err(|_| CloudError::Configuration)?
        .pop_if_empty()
        .push(bucket)
        .extend(key.split('/'));
    Ok(url)
}

pub(super) fn headers(
    config: &CloudStorageConfig,
    region: &str,
    method: &Method,
    url: &Url,
    body: &[u8],
    now: SystemTime,
) -> Result<HeaderMap, CloudError> {
    check_expiry(config, now)?;
    let identity = config.credentials.credentials.clone().into();
    let mut settings = settings();
    settings.payload_checksum_kind = PayloadChecksumKind::XAmzSha256;
    let params = v4::SigningParams::builder()
        .identity(&identity)
        .region(region)
        .name("s3")
        .time(now)
        .settings(settings)
        .build()
        .map_err(|_| CloudError::Signing)?
        .into();
    let mut headers = HeaderMap::new();
    if *method == Method::PUT {
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            HeaderValue::from_static("application/octet-stream"),
        );
    }
    let signable_headers: &[(&str, &str)] = if *method == Method::PUT {
        &[("content-type", "application/octet-stream")]
    } else {
        &[]
    };
    // Never give upload bytes to the signer's optional raw-body trace formatter.
    let digest = ring::digest::digest(&ring::digest::SHA256, body);
    let mut checksum = String::with_capacity(64);
    for byte in digest.as_ref() {
        write!(checksum, "{byte:02x}").map_err(|_| CloudError::Signing)?;
    }
    let request = SignableRequest::new(
        method.as_str(),
        url.as_str(),
        signable_headers.iter().copied(),
        SignableBody::Precomputed(checksum),
    )
    .map_err(|_| CloudError::Signing)?;
    // Upstream trace instrumentation includes the request URI; object keys are private.
    let instructions =
        tracing::dispatcher::with_default(&tracing::Dispatch::none(), || sign(request, &params))
            .map_err(|_| CloudError::Signing)?
            .into_parts()
            .0;
    for (key, value) in instructions.headers() {
        let name = HeaderName::from_bytes(key.as_bytes()).map_err(|_| CloudError::Signing)?;
        let mut value = HeaderValue::from_str(value).map_err(|_| CloudError::Signing)?;
        value.set_sensitive(true);
        headers.insert(name, value);
    }
    Ok(headers)
}

pub(super) fn signed_download(
    config: &CloudStorageConfig,
    region: &str,
    mut url: Url,
    lifetime: Duration,
    now: SystemTime,
) -> Result<SignedDownload, CloudError> {
    if lifetime < Duration::from_secs(1)
        || lifetime > Duration::from_secs(900)
        || lifetime.subsec_nanos() != 0
    {
        return Err(CloudError::InvalidLifetime);
    }
    let expires_at = now
        .checked_add(lifetime)
        .ok_or(CloudError::InvalidLifetime)?;
    check_expiry(config, expires_at)?;
    let identity = config.credentials.credentials.clone().into();
    let mut settings = settings();
    settings.signature_location = SignatureLocation::QueryParams;
    settings.expires_in = Some(lifetime);
    let params = v4::SigningParams::builder()
        .identity(&identity)
        .region(region)
        .name("s3")
        .time(now)
        .settings(settings)
        .build()
        .map_err(|_| CloudError::Signing)?
        .into();
    let request = SignableRequest::new(
        "GET",
        url.as_str(),
        std::iter::empty(),
        SignableBody::UnsignedPayload,
    )
    .map_err(|_| CloudError::Signing)?;
    let instructions =
        tracing::dispatcher::with_default(&tracing::Dispatch::none(), || sign(request, &params))
            .map_err(|_| CloudError::Signing)?
            .into_parts()
            .0;
    if instructions.headers().next().is_some() {
        return Err(CloudError::Signing);
    }
    url.query_pairs_mut().extend_pairs(
        instructions
            .params()
            .iter()
            .map(|(key, value)| (*key, value.as_ref())),
    );
    Ok(SignedDownload {
        url: url.into(),
        expires_at,
    })
}

fn settings() -> SigningSettings {
    let mut settings = SigningSettings::default();
    settings.percent_encoding_mode = PercentEncodingMode::Single;
    settings.uri_path_normalization_mode = UriPathNormalizationMode::Disabled;
    settings
}

fn check_expiry(config: &CloudStorageConfig, at: SystemTime) -> Result<(), CloudError> {
    if config
        .credentials
        .credentials
        .expiry()
        .is_some_and(|expiry| expiry <= at)
    {
        return Err(CloudError::CredentialsExpired);
    }
    Ok(())
}
