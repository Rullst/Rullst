#![cfg(feature = "bunny")]
//! Exact grant URLs per provider mode, grant lifetime bounds and webhook
//! body bounds of the Bunny adapter. Building grants needs no network.
use ring::hmac;
use rullst_media::{bunny::*, *};

const NOW: i64 = 1_800_000_000;
const VIDEO: &str = "12345678-1234-4234-8234-123456789abc";
const FIXTURE_ORIGIN: &str = "http://127.0.0.1:9";
const WEBHOOK: &str = "fixture_webhook_key_123456789";

fn config(keys: [&str; 4]) -> BunnyConfig {
    BunnyConfig::new(
        LibraryId::new(7).unwrap(),
        Reference::new("acceptance").unwrap(),
        "test-library.b-cdn.net",
        BunnyCredentials::new(keys[0], keys[1], keys[2], keys[3]).unwrap(),
        PrivateDelivery::configured(true, true, true).unwrap(),
    )
    .unwrap()
}

fn offline() -> BunnyStream {
    BunnyStream::new(config([
        "mock_api",
        "mock_webhook",
        "mock_embed",
        "mock_cdn",
    ]))
    .unwrap()
}

fn fixture() -> BunnyStream {
    BunnyStream::for_protocol_tests(
        config([
            "fixture_api_key_123456789",
            WEBHOOK,
            "fixture_embed_key_123456789",
            "fixture_cdn_key_123456789",
        ]),
        FIXTURE_ORIGIN,
    )
    .unwrap()
}

fn remote() -> BunnyStream {
    BunnyStream::new(config([
        "remote_api_key_123456789",
        "remote_webhook_key_123456789",
        "remote_embed_key_123456789",
        "remote_cdn_key_123456789",
    ]))
    .unwrap()
}

fn video() -> VideoId {
    VideoId::new(VIDEO).unwrap()
}

fn urls(provider: &BunnyStream) -> [String; 3] {
    [
        PlaybackKind::Embed,
        PlaybackKind::Hls,
        PlaybackKind::Mp4_720p,
    ]
    .map(|kind| {
        let grant = provider.playback(&video(), NOW, 900, kind).unwrap();
        assert_eq!(grant.expires_at, 1_800_000_900);
        grant.expose_url().to_owned()
    })
}

#[test]
fn offline_grants_point_at_the_reserved_invalid_origin() {
    let provider = offline();
    assert_eq!(provider.binding().mode, ProviderMode::Offline);
    let upload = provider.upload(&video(), NOW, 3600).unwrap();
    assert_eq!(upload.endpoint, "https://rullst-media.invalid/tusupload");
    assert_eq!(upload.expires_at, 1_800_003_600);
    for url in urls(&provider) {
        assert_eq!(
            url,
            "https://rullst-media.invalid/videos/12345678-1234-4234-8234-123456789abc?expires=1800000900"
        );
    }
}

#[test]
fn fixture_grants_use_the_fixture_origin() {
    let provider = fixture();
    let upload = provider.upload(&video(), NOW, 3600).unwrap();
    assert_eq!(upload.endpoint, "http://127.0.0.1:9/tusupload");
    assert_eq!(upload.expires_at, 1_800_003_600);
    assert_eq!(upload.mode, ProviderMode::ProtocolFixture);
    assert_eq!(
        upload.expose_signature(),
        "753d351f52d8417f4a4b39dc11d572801b8124199fd5fddea890ae7596c8bf9d"
    );
    assert_eq!(
        urls(&provider),
        [
            "http://127.0.0.1:9/embed/7/12345678-1234-4234-8234-123456789abc?token=1f4c6bc508a79a2e584ea93b91a6ec857d719f47908fb9d35243375281e457a8&expires=1800000900",
            "http://127.0.0.1:9/bcdn_token=HS256-hs6waQ36T7iemDVHnsmE5emYNOOnV0y5zaT2ltpXLW4&expires=1800000900&token_path=%2F12345678-1234-4234-8234-123456789abc%2F/12345678-1234-4234-8234-123456789abc/playlist.m3u8",
            "http://127.0.0.1:9/12345678-1234-4234-8234-123456789abc/play_720p.mp4?token=HS256-4f0oMU-rIX7gFKIfAfEWCutkh1GkqGsGfDId6eKkA9M&expires=1800000900",
        ]
    );
}

#[test]
fn remote_grants_use_bunny_player_and_cdn_origins() {
    let provider = remote();
    assert_eq!(provider.binding().mode, ProviderMode::RemoteUnvalidated);
    let upload = provider.upload(&video(), NOW, 3600).unwrap();
    assert_eq!(upload.endpoint, "https://video.bunnycdn.com/tusupload");
    assert_eq!(
        upload.expose_signature(),
        "dee0efbaf42d7e0d4d484b4a639aa69d65b706b1e13e2081d46b5e931dee4fda"
    );
    assert_eq!(
        urls(&provider),
        [
            "https://iframe.mediadelivery.net/embed/7/12345678-1234-4234-8234-123456789abc?token=8c5f37e5e12378803d8a430234bbe91c8ce25f271a25d936c24f7198c9cb9e54&expires=1800000900",
            "https://test-library.b-cdn.net/bcdn_token=HS256-uBT4m6fScRxDYDpoSGQ1LZQSNTHUipG5PTRivxhzRDk&expires=1800000900&token_path=%2F12345678-1234-4234-8234-123456789abc%2F/12345678-1234-4234-8234-123456789abc/playlist.m3u8",
            "https://test-library.b-cdn.net/12345678-1234-4234-8234-123456789abc/play_720p.mp4?token=HS256-0solMfRoCdhpTx8tCj1r9PSwlQWCYoFN9wszUkywPgY&expires=1800000900",
        ]
    );
}

#[test]
fn grant_lifetimes_end_at_their_maximum() {
    let provider = offline();
    assert_eq!(
        provider.upload(&video(), NOW, 3600).unwrap().expires_at,
        1_800_003_600
    );
    assert_eq!(
        provider.upload(&video(), NOW, 3601).unwrap_err(),
        MediaError::InvalidInput
    );
    assert_eq!(
        provider
            .playback(&video(), NOW, 900, PlaybackKind::Embed)
            .unwrap()
            .expires_at,
        1_800_000_900
    );
    assert_eq!(
        provider
            .playback(&video(), NOW, 901, PlaybackKind::Embed)
            .unwrap_err(),
        MediaError::InvalidInput
    );
}

#[test]
fn debug_output_redacts_secrets_and_keeps_the_binding() {
    let provider = offline();
    assert_eq!(
        format!("{provider:?}"),
        "BunnyStream { binding: ProviderBinding { library: LibraryId(7), mode: Offline, environment: Reference(\"c9b23a941a7a04b1938f7be2698b0efefd0fed4a5c173c99a393ee45619dc148\") }, .. }"
    );
    let playback = provider
        .playback(&video(), NOW, 900, PlaybackKind::Embed)
        .unwrap();
    assert_eq!(
        format!("{playback:?}"),
        "PlaybackGrant { url: \"[REDACTED]\", expires_at: 1800000900, mode: Offline }"
    );
    let upload = provider.upload(&video(), NOW, 3600).unwrap();
    assert_eq!(
        format!("{upload:?}"),
        "UploadGrant { signature: \"[REDACTED]\", expires_at: 1800003600, mode: Offline, protocol: Tus }"
    );
}

fn sign(body: &[u8]) -> String {
    let key = hmac::Key::new(hmac::HMAC_SHA256, WEBHOOK.as_bytes());
    hmac::sign(&key, body)
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn verify(provider: &BunnyStream, body: &[u8]) -> Result<VerifiedNotification, MediaError> {
    let signature = sign(body);
    provider.verify_notification(WebhookHeaders::new("v1", "hmac-sha256", &signature), body)
}

#[test]
fn notification_bodies_are_bounded_before_authentication() {
    let provider = fixture();
    let body =
        br#"{"VideoLibraryId":7,"VideoGuid":"12345678-1234-4234-8234-123456789abc","Status":10}"#;
    let event = verify(&provider, body).unwrap();
    assert_eq!(event.video().as_str(), VIDEO);
    assert_eq!(event.library().value(), 7);
    assert_eq!(
        event.body_digest(),
        "3b9a984b501ba954b77ada84519e446e750d054450e20d1bc1f28e8ef1959bca"
    );
    let mut padded = body.to_vec();
    padded.resize(4096, b' ');
    assert_eq!(verify(&provider, &padded).unwrap().video().as_str(), VIDEO);
    padded.push(b' ');
    assert_eq!(
        verify(&provider, &padded).unwrap_err(),
        MediaError::Signature
    );
    assert_eq!(verify(&provider, b"").unwrap_err(), MediaError::Signature);
}

#[test]
fn notification_status_codes_end_at_ten() {
    let provider = fixture();
    for (status, accepted) in [(0, true), (10, true), (11, false), (255, false)] {
        let body = format!(r#"{{"VideoLibraryId":7,"VideoGuid":"{VIDEO}","Status":{status}}}"#);
        let result = verify(&provider, body.as_bytes());
        if accepted {
            assert_eq!(result.unwrap().video().as_str(), VIDEO);
        } else {
            assert_eq!(result.unwrap_err(), MediaError::Protocol);
        }
    }
}
