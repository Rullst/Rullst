use super::*;
use crate::ai::backend::Backend;
use crate::ai::input::Input;
use crate::ai::mock::MockAssistant;
use crate::ai::session::{Mode, Session, Settings};
use crate::ai::term::Style;
use rullst_ai::StreamingAiClient;
use std::io::{Read, Write};
use std::net::TcpListener;

const ID: &str = "0123456789abcdef0123456789abcdef";

fn context(message: &str, file: &str) -> ErrorContext {
    ErrorContext {
        schema: SCHEMA.to_string(),
        id: ID.to_string(),
        message: message.to_string(),
        file: Some(file.to_string()),
        line: Some(2),
        backtrace: vec!["app::show at ./src/main.rs:2:5".to_string()],
        method: "GET".to_string(),
        path: "/users/7".to_string(),
    }
}

fn project() -> (tempfile::TempDir, std::path::PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(directory.path()).unwrap();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("src/main.rs"),
        "fn main() {\n    let id: u32 = \"x\".parse().unwrap();\n}\n",
    )
    .unwrap();
    (directory, root)
}

#[test]
fn ids_have_the_console_shape() {
    assert!(valid_id(ID));
    for id in [
        "",
        "0123",
        "../../_rullst/dev-telemetry",
        &ID.to_uppercase(),
    ] {
        assert!(!valid_id(id), "{id}");
    }
}

#[test]
fn only_loopback_sources_are_accepted() {
    let port = || Ok(4321);
    assert_eq!(
        source_url(None, port).unwrap().as_str(),
        "http://127.0.0.1:4321/"
    );
    for (raw, expected) in [
        ("http://127.0.0.1:3000", "http://127.0.0.1:3000/"),
        ("http://[::1]:3000/", "http://[::1]:3000/"),
        ("http://LOCALHOST:8080", "http://127.0.0.1:8080/"),
        ("http://127.0.0.2:3000", "http://127.0.0.2:3000/"),
    ] {
        assert_eq!(source_url(Some(raw), port).unwrap().as_str(), expected);
    }
    for raw in [
        "http://192.0.2.10:3000",
        "http://[::ffff:192.0.2.10]:3000",
        "http://example.com:3000",
        "http://localhost.example.com:3000",
        "https://127.0.0.1:3000",
        "http://user:pass@127.0.0.1:3000",
        "http://127.0.0.1:3000/other",
        "http://127.0.0.1:3000/?x=1",
        "file:///etc/passwd",
        "not a url",
    ] {
        let error = source_url(Some(raw), port).unwrap_err();
        assert!(error.contains("refusing"), "{raw}: {error}");
    }
    let error = source_url(None, || Err(std::io::Error::other("bad PORT"))).unwrap_err();
    assert!(error.contains("--url"), "{error}");
}

/// Serves one HTTP response on loopback and returns the request line.
fn serve_once(status: &str, body: String) -> (String, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let status = status.to_string();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = Vec::new();
        let mut buffer = [0u8; 1024];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            let read = stream.read(&mut buffer).unwrap();
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
        }
        let response = format!(
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).unwrap();
        String::from_utf8_lossy(&request)
            .lines()
            .next()
            .unwrap_or("")
            .to_string()
    });
    (base, handle)
}

#[tokio::test]
async fn the_context_is_read_from_the_error_endpoint() {
    let body = serde_json::json!({
        "schema": SCHEMA, "id": ID, "message": "boom", "file": "src/main.rs", "line": 2,
        "backtrace": [], "method": "GET", "path": "/users/7", "expires_in_seconds": 1700
    })
    .to_string();
    let (base, server) = serve_once("200 OK", body);
    let url = source_url(Some(&base), || Ok(0)).unwrap();
    let fetched = fetch(&url, ID).await.unwrap();
    assert_eq!(fetched.message, "boom");
    assert_eq!(
        server.join().unwrap(),
        format!("GET /_rullst/errors/{ID} HTTP/1.1")
    );

    let (base, server) = serve_once("404 Not Found", String::new());
    let url = source_url(Some(&base), || Ok(0)).unwrap();
    let error = fetch(&url, ID).await.unwrap_err();
    assert!(error.contains("unknown or expired"), "{error}");
    server.join().unwrap();

    let other = serde_json::json!({
        "schema": SCHEMA, "id": "f".repeat(32), "message": "x", "file": null, "line": null,
        "method": "GET", "path": "/"
    })
    .to_string();
    let (base, server) = serve_once("200 OK", other);
    let url = source_url(Some(&base), || Ok(0)).unwrap();
    assert!(fetch(&url, ID).await.unwrap_err().contains("different"));
    server.join().unwrap();
}

#[test]
fn the_brief_quotes_the_context_as_redacted_data() {
    let (_guard, root) = project();
    let secret = format!("{}{}", "AKIA", "Q7W3E9R2T5Y8U1I4");
    let prepared = brief(
        &root,
        &context(&format!("called unwrap with key {secret}"), "src/main.rs"),
    )
    .unwrap();
    let attachments = prepared.brief.attachments.join("\n");
    assert!(attachments.starts_with("<untrusted-data source=\"error-context\">"));
    assert!(attachments.contains("location: src/main.rs:2"));
    assert!(attachments.contains("request: GET /users/7"));
    assert!(attachments.contains("<untrusted-data source=\"file_src/main.rs\">"));
    assert!(!attachments.contains(&secret), "the secret is redacted");
    assert!(attachments.contains("[redacted: AWS access key ID]"));
    assert!(prepared.brief.instructions.starts_with(FIX_HEADING));
    assert!(
        prepared
            .brief
            .notes
            .iter()
            .any(|note| note.contains("1 secret-like value"))
    );
    assert!(
        prepared
            .summary
            .starts_with(&format!("Error {ID} · GET /users/7"))
    );

    // A location outside the project is not read.
    let outside = brief(&root, &context("boom", "/etc/hostname")).unwrap();
    assert_eq!(outside.brief.attachments.len(), 1);
    assert!(outside.brief.notes[0].contains("was not shared"));
}

#[tokio::test]
async fn an_offline_session_starts_with_the_error_context_as_quoted_data() {
    let (_guard, root) = project();
    let prepared = brief(&root, &context("boom at `parse`", "src/main.rs")).unwrap();
    let backend = Backend::Mock(StreamingAiClient::new(MockAssistant));
    let settings = Settings {
        label: "offline mock".to_string(),
        notice: None,
        style: Style { color: false },
        mode: Mode::PlanOnly("not an interactive terminal"),
        root: Some(root.clone()),
        cwd: root.clone(),
        prices: None,
    };
    let mut session = Session::new(&backend, settings, Vec::new(), Input::closed());
    session
        .briefed(&prepared.summary, prepared.brief, GOAL)
        .await;
    let output = String::from_utf8(session.into_output()).unwrap();
    assert!(
        output.contains(&format!("Error {ID} · GET /users/7")),
        "{output}"
    );
    // The offline assistant read these values from the quoted context block.
    assert!(
        output.contains("I received the recorded error as quoted data"),
        "{output}"
    );
    assert!(output.contains("- panic: `boom at 'parse'`"), "{output}");
    assert!(output.contains("- location: `src/main.rs:2`"), "{output}");
    assert!(output.contains("cargo check"), "{output}");
    let main = std::fs::read_to_string(root.join("src/main.rs")).unwrap();
    assert!(main.contains(".unwrap()"), "nothing was edited");
}
