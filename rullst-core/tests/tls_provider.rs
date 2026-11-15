//! `Server` start-up installs aws-lc-rs as the rustls process default, and
//! the TLS connections of a project that has them still work with it: an
//! HTTPS client and, with a PostgreSQL driver, a TLS database connection.
//!
//! Process-wide state: this binary holds a single test, so no other test can
//! install a rustls provider first. Core's `tls` feature (which its
//! `strict-postgres`, `strict-mysql`, `drivers-all`, `storage-s3` and
//! `telemetry` features enable) builds rustls; SQLite-only builds have none.

#![cfg(feature = "tls")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use rullst_core::{Server, routes};
use rustls::crypto::{CryptoProvider, aws_lc_rs};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

const DEADLINE: Duration = Duration::from_secs(30);

fn kx_names(provider: &CryptoProvider) -> Vec<rustls::NamedGroup> {
    provider
        .kx_groups
        .iter()
        .map(|group| group.name())
        .collect()
}

async fn start() {
    // Bind an ephemeral port and stop at once.
    tokio::time::timeout(
        DEADLINE,
        Server::new(routes![get("/" => || async { "ok" })]).run_with_shutdown(0, async {}),
    )
    .await
    .expect("startup finishes within the deadline")
    .expect("the server starts");
}

/// A loopback TLS listener with a fresh self-signed certificate for
/// `127.0.0.1`, and that certificate in PEM for the client to trust.
async fn tls_listener() -> (TcpListener, TlsAcceptor, Vec<u8>) {
    let certificate = rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()]).unwrap();
    let root = certificate.cert.pem().into_bytes();
    let key = rustls::pki_types::PrivatePkcs8KeyDer::from(certificate.signing_key.serialize_der());
    // The server accepts only the post-quantum hybrid group, which aws-lc-rs
    // supports and ring does not: a completed handshake shows that the client
    // used aws-lc-rs.
    let provider = CryptoProvider {
        kx_groups: vec![aws_lc_rs::kx_group::X25519MLKEM768],
        ..aws_lc_rs::default_provider()
    };
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(provider))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(vec![certificate.cert.der().clone()], key.into())
        .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    (listener, TlsAcceptor::from(Arc::new(config)), root)
}

fn negotiated<IO>(stream: &tokio_rustls::server::TlsStream<IO>) -> Option<rustls::NamedGroup> {
    stream
        .get_ref()
        .1
        .negotiated_key_exchange_group()
        .map(|group| group.name())
}

/// Answers one HTTPS request with `ok` and returns the negotiated group.
async fn serve_one_https_request(
    listener: TcpListener,
    acceptor: TlsAcceptor,
) -> Option<rustls::NamedGroup> {
    let (stream, _) = listener.accept().await.unwrap();
    let mut stream = acceptor.accept(stream).await.unwrap();
    let group = negotiated(&stream);
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1024];
    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
        let read = stream.read(&mut buffer).await.unwrap();
        assert_ne!(read, 0, "the client closed before its request ended");
        request.extend_from_slice(&buffer[..read]);
    }
    stream
        .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\nconnection: close\r\n\r\nok")
        .await
        .unwrap();
    stream.shutdown().await.unwrap();
    group
}

/// An HTTPS request with reqwest, which uses the process default.
async fn https_request_negotiates_aws_lc_rs() {
    let (listener, acceptor, root) = tls_listener().await;
    let url = format!("https://{}/", listener.local_addr().unwrap());
    let server = tokio::spawn(serve_one_https_request(listener, acceptor));
    let client = reqwest::Client::builder()
        .no_proxy()
        .tls_backend_rustls()
        .add_root_certificate(reqwest::Certificate::from_pem(&root).unwrap())
        .build()
        .unwrap();
    let response = tokio::time::timeout(DEADLINE, client.get(&url).send())
        .await
        .expect("the HTTPS request finishes within the deadline")
        .expect("the HTTPS request succeeds");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(response.text().await.unwrap(), "ok");
    let group = tokio::time::timeout(DEADLINE, server)
        .await
        .expect("the HTTPS server finishes within the deadline")
        .unwrap();
    assert_eq!(group, Some(rustls::NamedGroup::X25519MLKEM768));
}

/// A PostgreSQL server that only upgrades the connection to TLS, reads the
/// startup message over it and refuses the login with a known message.
#[cfg(any(feature = "strict-postgres", feature = "drivers-all"))]
async fn refuse_one_postgres_login_over_tls(
    listener: TcpListener,
    acceptor: TlsAcceptor,
) -> Option<rustls::NamedGroup> {
    let (mut stream, _) = listener.accept().await.unwrap();
    let mut request = [0_u8; 8];
    stream.read_exact(&mut request).await.unwrap();
    // SSLRequest: length 8, code 80877103.
    assert_eq!(&request, b"\x00\x00\x00\x08\x04\xd2\x16\x2f");
    stream.write_all(b"S").await.unwrap();
    let mut stream = acceptor.accept(stream).await.unwrap();
    let group = negotiated(&stream);
    let mut length = [0_u8; 4];
    stream.read_exact(&mut length).await.unwrap();
    let mut startup = vec![0_u8; u32::from_be_bytes(length) as usize - 4];
    stream.read_exact(&mut startup).await.unwrap();
    // Protocol 3.0, sent only over the encrypted connection.
    assert_eq!(&startup[..4], &196_608_u32.to_be_bytes());
    let fields = b"SFATAL\0VFATAL\0C28000\0Mtls fixture reached\0\0";
    let mut refusal = vec![b'E'];
    refusal.extend_from_slice(&(fields.len() as u32 + 4).to_be_bytes());
    refusal.extend_from_slice(fields);
    stream.write_all(&refusal).await.unwrap();
    let _ = stream.shutdown().await;
    group
}

/// A verified TLS PostgreSQL connection with SQLx, which selects aws-lc-rs.
#[cfg(any(feature = "strict-postgres", feature = "drivers-all"))]
async fn postgres_tls_negotiates_aws_lc_rs() {
    use sqlx::Connection;
    use sqlx::postgres::{PgConnectOptions, PgConnection, PgSslMode};

    let (listener, acceptor, root) = tls_listener().await;
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(refuse_one_postgres_login_over_tls(listener, acceptor));
    let options = PgConnectOptions::new()
        .host("127.0.0.1")
        .port(port)
        .username("rullst")
        .database("rullst")
        .ssl_mode(PgSslMode::VerifyFull)
        .ssl_root_cert_from_pem(root);
    let error = tokio::time::timeout(DEADLINE, PgConnection::connect_with(&options))
        .await
        .expect("the database connection finishes within the deadline")
        .expect_err("the fixture refuses the login");
    assert!(error.to_string().contains("tls fixture reached"), "{error}");
    let group = tokio::time::timeout(DEADLINE, server)
        .await
        .expect("the database server finishes within the deadline")
        .unwrap();
    assert_eq!(group, Some(rustls::NamedGroup::X25519MLKEM768));
}

#[tokio::test]
async fn server_startup_installs_aws_lc_rs_for_https_and_database_tls() {
    assert!(CryptoProvider::get_default().is_none());

    start().await;
    let installed = CryptoProvider::get_default().expect("start-up installs a provider");
    // aws-lc-rs, not ring: only aws-lc-rs offers the post-quantum hybrid group.
    assert_eq!(
        kx_names(installed),
        kx_names(&aws_lc_rs::default_provider())
    );
    assert!(kx_names(installed).contains(&rustls::NamedGroup::X25519MLKEM768));

    https_request_negotiates_aws_lc_rs().await;
    #[cfg(any(feature = "strict-postgres", feature = "drivers-all"))]
    postgres_tls_negotiates_aws_lc_rs().await;

    // An installed provider, such as an application's own, stays in force.
    start().await;
    let current = CryptoProvider::get_default().expect("provider still installed");
    assert!(Arc::ptr_eq(installed, current));
}
