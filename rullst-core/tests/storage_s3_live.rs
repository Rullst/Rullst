#![cfg(feature = "storage-s3")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Run only through `.github/test-storage-s3-live.sh` against its owned disposable service.
use rullst_core::{
    Storage, StorageError, TenantStorage,
    security::TenantMembership,
    storage::cloud::{CloudCredentials, CloudStorageConfig},
};
use std::time::Duration;

fn storage(r2: bool) -> Storage {
    let endpoint = std::env::var("RULLST_STORAGE_TEST_ENDPOINT")
        .expect("disposable service endpoint required");
    let credentials = CloudCredentials::new(
        "GK11111111111111111111111111111111",
        "2222222222222222222222222222222222222222222222222222222222222222",
    )
    .unwrap();
    let config = CloudStorageConfig::new(credentials)
        .with_loopback_test_endpoint(endpoint)
        .unwrap();
    let storage = if r2 {
        Storage::r2("private-files", "1234567890abcdef1234567890abcdef")
    } else {
        Storage::s3("private-files", "auto")
    };
    storage.with_cloud_config(config).unwrap()
}

async fn assert_grant_denied(response: reqwest::Response) {
    // Garage returns 400 for missing authorization and 403 for bad signatures.
    assert!(matches!(response.status().as_u16(), 400 | 403));
    let body = response.text().await.unwrap();
    assert!(!body.contains("private certificate"));
}

#[tokio::test]
#[ignore = "requires the digest-pinned disposable S3 service from test-storage-s3-live.sh"]
async fn private_object_journey_rejects_unsigned_tampered_and_expired_grants() {
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    for r2 in [false, true] {
        let storage = storage(r2);
        assert!(!storage.is_cloud_mock());
        let membership = TenantMembership::try_new(["school-a", "school-b"]).unwrap();
        let a =
            TenantStorage::from_context(storage.clone(), &membership.select("school-a").unwrap());
        let b =
            TenantStorage::from_context(storage.clone(), &membership.select("school-b").unwrap());
        let key = "certificates/á %?#.pdf";
        a.put(key, b"private certificate a").await.unwrap();
        b.put(key, b"private certificate b").await.unwrap();
        assert_eq!(a.get(key).await.unwrap(), b"private certificate a");
        assert_eq!(b.get(key).await.unwrap(), b"private certificate b");
        let metadata = a.metadata(key).await.unwrap();
        assert_eq!(metadata.size_bytes, 21);
        assert!(metadata.etag.is_some());
        assert!(a.get("../school-b/certificates/one.pdf").await.is_err());
        let grant = a.signed_download(key, Duration::from_secs(60)).unwrap();
        let response = client.get(grant.expose_url()).send().await.unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(
            response.bytes().await.unwrap().as_ref(),
            b"private certificate a"
        );

        let mut unsigned = reqwest::Url::parse(grant.expose_url()).unwrap();
        unsigned.set_query(None);
        assert_grant_denied(client.get(unsigned).send().await.unwrap()).await;
        let tampered = grant.expose_url().replace("school-a", "school-b");
        assert_grant_denied(client.get(tampered).send().await.unwrap()).await;
        let mut extended = reqwest::Url::parse(grant.expose_url()).unwrap();
        let pairs: Vec<_> = extended
            .query_pairs()
            .into_owned()
            .map(|(k, v)| {
                if k == "X-Amz-Expires" {
                    (k, "900".to_string())
                } else {
                    (k, v)
                }
            })
            .collect();
        extended.set_query(None);
        extended.query_pairs_mut().extend_pairs(pairs);
        assert_grant_denied(client.get(extended).send().await.unwrap()).await;

        let short_grant = a.signed_download(key, Duration::from_secs(1)).unwrap();
        tokio::time::sleep(Duration::from_secs(2)).await;
        assert_grant_denied(client.get(short_grant.expose_url()).send().await.unwrap()).await;
        a.delete(key).await.unwrap();
        a.delete(key).await.unwrap();
        assert!(matches!(a.get(key).await, Err(StorageError::NotFound(_))));
        assert_eq!(b.get(key).await.unwrap(), b"private certificate b");
        b.delete(key).await.unwrap();
    }
    storage(false)
        .put("restart/proof.txt", b"survives service restart")
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires the same disposable service after the script restarts it"]
async fn private_objects_persist_after_service_restart() {
    let storage = storage(false);
    assert_eq!(
        storage.get("restart/proof.txt").await.unwrap(),
        b"survives service restart"
    );
    storage.delete("restart/proof.txt").await.unwrap();
    assert!(matches!(
        storage.metadata("restart/proof.txt").await,
        Err(StorageError::NotFound(_))
    ));
}
