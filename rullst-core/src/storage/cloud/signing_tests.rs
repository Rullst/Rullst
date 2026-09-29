//! SigV4 canonical-path fixtures for S3 object keys.
//!
//! The two `aws_*` tests reproduce the published AWS S3 SigV4 examples. The
//! path-style signatures were computed independently from the AWS
//! specification (canonical URI = `UriEncode` of the decoded bucket and key)
//! with a reference implementation that reproduces both AWS examples.

use super::*;
use crate::security::TenantMembership;
use crate::storage::cloud::CloudCredentials;
use crate::{Storage, TenantStorage};

const EXAMPLE_ACCESS_KEY: &str = "AKIAIOSFODNN7EXAMPLE";
const EXAMPLE_SECRET_KEY: &str = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";

/// A key containing every character class that S3 canonicalizes differently
/// from the `url` crate's path-segment set, plus Unicode and `%`.
const SPECIAL_KEY: &str = "tenants/acme:prod/Report (final)+v=1,a;b@c!*'$&^|á%.pdf";
const SPECIAL_PATH: &str = "/private-files/tenants/acme%3Aprod/Report%20%28final%29%2Bv%3D1%2Ca%3Bb%40c%21%2A%27%24%26%5E%7C%C3%A1%25.pdf";

fn example_config() -> CloudStorageConfig {
    CloudStorageConfig::new(CloudCredentials::new(EXAMPLE_ACCESS_KEY, EXAMPLE_SECRET_KEY).unwrap())
}

/// `20130524T000000Z`, the timestamp used by the AWS examples.
fn example_time() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_369_353_600)
}

fn endpoint() -> Url {
    Url::parse("https://s3.us-east-1.amazonaws.com/").unwrap()
}

fn authorization(headers: &HeaderMap) -> String {
    headers
        .get(reqwest::header::AUTHORIZATION)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}

#[test]
fn aws_get_object_example_signature_is_reproduced() {
    let url = Url::parse("https://examplebucket.s3.amazonaws.com/test.txt").unwrap();
    let mut extra = HeaderMap::new();
    extra.insert(
        reqwest::header::RANGE,
        HeaderValue::from_static("bytes=0-9"),
    );
    let headers = sign_headers(
        &example_config(),
        "us-east-1",
        &Method::GET,
        &url,
        &[],
        example_time(),
        extra,
    )
    .unwrap();
    let authorization = authorization(&headers);
    assert!(
        authorization
            .contains("Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request")
    );
    assert!(authorization.contains("SignedHeaders=host;range;x-amz-content-sha256;x-amz-date"));
    assert!(
        authorization.ends_with(
            "Signature=f0e8bdb87c964420e857bd35b5d6ed310bd44f0170aba48dd91039c6036bdb41"
        )
    );
}

/// The AWS example key `test$file.text` has the canonical URI `/test%24file.text`.
#[test]
fn aws_put_object_example_with_a_reserved_character_is_reproduced() {
    assert_eq!(
        object_url(&endpoint(), "examplebucket", "test$file.text")
            .unwrap()
            .path(),
        "/examplebucket/test%24file.text"
    );
    let url = Url::parse("https://examplebucket.s3.amazonaws.com/test%24file.text").unwrap();
    let mut extra = HeaderMap::new();
    extra.insert(
        reqwest::header::DATE,
        HeaderValue::from_static("Fri, 24 May 2013 00:00:00 GMT"),
    );
    extra.insert(
        HeaderName::from_static("x-amz-storage-class"),
        HeaderValue::from_static("REDUCED_REDUNDANCY"),
    );
    let headers = sign_headers(
        &example_config(),
        "us-east-1",
        &Method::PUT,
        &url,
        b"Welcome to Amazon S3.",
        example_time(),
        extra,
    )
    .unwrap();
    assert_eq!(
        headers.get("x-amz-content-sha256").unwrap(),
        "44ce7dd67c959e0d3524ffac1771dfbba87d2b6b4b4e99e42034a8b803f8b072"
    );
    let authorization = authorization(&headers);
    assert!(
        authorization.contains(
            "SignedHeaders=date;host;x-amz-content-sha256;x-amz-date;x-amz-storage-class"
        )
    );
    assert!(
        authorization.ends_with(
            "Signature=98ad721746da40c64f1a55b78f14c238d841ea1380cd77a1b5971af0ece108bd"
        )
    );
}

#[test]
fn object_paths_percent_encode_every_byte_except_unreserved_characters() {
    let encoded = [
        (':', "%3A"),
        ('=', "%3D"),
        ('+', "%2B"),
        ('(', "%28"),
        (')', "%29"),
        ('!', "%21"),
        ('*', "%2A"),
        ('\'', "%27"),
        ('$', "%24"),
        ('&', "%26"),
        (',', "%2C"),
        (';', "%3B"),
        ('@', "%40"),
        ('^', "%5E"),
        ('|', "%7C"),
        ('[', "%5B"),
        (']', "%5D"),
        (' ', "%20"),
        ('%', "%25"),
        ('?', "%3F"),
        ('#', "%23"),
        ('"', "%22"),
        ('<', "%3C"),
        ('>', "%3E"),
        ('`', "%60"),
        ('{', "%7B"),
        ('}', "%7D"),
        ('á', "%C3%A1"),
    ];
    for (character, escape) in encoded {
        let url = object_url(&endpoint(), "private-files", &format!("a{character}b")).unwrap();
        assert_eq!(url.path(), format!("/private-files/a{escape}b"), "{escape}");
    }

    let unreserved = "AZaz09-._~";
    let url = object_url(&endpoint(), "private-files", &format!("{unreserved}/x")).unwrap();
    assert_eq!(url.path(), format!("/private-files/{unreserved}/x"));
    assert_eq!(
        object_url(&endpoint(), "private-files", SPECIAL_KEY)
            .unwrap()
            .path(),
        SPECIAL_PATH
    );
}

#[test]
fn special_character_keys_match_independent_path_style_signatures() {
    let url = object_url(&endpoint(), "private-files", SPECIAL_KEY).unwrap();
    let headers = headers(
        &example_config(),
        "us-east-1",
        &Method::GET,
        &url,
        &[],
        example_time(),
    )
    .unwrap();
    let authorization = authorization(&headers);
    assert!(authorization.contains("SignedHeaders=host;x-amz-content-sha256;x-amz-date"));
    assert!(
        authorization.ends_with(
            "Signature=581a2f07a8035775c55e30af5ba07255eb7f10d5b30e96ff9b54a66a26bca852"
        )
    );

    let grant = signed_download(
        &example_config(),
        "us-east-1",
        url,
        Duration::from_secs(60),
        example_time(),
    )
    .unwrap();
    let url = Url::parse(grant.expose_url()).unwrap();
    assert_eq!(url.path(), SPECIAL_PATH);
    let signature = url
        .query_pairs()
        .find(|(name, _)| name == "X-Amz-Signature")
        .map(|(_, value)| value.into_owned());
    assert_eq!(
        signature.as_deref(),
        Some("f66c45bcf6903bc0dc2cd7b73807d0abd4c5869f69f9fd2405d9e66b6409fe05")
    );
}

#[test]
fn tenant_identifiers_with_reserved_characters_are_encoded_once() {
    let storage = Storage::s3("private-files", "us-east-1")
        .with_cloud_config(CloudStorageConfig::new(
            CloudCredentials::new("RULLSTTESTACCESS", "rullst-test-secret").unwrap(),
        ))
        .unwrap();
    let membership = TenantMembership::try_new(["acme:prod"]).unwrap();
    let tenant = TenantStorage::from_context(storage, &membership.select("acme:prod").unwrap());
    let grant = tenant
        .signed_download("Report (final).pdf", Duration::from_secs(60))
        .unwrap();
    assert_eq!(
        Url::parse(grant.expose_url()).unwrap().path(),
        "/private-files/tenants/acme%3Aprod/Report%20%28final%29.pdf"
    );
}
