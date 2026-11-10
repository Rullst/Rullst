#![cfg(feature = "bunny")]
//! v13 upload-only configuration, titled creation and library listing.
mod support;
use rullst_media::{bunny::*, *};
use support::*;

fn upload_only(key: &str) -> BunnyConfig {
    BunnyConfig::upload_only(
        LibraryId::new(7).unwrap(),
        Reference::new("acceptance").unwrap(),
        BunnyCredentials::upload_only(key).unwrap(),
    )
    .unwrap()
}

fn signed_body() -> (Vec<u8>, String) {
    let body =
        br#"{"VideoLibraryId":7,"VideoGuid":"12345678-1234-4234-8234-123456789abc","Status":3}"#;
    let key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, WEBHOOK.as_bytes());
    let signature = ring::hmac::sign(&key, body)
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    (body.to_vec(), signature)
}

#[test]
fn upload_only_credentials_validate_the_api_key_and_refuse_delivery_configs() {
    for key in [
        "short_remote",
        "fixture_short",
        "with space in the key",
        &"k".repeat(513),
    ] {
        assert!(BunnyCredentials::upload_only(key).is_err(), "{key}");
    }
    // Remote keys hold 16 to 512 printable bytes; offline keys only the cap.
    assert!(BunnyCredentials::upload_only("k".repeat(15)).is_err());
    for key in [
        "k".repeat(16),
        "k".repeat(512),
        "mock_".into(),
        String::new(),
    ] {
        assert!(
            BunnyCredentials::upload_only(key.clone()).is_ok(),
            "{}",
            key.len()
        );
    }
    assert!(BunnyCredentials::upload_only(format!("mock_{}", "k".repeat(508))).is_err());
    let credentials = BunnyCredentials::upload_only(API).unwrap();
    assert_eq!(format!("{credentials:?}"), "BunnyCredentials([REDACTED])");
    // A complete configuration needs the delivery keys it signs with.
    assert!(matches!(
        BunnyConfig::new(
            LibraryId::new(7).unwrap(),
            Reference::new("acceptance").unwrap(),
            "test-library.b-cdn.net",
            credentials,
            PrivateDelivery::configured(true, true, true).unwrap(),
        ),
        Err(MediaError::Configuration)
    ));
    // Complete credentials are accepted; only the API key is used.
    BunnyConfig::upload_only(
        LibraryId::new(7).unwrap(),
        Reference::new("acceptance").unwrap(),
        BunnyCredentials::new(API, WEBHOOK, EMBED, CDN).unwrap(),
    )
    .unwrap();
    let remote = BunnyStream::new(upload_only("live_api_key_123456789")).unwrap();
    assert_eq!(remote.binding().mode, ProviderMode::RemoteUnvalidated);
    // Fixture keys still need the explicit loopback constructor.
    assert!(matches!(
        BunnyStream::new(upload_only(API)),
        Err(MediaError::Configuration)
    ));
}

#[tokio::test]
async fn upload_only_offline_manages_videos_but_never_delivers() {
    let provider = BunnyStream::new(upload_only("mock_api")).unwrap();
    assert_eq!(provider.binding().mode, ProviderMode::Offline);
    let marker = format!("rullst-video-{}", "a".repeat(32));
    let video = provider
        .create_with_metadata(&marker, &metadata())
        .await
        .unwrap();
    assert_eq!(video.title, "Ownership in Rust");
    let stored = provider.get(&video.id).await.unwrap().unwrap();
    assert_eq!(stored.title, "Ownership in Rust");
    assert_eq!(stored.description, metadata().description());
    assert!(provider.upload(&video.id, NOW, 300).is_ok());
    for kind in [
        PlaybackKind::Embed,
        PlaybackKind::Hls,
        PlaybackKind::Mp4_720p,
    ] {
        assert!(matches!(
            provider.playback(&video.id, NOW, 300, kind),
            Err(MediaError::Unsupported)
        ));
    }
    let (body, signature) = signed_body();
    let headers = WebhookHeaders {
        version: "v1",
        algorithm: "hmac-sha256",
        signature: &signature,
    };
    assert!(matches!(
        provider.verify_notification(headers, &body),
        Err(MediaError::Unsupported)
    ));
    provider.delete(&video.id).await.unwrap();
    assert!(provider.get(&video.id).await.unwrap().is_none());
}

#[tokio::test]
async fn upload_only_fixture_signs_the_same_upload_grant_and_titles_on_create() {
    let fixture = Fixture::new().await;
    let provider = BunnyStream::for_protocol_tests(upload_only(API), &fixture.origin).unwrap();
    let video = VideoId::new(VIDEO).unwrap();
    // Upload grants use only the API key, so they match a complete configuration.
    assert_eq!(
        provider
            .upload(&video, NOW, 300)
            .unwrap()
            .expose_signature(),
        fixture
            .provider()
            .upload(&video, NOW, 300)
            .unwrap()
            .expose_signature()
    );
    assert_ne!(provider.binding(), fixture.provider().binding());
    let marker = format!("rullst-video-{}", "b".repeat(32));
    let created = provider
        .create_with_metadata(&marker, &metadata())
        .await
        .unwrap();
    assert_eq!(created.title, "Ownership in Rust");
    let remote = fixture.remote.lock().unwrap();
    assert_eq!(remote.calls, ["create", "read", "update"]);
    let stored = &remote.videos[created.id.as_str()];
    assert_eq!(stored["title"], "Ownership in Rust");
    assert_eq!(stored["metaTags"][0]["value"], metadata().description());
}

#[tokio::test]
async fn a_failed_metadata_step_still_returns_the_created_video() {
    let fixture = Fixture::new().await;
    let provider = fixture.provider();
    fixture.remote.lock().unwrap().fail_reads = true;
    let marker = format!("rullst-video-{}", "c".repeat(32));
    let created = provider
        .create_with_metadata(&marker, &metadata())
        .await
        .unwrap();
    assert_eq!(
        created.title, marker,
        "the marker title tells the caller to retry"
    );
    fixture.remote.lock().unwrap().fail_reads = false;
    // The marker still finds the video, and `update` applies the title.
    let found = provider.find_created(&marker).await.unwrap().unwrap();
    assert_eq!(found.id, created.id);
    provider.update(&created.id, &metadata()).await.unwrap();
    assert_eq!(
        provider.get(&created.id).await.unwrap().unwrap().title,
        "Ownership in Rust"
    );
}

#[tokio::test]
async fn listing_sends_bounded_encoded_queries_and_validates_items() {
    let fixture = Fixture::new().await;
    let provider = fixture.provider();
    for (index, title) in ["Rust & C++ basics", "Ownership", "Lifetimes in Rust"]
        .into_iter()
        .enumerate()
    {
        let marker = format!("rullst-video-{index:032x}");
        let metadata = Metadata::new(title, "").unwrap();
        provider
            .create_with_metadata(&marker, &metadata)
            .await
            .unwrap();
    }
    let created = fixture.remote.lock().unwrap().videos.clone();
    let first = created.keys().next().unwrap().clone();
    fixture
        .remote
        .lock()
        .unwrap()
        .videos
        .get_mut(&first)
        .unwrap()["status"] = 4.into();

    let query = ListQuery::new(1, 10)
        .unwrap()
        .with_search("  rust & c++ ")
        .unwrap();
    let page = provider.list(&query).await.unwrap();
    let sent = fixture
        .remote
        .lock()
        .unwrap()
        .queries
        .last()
        .unwrap()
        .clone();
    assert_eq!(sent["search"], "rust & c++");
    assert_eq!(
        (sent["page"].as_str(), sent["itemsPerPage"].as_str()),
        ("1", "10")
    );
    assert_eq!(sent["orderBy"], "date");
    assert_eq!((page.total, page.page, page.per_page), (1, 1, 10));
    assert!(!page.has_more());
    let item = &page.items[0];
    assert_eq!(item.id.as_str(), first);
    assert_eq!(item.title, "Rust & C++ basics");
    assert_eq!(
        (item.length_seconds, item.processing),
        (12, Processing::Ready)
    );
    assert_eq!(item.created_at, Some(1_793_534_401));
    assert_eq!(
        item.thumbnail_url.as_deref(),
        Some(format!("https://test-library.b-cdn.net/{first}/thumbnail.jpg").as_str())
    );

    let by_title = ListQuery::new(1, 10).unwrap().with_order(ListOrder::Title);
    let titles: Vec<_> = provider
        .list(&by_title)
        .await
        .unwrap()
        .items
        .into_iter()
        .map(|item| item.title)
        .collect();
    assert_eq!(
        titles,
        ["Lifetimes in Rust", "Ownership", "Rust & C++ basics"]
    );
    let sent = fixture
        .remote
        .lock()
        .unwrap()
        .queries
        .last()
        .unwrap()
        .clone();
    assert_eq!(sent["orderBy"], "title");
    assert!(!sent.contains_key("search"));
    let later = provider
        .list(&ListQuery::new(2, 10).unwrap())
        .await
        .unwrap();
    assert!(later.items.is_empty());
    assert_eq!((later.total, later.page), (3, 2));

    // A full page reports that more follow.
    for index in 3..11 {
        let marker = format!("rullst-video-{index:032x}");
        provider.create(&marker).await.unwrap();
    }
    let full = provider
        .list(&ListQuery::new(1, 10).unwrap())
        .await
        .unwrap();
    assert_eq!((full.items.len(), full.total), (10, 11));
    assert!(full.has_more());

    fixture.remote.lock().unwrap().list_page_size = Some(100);
    assert!(matches!(
        provider.list(&by_title).await,
        Err(MediaError::Protocol)
    ));
    {
        let mut remote = fixture.remote.lock().unwrap();
        remote.list_page_size = None;
        remote.wrong_identity = true;
    }
    assert!(matches!(
        provider.list(&by_title).await,
        Err(MediaError::Protocol)
    ));
}

#[tokio::test]
async fn offline_listing_serves_a_fixed_sample_library() {
    let provider = BunnyStream::new(upload_only("mock_api")).unwrap();
    let page = provider
        .list(&ListQuery::new(1, 10).unwrap())
        .await
        .unwrap();
    let titles: Vec<_> = page.items.iter().map(|item| item.title.as_str()).collect();
    assert_eq!(
        titles,
        ["Getting started", "Ownership in Rust", "Failed upload"]
    );
    assert_eq!(page.total, 3);
    assert_eq!(page.items[0].processing, Processing::Ready);
    assert_eq!(page.items[0].length_seconds, 754);
    assert_eq!(page.items[0].created_at, Some(1_793_534_400));
    assert_eq!(
        page.items[0].thumbnail_url.as_deref(),
        Some("https://rullst-media.invalid/0c1f8a52-3d4e-4b6a-9f7e-2a1b3c4d5e6f/thumbnail.jpg")
    );
    // Repeating a query gives the same page.
    assert_eq!(
        provider
            .list(&ListQuery::new(1, 10).unwrap())
            .await
            .unwrap(),
        page
    );
    let by_title = ListQuery::new(1, 10).unwrap().with_order(ListOrder::Title);
    let titles: Vec<_> = provider
        .list(&by_title)
        .await
        .unwrap()
        .items
        .into_iter()
        .map(|item| item.title)
        .collect();
    assert_eq!(
        titles,
        ["Failed upload", "Getting started", "Ownership in Rust"]
    );
    let rust = ListQuery::new(1, 10).unwrap().with_search("RUST").unwrap();
    let found = provider.list(&rust).await.unwrap();
    assert_eq!((found.total, found.items.len()), (1, 1));
    assert_eq!(found.items[0].processing, Processing::Processing);
    let second = provider
        .list(&ListQuery::new(2, 10).unwrap())
        .await
        .unwrap();
    assert!(second.items.is_empty() && second.total == 3 && !second.has_more());
    // The samples are listed, never stored.
    assert!(provider.get(&page.items[0].id).await.unwrap().is_none());
}

#[test]
fn list_queries_are_bounded() {
    for (page, per_page) in [(0, 10), (1, 9), (1, 101), (100_001, 10)] {
        assert!(matches!(
            ListQuery::new(page, per_page),
            Err(MediaError::InvalidInput)
        ));
    }
    let query = ListQuery::new(100_000, 100).unwrap();
    assert_eq!((query.page(), query.per_page()), (100_000, 100));
    assert_eq!(query.order(), ListOrder::Date);
    assert!(query.clone().with_search("x".repeat(101)).is_err());
    assert!(query.clone().with_search("tab\there").is_err());
    let searched = query.clone().with_search("x".repeat(100)).unwrap();
    assert_eq!(searched.search().map(str::len), Some(100));
    assert_eq!(searched.with_search("   ").unwrap().search(), None);
}
