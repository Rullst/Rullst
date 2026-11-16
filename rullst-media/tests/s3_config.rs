#![cfg(feature = "s3")]
//! Exact bounds of the S3 configuration: one rule at a time, at the limit
//! and one past it.
use rullst_media::{s3::*, *};

const NOW: i64 = 1_800_000_000;

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
fn r2() -> S3Config {
    S3Config::r2(library(), "0123abcd", "videos", live()).unwrap()
}
fn custom(endpoint: &str, region: &str) -> Result<S3Config, MediaError> {
    S3Config::custom(library(), endpoint, region, "videos", live())
}
fn video() -> VideoId {
    VideoId::new("12345678-1234-4234-8234-123456789abc").unwrap()
}

#[test]
fn session_tokens_hold_one_to_8192_visible_bytes() {
    assert!(live().with_session_token("t".repeat(8192)).is_ok());
    for token in ["t".repeat(8193), String::new(), "two words".into()] {
        assert_eq!(
            live().with_session_token(token.clone()).unwrap_err(),
            MediaError::Configuration,
            "{} bytes",
            token.len()
        );
    }
}

#[test]
fn the_default_object_limit_is_two_gibibytes() {
    assert_eq!(S3Config::DEFAULT_MAX_OBJECT_BYTES, 2_147_483_648);
    let storage = S3Storage::new(r2()).unwrap();
    let at_limit = UploadDeclaration::new("video/mp4", 2_147_483_648).unwrap();
    let grant = storage
        .upload_declared(&video(), NOW, 300, &at_limit)
        .unwrap();
    assert_eq!(grant.content_length, Some(2_147_483_648));
    let over = UploadDeclaration::new("video/mp4", 2_147_483_649).unwrap();
    assert_eq!(
        storage
            .upload_declared(&video(), NOW, 300, &over)
            .unwrap_err(),
        MediaError::Capacity
    );
}

#[test]
fn object_limits_range_from_one_byte_to_five_gibibytes() {
    let storage = S3Storage::new(r2().with_max_object_bytes(5_368_709_120).unwrap()).unwrap();
    let largest = UploadDeclaration::new("video/mp4", 5_368_709_120).unwrap();
    let grant = storage
        .upload_declared(&video(), NOW, 300, &largest)
        .unwrap();
    assert_eq!(grant.content_length, Some(5_368_709_120));
    assert!(r2().with_max_object_bytes(1).is_ok());
    for maximum in [0, 5_368_709_121] {
        assert_eq!(
            r2().with_max_object_bytes(maximum).unwrap_err(),
            MediaError::Configuration,
            "{maximum}"
        );
    }
}

#[test]
fn r2_account_ids_hold_one_to_64_characters() {
    assert!(S3Config::r2(library(), "a".repeat(64), "videos", live()).is_ok());
    for account in ["a".repeat(65), String::new()] {
        assert_eq!(
            S3Config::r2(library(), account.clone(), "videos", live()).unwrap_err(),
            MediaError::Configuration,
            "{} bytes",
            account.len()
        );
    }
}

#[test]
fn custom_endpoints_refuse_each_unsafe_part_alone() {
    assert!(custom("https://minio.internal:9000", "us-east-1").is_ok());
    for endpoint in [
        "https://user@minio.internal",
        "https://:secret@minio.internal",
        "https://minio.internal:0",
        "https://minio.internal/#fragment",
    ] {
        assert_eq!(
            custom(endpoint, "us-east-1").unwrap_err(),
            MediaError::Configuration,
            "{endpoint}"
        );
    }
}

#[test]
fn regions_hold_one_to_32_lower_case_letters_digits_or_hyphens() {
    assert!(custom("https://minio.internal", &"a".repeat(32)).is_ok());
    assert!(custom("https://minio.internal", "eu-west-1").is_ok());
    for region in [
        "a".repeat(33),
        String::new(),
        "US-EAST-1".into(),
        "eu_west".into(),
    ] {
        assert_eq!(
            custom("https://minio.internal", &region).unwrap_err(),
            MediaError::Configuration,
            "{region}"
        );
    }
    assert!(S3Config::aws(library(), "EU-WEST-1", "videos", live()).is_err());
}

#[test]
fn key_prefixes_hold_up_to_eight_segments_and_256_bytes() {
    let longest_segment = format!("{}/", "s".repeat(63));
    assert!(r2().with_key_prefix(longest_segment).is_ok());
    assert!(r2().with_key_prefix("1/2/3/4/5/6/7/8/").is_ok());
    let longest = format!("{}/", "p".repeat(63)).repeat(4);
    assert_eq!(longest.len(), 256);
    assert!(r2().with_key_prefix(longest.clone()).is_ok());
    let over = format!("{longest}b/");
    assert_eq!(over.len(), 258);
    assert_eq!(
        r2().with_key_prefix(over).unwrap_err(),
        MediaError::Configuration
    );
}

#[test]
fn accepted_content_types_hold_one_to_sixteen_values() {
    let types = |count: usize| (1..=count).map(|i| format!("video/t{i:02}"));
    assert!(r2().with_content_types(["video/mp4"]).is_ok());
    assert!(r2().with_content_types(types(16)).is_ok());
    for count in [0, 17] {
        assert_eq!(
            r2().with_content_types(types(count)).unwrap_err(),
            MediaError::Configuration,
            "{count}"
        );
    }
}
