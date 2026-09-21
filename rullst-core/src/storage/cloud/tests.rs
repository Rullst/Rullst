use super::*;
use crate::{Storage, StorageError, TenantStorage, security::TenantMembership};
use std::time::{Duration, SystemTime};

fn live_credentials() -> CloudCredentials {
    CloudCredentials::new(
        "RULLSTTESTACCESS",
        "rullst-test-secret-never-a-provider-key",
    )
    .unwrap()
}

fn offline() -> Storage {
    Storage::s3("private-files", "us-east-1")
        .with_cloud_config(CloudStorageConfig::new(
            CloudCredentials::new("", "").unwrap(),
        ))
        .unwrap()
}

#[test]
fn configuration_fails_closed_without_disclosing_credentials() {
    for (access, secret) in [
        ("", "real"),
        ("real", ""),
        ("mock_key", "real"),
        ("real", "mock_secret"),
        ("key\r\n", "secret"),
        ("key", "secret\n"),
    ] {
        assert_eq!(
            CloudCredentials::new(access, secret).unwrap_err(),
            CloudError::Configuration
        );
    }
    let credentials = live_credentials();
    let debug = format!("{credentials:?}");
    assert!(!debug.contains("RULLSTTESTACCESS"));
    assert!(!debug.contains("rullst-test-secret"));
    let mock =
        CloudStorageConfig::new(CloudCredentials::new("mock_access", "mock_secret").unwrap());
    assert_eq!(
        mock.require_production().unwrap_err(),
        CloudError::Configuration
    );
    let production = CloudStorageConfig::new(credentials)
        .require_production()
        .unwrap();
    assert_eq!(
        production
            .with_loopback_test_endpoint("http://127.0.0.1:9000")
            .unwrap_err(),
        CloudError::Configuration
    );
}

#[test]
fn loopback_policy_excludes_remote_and_ambiguous_endpoints() {
    for endpoint in [
        "http://localhost:9000",
        "http://192.168.1.1",
        "https://example.org",
        "http://user@127.0.0.1",
        "http://127.0.0.1/prefix",
        "http://127.0.0.1/?key=value",
        "http://127.0.0.1/#fragment",
        "ftp://127.0.0.1",
    ] {
        assert_eq!(
            CloudStorageConfig::new(live_credentials())
                .with_loopback_test_endpoint(endpoint)
                .unwrap_err(),
            CloudError::Configuration,
            "{endpoint}"
        );
    }
    for endpoint in ["http://127.0.0.1:9000", "https://[::1]:9000"] {
        let config = CloudStorageConfig::new(live_credentials())
            .with_loopback_test_endpoint(endpoint)
            .unwrap();
        assert_eq!(
            config.require_production().unwrap_err(),
            CloudError::Configuration
        );
    }
}

#[test]
fn provider_and_resource_configuration_rejects_invalid_values() {
    for bucket in [
        "",
        "ab",
        "../x",
        "UPPER",
        "bad..name",
        "127.0.0.1",
        "-first",
        "last-",
        "bad.-name",
    ] {
        assert!(
            Storage::s3(bucket, "us-east-1")
                .with_cloud_config(CloudStorageConfig::new(live_credentials()))
                .is_err()
        );
    }
    assert!(
        Storage::r2("private-files", "wrong-account")
            .with_cloud_config(CloudStorageConfig::new(live_credentials()))
            .is_err()
    );
    assert!(
        Storage::s3("private-files", "us-east-1.evil.test")
            .with_cloud_config(CloudStorageConfig::new(live_credentials()))
            .is_err()
    );
    assert!(
        Storage::local("unused")
            .with_cloud_config(CloudStorageConfig::new(live_credentials()))
            .is_err()
    );
    for (size, time) in [
        (0, Duration::from_secs(1)),
        (usize::MAX, Duration::from_secs(1)),
        (1, Duration::ZERO),
        (1, Duration::from_secs(121)),
    ] {
        assert_eq!(
            CloudStorageConfig::new(live_credentials())
                .with_limits(size, time)
                .unwrap_err(),
            CloudError::Configuration
        );
    }
}

#[tokio::test]
async fn offline_journey_preserves_tenant_boundaries_and_never_fakes_a_grant() {
    let storage = offline();
    assert!(storage.is_cloud_mock());
    let membership = TenantMembership::try_new(["school-a", "school-b"]).unwrap();
    let a = TenantStorage::from_context(storage.clone(), &membership.select("school-a").unwrap());
    let b = TenantStorage::from_context(storage.clone(), &membership.select("school-b").unwrap());
    a.put("certificates/one.pdf", b"school a").await.unwrap();
    b.put("certificates/one.pdf", b"school b").await.unwrap();
    assert_eq!(a.get("certificates/one.pdf").await.unwrap(), b"school a");
    assert_eq!(b.get("certificates/one.pdf").await.unwrap(), b"school b");
    assert_eq!(
        a.metadata("certificates/one.pdf").await.unwrap().size_bytes,
        8
    );
    assert!(matches!(
        a.get("../school-b/certificates/one.pdf").await,
        Err(StorageError::Cloud(CloudError::InvalidKey))
    ));
    assert!(matches!(
        a.signed_download("certificates/one.pdf", Duration::from_secs(60)),
        Err(StorageError::Cloud(CloudError::MockGrantUnsupported))
    ));
    assert!(storage.url("certificates/one.pdf").is_err());
    a.delete("certificates/one.pdf").await.unwrap();
    a.delete("certificates/one.pdf").await.unwrap();
    assert!(matches!(
        a.get("certificates/one.pdf").await,
        Err(StorageError::NotFound(_))
    ));
    assert_eq!(b.get("certificates/one.pdf").await.unwrap(), b"school b");
}

#[tokio::test]
async fn offline_limits_are_enforced_before_mutation() {
    let storage = Storage::s3("private-files", "us-east-1")
        .with_cloud_config(
            CloudStorageConfig::new(CloudCredentials::new("", "").unwrap())
                .with_limits(4, Duration::from_secs(1))
                .unwrap(),
        )
        .unwrap();
    storage.put("file", b"good").await.unwrap();
    assert!(matches!(
        storage.put("file", b"large").await,
        Err(StorageError::Cloud(CloudError::SizeLimit))
    ));
    assert_eq!(storage.get("file").await.unwrap(), b"good");
    for i in 1..256 {
        storage.put(&format!("file-{i}"), b"x").await.unwrap();
    }
    assert!(matches!(
        storage.put("overflow", b"x").await,
        Err(StorageError::Cloud(CloudError::MockUnavailable))
    ));
    storage.delete("file").await.unwrap();
    storage.put("overflow", b"x").await.unwrap();
}

#[tokio::test]
async fn every_cloud_operation_rejects_unsafe_or_ambiguous_keys() {
    let storage = offline();
    for key in [
        "",
        "../secret",
        "/absolute",
        "a/../b",
        "a/./b",
        "a//b",
        "a/",
        "a\\b",
        "a\0b",
        "a\nb",
    ] {
        for result in [
            storage.put(key, b"x").await,
            storage.get(key).await.map(|_| ()),
            storage.metadata(key).await.map(|_| ()),
            storage.delete(key).await,
            storage
                .signed_download(key, Duration::from_secs(60))
                .map(|_| ()),
        ] {
            assert!(
                matches!(result, Err(StorageError::Cloud(CloudError::InvalidKey))),
                "{key:?}: {result:?}"
            );
        }
    }
    assert!(storage.put(&"x".repeat(1025), b"x").await.is_err());
}

#[test]
fn signed_downloads_bind_provider_tenant_key_region_and_short_lifetime() {
    let storage = Storage::r2("private-files", "1234567890abcdef1234567890abcdef")
        .with_cloud_config(
            CloudStorageConfig::new(live_credentials())
                .require_production()
                .unwrap(),
        )
        .unwrap();
    let membership = TenantMembership::try_new(["school-a"]).unwrap();
    let tenant = TenantStorage::from_context(storage, &membership.select("school-a").unwrap());
    let grant = tenant
        .signed_download("certificates/á %?#.pdf", Duration::from_secs(90))
        .unwrap();
    let url = reqwest::Url::parse(grant.expose_url()).unwrap();
    assert_eq!(
        url.host_str(),
        Some("1234567890abcdef1234567890abcdef.r2.cloudflarestorage.com")
    );
    assert_eq!(
        url.path(),
        "/private-files/tenants/school-a/certificates/%C3%A1%20%25%3F%23.pdf"
    );
    let query: std::collections::BTreeMap<_, _> = url.query_pairs().into_owned().collect();
    assert_eq!(query.get("X-Amz-Expires").unwrap(), "90");
    assert!(
        query
            .get("X-Amz-Credential")
            .unwrap()
            .ends_with("/auto/s3/aws4_request")
    );
    assert_eq!(query.get("X-Amz-SignedHeaders").unwrap(), "host");
    assert!(!format!("{grant:?}").contains("X-Amz"));
    assert!(!format!("{grant:?}").contains("school-a"));
    for lifetime in [
        Duration::ZERO,
        Duration::from_secs(901),
        Duration::from_millis(1500),
    ] {
        assert!(matches!(
            tenant.signed_download("file", lifetime),
            Err(StorageError::Cloud(CloudError::InvalidLifetime))
        ));
    }
}

#[test]
fn temporary_credentials_cannot_issue_grants_beyond_expiry() {
    let now = SystemTime::now();
    let credentials = live_credentials()
        .with_session_token("session+/=token", now + Duration::from_secs(120))
        .unwrap();
    let storage = Storage::s3("private-files", "us-east-1")
        .with_cloud_config(CloudStorageConfig::new(credentials))
        .unwrap();
    let grant = storage
        .signed_download("file", Duration::from_secs(60))
        .unwrap();
    assert!(
        reqwest::Url::parse(grant.expose_url())
            .unwrap()
            .query_pairs()
            .any(|(key, value)| key == "X-Amz-Security-Token" && value == "session+/=token")
    );
    assert!(matches!(
        storage.signed_download("file", Duration::from_secs(120)),
        Err(StorageError::Cloud(CloudError::CredentialsExpired))
    ));
}

#[test]
fn expired_session_credentials_cannot_sign_a_request() {
    let now = SystemTime::now();
    let config = CloudStorageConfig::new(
        live_credentials()
            .with_session_token("session-token", now + Duration::from_secs(10))
            .unwrap(),
    );
    let url = reqwest::Url::parse("https://s3.us-east-1.amazonaws.com/private-files/file").unwrap();
    assert_eq!(
        super::signing::headers(
            &config,
            "us-east-1",
            &reqwest::Method::GET,
            &url,
            &[],
            now + Duration::from_secs(11)
        )
        .unwrap_err(),
        CloudError::CredentialsExpired
    );
    assert!(
        live_credentials()
            .with_session_token("token", now - Duration::from_secs(1))
            .is_err()
    );
    assert!(
        live_credentials()
            .with_session_token("bad token", now + Duration::from_secs(10))
            .is_err()
    );
    assert!(
        CloudCredentials::new("", "")
            .unwrap()
            .with_session_token("token", now + Duration::from_secs(10))
            .is_err()
    );
}

#[tokio::test]
async fn offline_total_byte_quota_is_independent_of_object_and_count_limits() {
    let storage = Storage::s3("private-files", "us-east-1")
        .with_cloud_config(
            CloudStorageConfig::new(CloudCredentials::new("", "").unwrap())
                .with_limits(128 * 1024 * 1024, Duration::from_secs(1))
                .unwrap(),
        )
        .unwrap();
    let bytes = vec![0; 64 * 1024 * 1024 + 1];
    assert_eq!(
        storage.put("oversized-store", &bytes).await.unwrap_err(),
        StorageError::Cloud(CloudError::MockUnavailable)
    );
    assert!(matches!(
        storage.get("oversized-store").await,
        Err(StorageError::NotFound(_))
    ));
    storage.put("small", b"available").await.unwrap();
}

#[test]
fn signing_does_not_expose_private_keys_or_payloads_to_trace_subscribers() {
    use std::{
        io::Write,
        sync::{Arc, Mutex},
    };
    #[derive(Clone)]
    struct Capture(Arc<Mutex<Vec<u8>>>);
    impl Write for Capture {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let captured = Capture(Arc::default());
    let writer = captured.clone();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_max_level(tracing::Level::TRACE)
        .with_writer(move || writer.clone())
        .finish();
    tracing::subscriber::with_default(subscriber, || {
        let config = CloudStorageConfig::new(live_credentials());
        let url = reqwest::Url::parse(
            "https://s3.us-east-1.amazonaws.com/private-files/private-subject-marker",
        )
        .unwrap();
        super::signing::headers(
            &config,
            "us-east-1",
            &reqwest::Method::PUT,
            &url,
            b"private-payload-marker",
            SystemTime::now(),
        )
        .unwrap();
        super::signing::signed_download(
            &config,
            "us-east-1",
            url,
            Duration::from_secs(60),
            SystemTime::now(),
        )
        .unwrap();
        tracing::info!("capture-is-active");
    });
    let bytes = captured.0.lock().unwrap();
    let output = String::from_utf8_lossy(&bytes);
    assert!(output.contains("capture-is-active"));
    for marker in [
        "private-subject-marker",
        "private-payload-marker",
        "RULLSTTESTACCESS",
        "rullst-test-secret",
        "X-Amz-",
    ] {
        assert!(!output.contains(marker), "signing emitted a private value");
    }
}
