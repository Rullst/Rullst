#![cfg(feature = "s3")]
//! `S3Credentials::from_env`. The process environment is global, so this
//! binary holds a single test that sets it.
use rullst_media::{s3::*, *};

fn set(name: &str, value: Option<&str>) {
    // SAFETY: this binary runs no other test, so nothing reads the
    // environment concurrently.
    match value {
        Some(value) => unsafe { std::env::set_var(name, value) },
        None => unsafe { std::env::remove_var(name) },
    }
}

fn security_token(credentials: S3Credentials) -> Option<String> {
    let config = S3Config::r2(
        LibraryId::new(7).unwrap(),
        "0123abcd",
        "videos",
        credentials,
    )
    .unwrap();
    let video = VideoId::new("12345678-1234-4234-8234-123456789abc").unwrap();
    let grant = S3Storage::new(config)
        .unwrap()
        .playback(&video, 1_800_000_000, 60, PlaybackKind::Original)
        .unwrap();
    let (_, query) = grant.expose_url().split_once('?').unwrap();
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix("X-Amz-Security-Token="))
        .map(str::to_owned)
}

#[test]
fn only_a_non_empty_session_token_is_applied() {
    set("AWS_ACCESS_KEY_ID", None);
    set("AWS_SECRET_ACCESS_KEY", None);
    set("AWS_SESSION_TOKEN", Some(""));
    assert_eq!(
        S3Credentials::from_env().unwrap().mode(),
        ProviderMode::Offline
    );
    // The offline mock refuses temporary credentials.
    set("AWS_SESSION_TOKEN", Some("session-token"));
    assert_eq!(
        S3Credentials::from_env().unwrap_err(),
        MediaError::Configuration
    );
    set("AWS_ACCESS_KEY_ID", Some("AKIAIOSFODNN7EXAMPLE"));
    set(
        "AWS_SECRET_ACCESS_KEY",
        Some("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"),
    );
    let credentials = S3Credentials::from_env().unwrap();
    assert_eq!(credentials.mode(), ProviderMode::RemoteUnvalidated);
    assert_eq!(
        security_token(credentials).as_deref(),
        Some("session-token")
    );
    set("AWS_SESSION_TOKEN", Some(""));
    assert_eq!(security_token(S3Credentials::from_env().unwrap()), None);
    set("AWS_SESSION_TOKEN", None);
    assert_eq!(security_token(S3Credentials::from_env().unwrap()), None);
}
