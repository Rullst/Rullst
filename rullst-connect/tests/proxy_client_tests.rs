#![cfg(not(miri))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_connect::ConnectError;
use rullst_connect::client::{HttpClient, HttpRequest, ReqwestClient};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::time::Duration;

fn start_proxy_fixture() -> (String, mpsc::Receiver<String>, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind proxy fixture");
    let address = listener.local_addr().expect("proxy fixture address");
    let (request_sender, request_receiver) = mpsc::channel();
    let handle = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().expect("accept proxied request");
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("set fixture timeout");
        let mut request = Vec::new();
        let mut chunk = [0_u8; 1_024];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            let count = socket.read(&mut chunk).expect("read proxied request");
            if count == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..count]);
        }
        request_sender
            .send(String::from_utf8(request).expect("ASCII proxy request"))
            .expect("send captured request");
        socket
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 11\r\nConnection: close\r\n\r\n{\"ok\":true}",
            )
            .expect("write proxy response");
    });
    (format!("http://{address}"), request_receiver, handle)
}

#[tokio::test]
async fn explicit_authenticated_proxy_routes_request_and_sends_basic_auth() {
    let (proxy_url, captured, fixture) = start_proxy_fixture();
    let client = ReqwestClient::try_with_proxy_basic_auth(&proxy_url, "proxy_user", "proxy_pass")
        .expect("valid loopback authenticated proxy");
    let response = client
        .execute(HttpRequest {
            method: "GET".into(),
            url: "http://origin.example.invalid/oauth/token".to_string(),
            headers: reqwest::header::HeaderMap::new(),
            form: None,
            json: None,
            basic_auth: None,
            bearer_auth: None,
        })
        .await
        .expect("proxied response");

    assert_eq!(response.status, 200);
    assert_eq!(response.body["ok"], true);
    let request = captured
        .recv_timeout(Duration::from_secs(5))
        .expect("captured proxy request")
        .to_ascii_lowercase();
    assert!(request.starts_with("get http://origin.example.invalid/oauth/token http/1.1\r\n"));
    assert!(request.contains("proxy-authorization: basic chjvehlfdxnlcjpwcm94ev9wyxnz\r\n"));
    fixture.join().expect("proxy fixture");
}

#[test]
fn proxy_configuration_is_bounded_and_does_not_echo_credentials() {
    let embedded = ReqwestClient::try_with_proxy("http://proxy_user:do_not_echo@localhost:8080")
        .err()
        .expect("embedded proxy credentials must be rejected");
    assert!(!embedded.to_string().contains("do_not_echo"));

    let cleartext = ReqwestClient::try_with_proxy_basic_auth(
        "http://proxy.corp.example:8080",
        "proxy_user",
        "do_not_echo",
    )
    .err()
    .expect("authenticated remote HTTP proxy must be rejected");
    assert!(matches!(
        cleartext,
        ConnectError::InvalidConfiguration {
            field: "proxy_url",
            ..
        }
    ));
    assert!(!cleartext.to_string().contains("do_not_echo"));

    assert!(ReqwestClient::try_with_proxy("socks5://localhost:1080").is_err());
    assert!(ReqwestClient::try_with_proxy("https://localhost:8443/path").is_err());
}
