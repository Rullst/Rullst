#![cfg(feature = "bunny")]
mod support;
use rullst_media::{bunny::*, *};
use support::*;

#[test]
fn identity_metadata_and_secret_modes_fail_closed() {
    for id in ["", "../secret", "with space", "a/b", "a?b", "é"] {
        assert!(Reference::new(id).is_err());
    }
    assert!(VideoId::new(VIDEO.to_uppercase()).is_err());
    assert!(VideoId::new("00000000-0000-0000-0000-000000000000").is_err());
    assert!(LibraryId::new(0).is_err());
    assert!(Metadata::new("<script>", "").is_err());
    assert!(Metadata::new("\n", "").is_err());
    assert!(Metadata::new("title", "x".repeat(4097)).is_err());
    assert!(BunnyCredentials::new(API, "mock_webhook", EMBED, CDN).is_err());
    assert!(PrivateDelivery::configured(true, false, true).is_err());
    assert!(BunnyStream::new(config("fixture")).is_err());
    for origin in [
        "http://localhost:1234",
        "https://example.com",
        "http://user@127.0.0.1:1234",
        "http://127.0.0.1/path",
        "http://127.0.0.1?x",
        "http://127.0.0.1:0",
    ] {
        assert!(
            BunnyStream::for_protocol_tests(config("fixture"), origin).is_err(),
            "{origin}"
        );
    }
    let secret = BunnyCredentials::new(API, WEBHOOK, EMBED, CDN).unwrap();
    assert_eq!(format!("{secret:?}"), "BunnyCredentials([REDACTED])");
}

#[tokio::test]
async fn independent_signature_vectors_bind_video_expiry_library_and_directory() {
    let fixture = Fixture::new().await;
    let provider = fixture.provider();
    let video = VideoId::new(VIDEO).unwrap();
    let embed = provider
        .playback(&video, NOW, 300, PlaybackKind::Embed)
        .unwrap();
    assert!(embed.expose_url().contains(
        "token=2c7c0220e61162de8963ba8a79aab07feab0ced4e2441942d1e3309ec61b94b7&expires=1800000300"
    ));
    let hls = provider
        .playback(&video, NOW, 300, PlaybackKind::Hls)
        .unwrap();
    assert!(hls.expose_url().contains("bcdn_token=HS256-A8UVdqtejl95Waw1jNHqLTWqvuVLzv6AlQdjsAlApeg&expires=1800000300&token_path=%2F12345678-1234-4234-8234-123456789abc%2F/"));
    let upload = provider.upload(&video, NOW, 300).unwrap();
    assert_eq!(
        upload.expose_signature(),
        "54a6d056526d1a2f8766177319c2a2f7fb2cfc9eac77664244adebbda82fff66"
    );
    assert!(!format!("{embed:?}{hls:?}{upload:?}{provider:?}").contains("2c7c0220"));
    for ttl in [0, 901] {
        assert!(
            provider
                .playback(&video, NOW, ttl, PlaybackKind::Embed)
                .is_err()
        );
    }
    assert!(provider.upload(&video, NOW, 3601).is_err());
    assert!(provider.upload(&video, i64::MAX, 300).is_err());
}

#[tokio::test]
async fn provider_lifecycle_validates_distinct_api_statuses_and_response_identity() {
    let fixture = Fixture::new().await;
    let provider = fixture.provider();
    let marker = "rullst-video-0123456789abcdef0123456789abcdef";
    let video = provider.create(marker).await.unwrap();
    assert_eq!(
        provider.find_created(marker).await.unwrap(),
        Some(video.clone())
    );
    provider.update(&video.id, &metadata()).await.unwrap();
    for (number, status) in [
        (0, Processing::AwaitingUpload),
        (1, Processing::Processing),
        (2, Processing::Processing),
        (3, Processing::Processing),
        (4, Processing::Ready),
        (5, Processing::Failed),
        (6, Processing::Failed),
        (7, Processing::Processing),
        (8, Processing::Processing),
    ] {
        fixture
            .remote
            .lock()
            .unwrap()
            .videos
            .get_mut(video.id.as_str())
            .unwrap()["status"] = serde_json::json!(number);
        assert_eq!(
            provider.get(&video.id).await.unwrap().unwrap().processing,
            status
        );
    }
    fixture.remote.lock().unwrap().wrong_identity = true;
    assert_eq!(provider.get(&video.id).await, Err(MediaError::Protocol));
    fixture.remote.lock().unwrap().wrong_identity = false;
    fixture
        .remote
        .lock()
        .unwrap()
        .videos
        .get_mut(video.id.as_str())
        .unwrap()["status"] = serde_json::json!(99);
    assert_eq!(provider.get(&video.id).await, Err(MediaError::Protocol));
    provider.delete(&video.id).await.unwrap();
    provider.delete(&video.id).await.unwrap();
    assert_eq!(provider.get(&video.id).await.unwrap(), None);
}

#[tokio::test]
async fn ambiguous_create_never_retries_and_redirects_and_false_success_are_rejected() {
    let fixture = Fixture::new().await;
    let provider = fixture.provider();
    let marker = "rullst-video-1123456789abcdef0123456789abcdef";
    fixture.remote.lock().unwrap().lost_create = true;
    assert_eq!(provider.create(marker).await, Err(MediaError::Unavailable));
    assert_eq!(
        fixture
            .remote
            .lock()
            .unwrap()
            .calls
            .iter()
            .filter(|v| *v == "create")
            .count(),
        1
    );
    let video = provider.find_created(marker).await.unwrap().unwrap();
    fixture.remote.lock().unwrap().redirect = true;
    assert_eq!(provider.get(&video.id).await, Err(MediaError::Protocol));
    fixture.remote.lock().unwrap().redirect = false;
    fixture.remote.lock().unwrap().bad_success = true;
    assert_eq!(
        provider.update(&video.id, &metadata()).await,
        Err(MediaError::Protocol)
    );
}

#[tokio::test]
async fn webhook_authenticates_exact_bytes_version_and_library_without_status_authority() {
    let fixture = Fixture::new().await;
    let provider = fixture.provider();
    let video = VideoId::new(VIDEO).unwrap();
    let event = notification(&provider, &video, 3);
    assert_eq!(event.video(), &video);
    assert_eq!(event.mode(), ProviderMode::ProtocolFixture);
    let body =
        br#"{"VideoLibraryId":7,"VideoGuid":"12345678-1234-4234-8234-123456789abc","Status":3}"#;
    let key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, WEBHOOK.as_bytes());
    let signature: String = ring::hmac::sign(&key, body)
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    for (version, algorithm, sig, payload) in [
        ("v2", "hmac-sha256", signature.clone(), body.to_vec()),
        ("v1", "sha256", signature.clone(), body.to_vec()),
        ("v1", "hmac-sha256", signature.to_uppercase(), body.to_vec()),
        (
            "v1",
            "hmac-sha256",
            signature.clone(),
            [body.as_slice(), b" "].concat(),
        ),
    ] {
        assert!(
            provider
                .verify_notification(
                    WebhookHeaders {
                        version,
                        algorithm,
                        signature: &sig
                    },
                    &payload
                )
                .is_err()
        );
    }
    // Authenticated bytes still need valid semantics and unambiguous JSON.
    for payload in [
        String::from_utf8(body.to_vec())
            .unwrap()
            .replace("\"VideoLibraryId\":7", "\"VideoLibraryId\":8"),
        String::from_utf8(body.to_vec())
            .unwrap()
            .replace("\"Status\":3", "\"Status\":11"),
        String::from_utf8(body.to_vec())
            .unwrap()
            .replace("\"Status\":3", "\"Status\":3,\"Status\":4"),
        " ".repeat(4097),
    ] {
        let signature: String = ring::hmac::sign(&key, payload.as_bytes())
            .as_ref()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert!(
            provider
                .verify_notification(
                    WebhookHeaders {
                        version: "v1",
                        algorithm: "hmac-sha256",
                        signature: &signature
                    },
                    payload.as_bytes()
                )
                .is_err()
        );
    }
}

#[tokio::test]
async fn empty_credentials_are_deterministic_offline_and_not_playback_authority() {
    let provider = BunnyStream::new(config("offline")).unwrap();
    let a = provider
        .create("rullst-video-0123456789abcdef0123456789abcdef")
        .await
        .unwrap();
    let b = provider
        .create("rullst-video-0123456789abcdef0123456789abcdef")
        .await
        .unwrap();
    assert_eq!(a, b);
    assert_eq!(provider.binding().mode, ProviderMode::Offline);
    let grant = provider
        .playback(&a.id, NOW, 60, PlaybackKind::Embed)
        .unwrap();
    assert!(
        grant
            .expose_url()
            .starts_with("https://rullst-media.invalid/")
    );
}

#[tokio::test]
async fn metadata_edits_preserve_unrelated_tags_and_refuse_to_overflow_the_provider_limit() {
    let fixture = Fixture::new().await;
    let provider = fixture.provider();
    let video = provider
        .create("rullst-video-0123456789abcdef0123456789abcdef")
        .await
        .unwrap();
    fixture
        .remote
        .lock()
        .unwrap()
        .videos
        .get_mut(video.id.as_str())
        .unwrap()["metaTags"] = serde_json::json!([
        {"property":"og:type","value":"video.other"}, {"property":"description","value":"Old"}
    ]);
    provider.update(&video.id, &metadata()).await.unwrap();
    let current = fixture.remote.lock().unwrap().videos[video.id.as_str()].clone();
    assert_eq!(
        current["metaTags"],
        serde_json::json!([
            {"property":"og:type","value":"video.other"}, {"property":"description","value":metadata().description()}
        ])
    );
    let full: Vec<_> = (0..50)
        .map(|i| serde_json::json!({"property":format!("tag-{i}"),"value":"keep"}))
        .collect();
    fixture
        .remote
        .lock()
        .unwrap()
        .videos
        .get_mut(video.id.as_str())
        .unwrap()["metaTags"] = serde_json::json!(full);
    let before = fixture
        .remote
        .lock()
        .unwrap()
        .calls
        .iter()
        .filter(|s| *s == "update")
        .count();
    assert_eq!(
        provider.update(&video.id, &metadata()).await.unwrap_err(),
        MediaError::Capacity
    );
    assert_eq!(
        fixture
            .remote
            .lock()
            .unwrap()
            .calls
            .iter()
            .filter(|s| *s == "update")
            .count(),
        before
    );
    assert_eq!(
        fixture.remote.lock().unwrap().videos[video.id.as_str()]["metaTags"],
        serde_json::json!(full)
    );
}
