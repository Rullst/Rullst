use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn empty_azure_credentials_use_offline_transport() {
    let driver = AzureCommunicationDriver::new(
        "https://fixture.communication.azure.com",
        StaticAzureMailCredential::new("", 0).unwrap(),
    )
    .unwrap();
    driver
        .send(
            &Message::new()
                .to("member@example.com")
                .subject("fixture")
                .text("welcome"),
        )
        .await
        .unwrap();
    let driver = AzureCommunicationDriver::new(
        "",
        StaticAzureMailCredential::new("unused", u64::MAX).unwrap(),
    )
    .unwrap();
    driver
        .send(&Message::new().to("member@example.com").subject("fixture"))
        .await
        .unwrap();
}

#[test]
fn azure_endpoint_and_payload_are_bounded_and_tracking_free() {
    for endpoint in [
        "http://fixture.communication.azure.com",
        "https://evil.example",
        "https://fixture.communication.azure.com.evil.example",
        "https://user:secret@fixture.communication.azure.com",
        "https://fixture.communication.azure.com/path",
    ] {
        assert!(
            AzureCommunicationDriver::new(endpoint, StaticAzureMailCredential::new("", 0).unwrap())
                .is_err()
        );
    }
    let message = Message::new()
        .to("member@example.com")
        .from("accounts@example.com")
        .subject("Reset")
        .text("https://app.example/reset?token=opaque123");
    let prepared = DeliveryPipeline::prepare(&message).unwrap();
    let value: Value = serde_json::from_slice(&payload(prepared.message()).unwrap()).unwrap();
    assert_eq!(value["userEngagementTrackingDisabled"], true);
    assert!(
        value["content"]["plainText"]
            .as_str()
            .unwrap()
            .contains("token=opaque123")
    );
    assert_eq!(value["recipients"]["to"].as_array().unwrap().len(), 1);
    assert!(payload(&Message::new().to("member@example.com")).is_err());
    assert!(
        !format!(
            "{:?}",
            AzureMailAccessToken::new("sensitive-token", 1000).unwrap()
        )
        .contains("sensitive-token")
    );
}

#[tokio::test]
async fn managed_identity_uses_local_header_and_scoped_resource() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/msi/token", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut bytes = vec![0; 8192];
        let read = socket.read(&mut bytes).await.unwrap();
        let request = String::from_utf8_lossy(&bytes[..read]);
        assert!(request.contains("resource=https%3A%2F%2Fcommunication.azure.com"));
        assert!(
            request
                .to_ascii_lowercase()
                .contains("x-identity-header: fixture_identity_header")
        );
        assert!(request.contains("api-version=2019-08-01"));
        let body = r#"{"access_token":"fixture_access_token","token_type":"Bearer","expires_on":"2000000000"}"#;
        socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
    });
    let credential = AzureManagedIdentity::new(endpoint, "fixture_identity_header", None).unwrap();
    let token = credential.access_token().await.unwrap();
    assert_eq!(token.value.expose_secret(), "fixture_access_token");
    assert_eq!(token.expires_at, 2000000000);
    server.await.unwrap();
    for endpoint in [
        "http://localhost/token",
        "http://evil.example/token",
        "http://127.0.0.1/token?injected=1",
        "https://127.0.0.1/token",
    ] {
        assert!(AzureManagedIdentity::new(endpoint, "fixture_identity_header", None).is_err());
    }
}
