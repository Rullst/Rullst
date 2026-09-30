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
    let display = message.clone().from("Accounts <accounts@example.com>");
    let value: Value = serde_json::from_slice(&payload(&display).unwrap()).unwrap();
    assert_eq!(value["senderAddress"], "accounts@example.com");
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
    // Container Apps injects `http://localhost:<port>/msi/token`; App Service
    // uses a literal loopback IP. Both must reach the host-local endpoint.
    for host in ["127.0.0.1", "localhost"] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let endpoint = format!("http://{host}:{port}/msi/token");
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
        let credential =
            AzureManagedIdentity::new(endpoint, "fixture_identity_header", None).unwrap();
        let token = credential.access_token().await.unwrap();
        assert_eq!(token.value.expose_secret(), "fixture_access_token");
        assert_eq!(token.expires_at, 2000000000);
        server.await.unwrap();
    }
    for endpoint in [
        "http://localhost.evil.example/token",
        "http://evil-localhost/token",
        "https://localhost/token",
        "http://evil.example/token",
        "http://127.0.0.1/token?injected=1",
        "https://127.0.0.1/token",
    ] {
        assert!(AzureManagedIdentity::new(endpoint, "fixture_identity_header", None).is_err());
    }
}

#[test]
fn acs_send_url_keeps_the_resource_origin() {
    let endpoint = reqwest::Url::parse("https://fixture.communication.azure.com/").unwrap();
    assert_eq!(
        send_url(&endpoint).as_str(),
        "https://fixture.communication.azure.com/emails:send?api-version=2023-03-31"
    );
}

async fn read_fixture_request(socket: &mut tokio::net::TcpStream) -> String {
    let mut bytes = Vec::new();
    let mut chunk = [0; 4096];
    loop {
        let read = socket.read(&mut chunk).await.unwrap();
        assert!(read > 0);
        bytes.extend_from_slice(&chunk[..read]);
        let text = String::from_utf8_lossy(&bytes).to_string();
        if let Some(end) = text.find("\r\n\r\n") {
            let length = text[..end]
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())?
                })
                .unwrap_or(0);
            if bytes.len() >= end + 4 + length {
                return text;
            }
        }
    }
}

#[tokio::test]
async fn acs_send_posts_to_the_resource_and_polls_its_operation() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let id = "11111111-2222-3333-4444-555555555555";
    let operation = format!("{origin}/emails/operations/{id}?api-version=2023-03-31");
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let request = read_fixture_request(&mut socket).await;
        assert!(request.starts_with("POST /emails:send?api-version=2023-03-31 HTTP/1.1\r\n"));
        assert!(request.contains("\"senderAddress\":\"accounts@example.com\""));
        let body = format!(r#"{{"id":"{id}","status":"Running"}}"#);
        socket.write_all(format!("HTTP/1.1 202 Accepted\r\nOperation-Location: {operation}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
        drop(socket);
        let (mut socket, _) = listener.accept().await.unwrap();
        let request = read_fixture_request(&mut socket).await;
        assert!(request.starts_with(&format!(
            "GET /emails/operations/{id}?api-version=2023-03-31 HTTP/1.1\r\n"
        )));
        let body = format!(r#"{{"id":"{id}","status":"Succeeded"}}"#);
        socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
    });
    // Production endpoints must be HTTPS ACS hosts; the fixture injects a
    // loopback origin directly so the built request path is observable.
    let driver = AzureCommunicationDriver {
        endpoint: Some(reqwest::Url::parse(&format!("{origin}/")).unwrap()),
        credential: StaticAzureMailCredential::new("fixture_acs_token", u64::MAX).unwrap(),
    };
    let message = Message::new()
        .to("member@example.com")
        .from("accounts@example.com")
        .subject("Welcome")
        .text("hello");
    let outcome = tokio::time::timeout(std::time::Duration::from_secs(10), driver.send(&message))
        .await
        .unwrap();
    assert!(outcome.is_ok());
    server.await.unwrap();
}
