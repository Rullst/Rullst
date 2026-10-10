#![cfg(feature = "s3")]
//! Presigned grant structure, key safety and endpoint configuration, offline.
use rullst_media::{s3::*, *};
use std::collections::BTreeMap;

const NOW: i64 = 1_800_000_000; // 2027-01-15T08:00:00Z
const MARKER: &str = "rullst-video-0123456789abcdef0123456789abcdef";

fn library() -> LibraryId {
    LibraryId::new(7).unwrap()
}
fn live() -> S3Credentials {
    S3Credentials::new(
        "AKIAIOSFODNN7EXAMPLE",
        "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
    )
    .unwrap()
}
fn r2(credentials: S3Credentials) -> S3Config {
    S3Config::r2(library(), "0123abcd", "course-videos", credentials)
        .unwrap()
        .with_key_prefix("tenants/videos/")
        .unwrap()
        .with_max_object_bytes(1_000_000)
        .unwrap()
}
fn video() -> VideoId {
    VideoId::new("12345678-1234-4234-8234-123456789abc").unwrap()
}
fn query(signature: &str) -> BTreeMap<String, String> {
    signature
        .split('&')
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap();
            (key.to_owned(), value.to_owned())
        })
        .collect()
}

#[test]
fn presigned_put_binds_key_type_length_and_a_short_expiry() {
    let storage = S3Storage::new(r2(live())).unwrap();
    assert_eq!(storage.binding().mode, ProviderMode::RemoteUnvalidated);
    let declared = UploadDeclaration::new("Video/MP4", 524_288).unwrap();
    assert_eq!(declared.content_type(), "video/mp4");
    let grant = storage
        .upload_declared(&video(), NOW, 300, &declared)
        .unwrap();
    assert_eq!(
        grant.endpoint,
        "https://0123abcd.r2.cloudflarestorage.com/course-videos/tenants/videos/12345678-1234-4234-8234-123456789abc/original"
    );
    assert_eq!(grant.protocol, UploadProtocol::PresignedPut);
    assert_eq!(grant.content_type.as_deref(), Some("video/mp4"));
    assert_eq!(grant.content_length, Some(524_288));
    assert_eq!(grant.expires_at, NOW + 300);
    assert_eq!(grant.video, video());
    let fields = query(grant.expose_signature());
    assert_eq!(fields["X-Amz-Algorithm"], "AWS4-HMAC-SHA256");
    assert_eq!(
        fields["X-Amz-Credential"],
        "AKIAIOSFODNN7EXAMPLE%2F20270115%2Fauto%2Fs3%2Faws4_request"
    );
    assert_eq!(fields["X-Amz-Date"], "20270115T080000Z");
    assert_eq!(fields["X-Amz-Expires"], "300");
    assert_eq!(
        fields["X-Amz-SignedHeaders"],
        "content-length%3Bcontent-type%3Bhost"
    );
    let signature = &fields["X-Amz-Signature"];
    assert_eq!(signature.len(), 64);
    assert!(signature.bytes().all(|b| b.is_ascii_hexdigit()));
    // Deterministic for a fixed clock and credentials; any bound field changes it.
    let again = storage
        .upload_declared(&video(), NOW, 300, &declared)
        .unwrap();
    assert_eq!(again.expose_signature(), grant.expose_signature());
    for other in [
        UploadDeclaration::new("video/mp4", 524_289).unwrap(),
        UploadDeclaration::new("video/webm", 524_288).unwrap(),
    ] {
        let changed = storage.upload_declared(&video(), NOW, 300, &other).unwrap();
        assert_ne!(
            query(changed.expose_signature())["X-Amz-Signature"],
            *signature
        );
    }
    // Serialized for the browser; Debug never shows the bearer parts.
    let json = serde_json::to_value(&grant).unwrap();
    assert_eq!(json["protocol"], "PresignedPut");
    assert_eq!(json["content_length"], 524_288);
    let debug = format!("{grant:?}");
    assert!(!debug.contains("X-Amz") && !debug.contains("r2.cloudflarestorage"));
}

#[test]
fn grants_reject_unbounded_expiry_unaccepted_types_and_oversized_objects() {
    let storage = S3Storage::new(r2(live())).unwrap();
    let declared = UploadDeclaration::new("video/mp4", 10).unwrap();
    for ttl in [0, MAX_GRANT_SECONDS + 1, 3600] {
        assert_eq!(
            storage
                .upload_declared(&video(), NOW, ttl, &declared)
                .unwrap_err(),
            MediaError::InvalidInput
        );
        assert_eq!(
            storage
                .playback(&video(), NOW, ttl, PlaybackKind::Original)
                .unwrap_err(),
            MediaError::InvalidInput
        );
    }
    let html = UploadDeclaration::new("text/html", 10).unwrap();
    assert_eq!(
        storage
            .upload_declared(&video(), NOW, 60, &html)
            .unwrap_err(),
        MediaError::InvalidInput
    );
    let large = UploadDeclaration::new("video/mp4", 1_000_001).unwrap();
    assert_eq!(
        storage
            .upload_declared(&video(), NOW, 60, &large)
            .unwrap_err(),
        MediaError::Capacity
    );
    // Without a declaration there is nothing to bind.
    assert_eq!(
        storage.upload(&video(), NOW, 60).unwrap_err(),
        MediaError::Unsupported
    );
    for (value, length) in [
        ("video/mp4; codecs=avc1", 1),
        ("video", 1),
        ("video/", 1),
        ("vid eo/mp4", 1),
        ("video/mp4", 0),
        ("video/mp4", UploadDeclaration::MAX_LENGTH + 1),
    ] {
        assert_eq!(
            UploadDeclaration::new(value, length).unwrap_err(),
            MediaError::InvalidInput
        );
    }
}

#[test]
fn presigned_get_serves_only_the_original_with_a_short_expiry() {
    let storage = S3Storage::new(r2(live())).unwrap();
    let grant = storage
        .playback(&video(), NOW, 120, PlaybackKind::Original)
        .unwrap();
    assert_eq!(grant.expires_at, NOW + 120);
    let (url, signature) = grant.expose_url().split_once('?').unwrap();
    assert!(
        url.ends_with(
            "/course-videos/tenants/videos/12345678-1234-4234-8234-123456789abc/original"
        )
    );
    let fields = query(signature);
    assert_eq!(fields["X-Amz-Expires"], "120");
    assert_eq!(fields["X-Amz-SignedHeaders"], "host");
    assert!(!format!("{grant:?}").contains("X-Amz"));
    for kind in [
        PlaybackKind::Embed,
        PlaybackKind::Hls,
        PlaybackKind::Mp4_720p,
    ] {
        assert_eq!(
            storage.playback(&video(), NOW, 60, kind).unwrap_err(),
            MediaError::Unsupported
        );
    }
}

#[test]
fn temporary_credentials_are_signed_into_the_query() {
    let credentials = live()
        .with_session_token("FwoGZXIvYXdzEJr//token+value=")
        .unwrap();
    let storage = S3Storage::new(r2(credentials)).unwrap();
    let grant = storage
        .playback(&video(), NOW, 60, PlaybackKind::Original)
        .unwrap();
    let (_, signature) = grant.expose_url().split_once('?').unwrap();
    assert_eq!(
        query(signature)["X-Amz-Security-Token"],
        "FwoGZXIvYXdzEJr%2F%2Ftoken%2Bvalue%3D"
    );
    assert!(
        S3Credentials::new("", "")
            .unwrap()
            .with_session_token("x")
            .is_err()
    );
}

#[test]
fn endpoints_cover_r2_aws_and_minio_and_reject_unsafe_configuration() {
    let cloudflare = S3Storage::new(r2(live())).unwrap();
    let again = S3Storage::new(r2(live())).unwrap();
    assert_eq!(cloudflare.binding(), again.binding());
    let aws = S3Config::aws(library(), "eu-west-1", "course-videos", live()).unwrap();
    let aws = S3Storage::new(aws).unwrap();
    let grant = aws
        .playback(&video(), NOW, 60, PlaybackKind::Original)
        .unwrap();
    assert!(grant.expose_url().starts_with(
        "https://course-videos.s3.eu-west-1.amazonaws.com/12345678-1234-4234-8234-123456789abc/original?"
    ));
    assert!(grant.expose_url().contains("%2Feu-west-1%2Fs3%2F"));
    assert_ne!(aws.binding().environment, cloudflare.binding().environment);
    let minio = S3Config::custom(
        library(),
        "https://minio.internal:9000",
        "us-east-1",
        "videos",
        live(),
    )
    .unwrap();
    let grant = S3Storage::new(minio)
        .unwrap()
        .playback(&video(), NOW, 60, PlaybackKind::Original)
        .unwrap();
    assert!(
        grant
            .expose_url()
            .starts_with("https://minio.internal:9000/videos/")
    );
    assert!(
        S3Config::custom(
            library(),
            "http://127.0.0.1:9000",
            "us-east-1",
            "videos",
            live()
        )
        .is_ok()
    );
    for endpoint in [
        "http://minio.internal:9000",
        "https://user:pass@minio.internal",
        "https://minio.internal/prefix",
        "https://minio.internal/?x=1",
        "ftp://minio.internal",
    ] {
        assert!(
            S3Config::custom(library(), endpoint, "us-east-1", "videos", live()).is_err(),
            "{endpoint}"
        );
    }
    for bucket in ["ab", "Upper", "a..b", "-edge", "edge-", "under_score"] {
        assert!(
            S3Config::custom(library(), "https://s3.local", "us-east-1", bucket, live()).is_err()
        );
    }
    assert!(S3Config::aws(library(), "auto", "videos", live()).is_err());
    assert!(S3Config::r2(library(), "Account!", "videos", live()).is_err());
    // Dotted buckets break virtual-hosted TLS names.
    assert!(S3Config::aws(library(), "us-east-1", "a.b.c", live()).is_err());
}

#[test]
fn keys_are_server_generated_and_prefixes_cannot_traverse() {
    let config = || S3Config::r2(library(), "0123abcd", "videos", live()).unwrap();
    for prefix in [
        "../",
        "a/../",
        "/abs/",
        "a",
        "a//",
        "a b/",
        "a/./",
        "%2e%2e/",
        "a\\b/",
        "1/2/3/4/5/6/7/8/9/",
    ] {
        assert!(config().with_key_prefix(prefix).is_err(), "{prefix}");
    }
    assert!(config().with_key_prefix("").is_ok());
    assert!(config().with_key_prefix("x".repeat(64) + "/").is_err());
    let storage = S3Storage::new(config().with_key_prefix("school_a/v-1/").unwrap()).unwrap();
    let grant = storage
        .playback(&video(), NOW, 60, PlaybackKind::Original)
        .unwrap();
    assert!(
        grant
            .expose_url()
            .contains("/videos/school_a/v-1/12345678-1234-4234-8234-123456789abc/original?")
    );
    // Identities are UUID-shaped digests chosen by the adapter, never input paths.
    assert!(VideoId::new("../../etc/passwd").is_err());
    assert!(
        S3Config::r2(library(), "0123abcd", "videos", live())
            .unwrap()
            .with_content_types(["video/mp4", "Video/MP4"])
            .is_err()
    );
}

#[tokio::test]
async fn offline_mock_is_deterministic_and_never_signs_for_a_bucket() {
    let storage =
        S3Storage::new(r2(S3Credentials::new("mock_access", "mock_secret").unwrap())).unwrap();
    assert_eq!(storage.binding().mode, ProviderMode::Offline);
    let created = storage.create(MARKER).await.unwrap();
    assert_eq!(created.processing, Processing::AwaitingUpload);
    assert_eq!(created.title, MARKER);
    assert_eq!(storage.create(MARKER).await.unwrap().id, created.id);
    assert_eq!(
        storage.find_created(MARKER).await.unwrap().unwrap().id,
        created.id
    );
    assert!(storage.create("../escape").await.is_err());
    let declared = UploadDeclaration::new("video/webm", 64).unwrap();
    let grant = storage
        .upload_declared(&created.id, NOW, 60, &declared)
        .unwrap();
    assert_eq!(grant.mode, ProviderMode::Offline);
    assert!(grant.endpoint.starts_with("https://rullst-media.invalid/"));
    let playback = storage
        .playback(&created.id, NOW, 60, PlaybackKind::Original)
        .unwrap();
    assert!(
        playback
            .expose_url()
            .starts_with("https://rullst-media.invalid/")
    );
    storage
        .simulate_upload(&created.id, "video/webm", 64)
        .unwrap();
    assert_eq!(
        storage.get(&created.id).await.unwrap().unwrap().processing,
        Processing::Ready
    );
    storage.delete(&created.id).await.unwrap();
    assert!(storage.get(&created.id).await.unwrap().is_none());
    // Mixed, partial and fixture credentials never select the mock.
    assert!(S3Credentials::new("mock_access", "").is_err());
    assert!(S3Credentials::new("mock_access", "real-secret-value").is_err());
    assert!(
        S3Storage::new(r2(
            S3Credentials::new("fixture_access", "fixture_secret").unwrap()
        ))
        .is_err()
    );
    assert_eq!(format!("{:?}", live()), "S3Credentials([REDACTED])");
    let live_storage = S3Storage::new(r2(live())).unwrap();
    assert_eq!(
        live_storage
            .simulate_upload(&created.id, "video/webm", 64)
            .unwrap_err(),
        MediaError::Unsupported
    );
}

#[cfg(feature = "bunny")]
#[test]
fn bunny_rejects_object_storage_capabilities() {
    use rullst_media::bunny::*;
    let config = BunnyConfig::new(
        library(),
        Reference::new("acceptance").unwrap(),
        "test-library.b-cdn.net",
        BunnyCredentials::new("mock_api", "mock_webhook", "mock_embed", "mock_cdn").unwrap(),
        PrivateDelivery::configured(true, true, true).unwrap(),
    )
    .unwrap();
    let bunny = BunnyStream::new(config).unwrap();
    let declared = UploadDeclaration::new("video/mp4", 10).unwrap();
    assert_eq!(
        bunny
            .upload_declared(&video(), NOW, 60, &declared)
            .unwrap_err(),
        MediaError::Unsupported
    );
    assert_eq!(
        bunny
            .playback(&video(), NOW, 60, PlaybackKind::Original)
            .unwrap_err(),
        MediaError::Unsupported
    );
    assert_eq!(
        bunny.upload(&video(), NOW, 60).unwrap().protocol,
        UploadProtocol::Tus
    );
}

#[test]
fn an_upload_of_exactly_the_object_limit_is_granted() {
    let storage = S3Storage::new(r2(live())).unwrap();
    let at_limit = UploadDeclaration::new("video/mp4", 1_000_000).unwrap();
    let grant = storage
        .upload_declared(&video(), NOW, 300, &at_limit)
        .unwrap();
    assert_eq!(grant.content_length, Some(1_000_000));
    let over = UploadDeclaration::new("video/mp4", 1_000_001).unwrap();
    assert_eq!(
        storage
            .upload_declared(&video(), NOW, 300, &over)
            .unwrap_err(),
        MediaError::Capacity
    );
}

#[tokio::test]
async fn a_renamed_video_no_longer_matches_its_creation_marker() {
    let storage =
        S3Storage::new(r2(S3Credentials::new("mock_access", "mock_secret").unwrap())).unwrap();
    let created = storage.create(MARKER).await.unwrap();
    storage
        .update(&created.id, &Metadata::new("Lesson 1", "").unwrap())
        .await
        .unwrap();
    assert_eq!(
        storage.find_created(MARKER).await.unwrap_err(),
        MediaError::Conflict
    );
}

#[tokio::test]
async fn a_full_offline_store_still_answers_an_existing_marker() {
    let storage =
        S3Storage::new(r2(S3Credentials::new("mock_access", "mock_secret").unwrap())).unwrap();
    let marker = |index: u32| format!("rullst-video-{index:032x}");
    for index in 0..10_000 {
        storage.create(&marker(index)).await.unwrap();
    }
    // Creation is idempotent at capacity; only a new marker is refused.
    let existing = storage.create(&marker(42)).await.unwrap();
    assert_eq!(existing.title, marker(42));
    assert_eq!(
        storage.create(&marker(10_000)).await.unwrap_err(),
        MediaError::Capacity
    );
}
