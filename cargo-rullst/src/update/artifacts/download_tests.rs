use super::*;
use std::{
    cell::{Cell, RefCell},
    fs,
};

const BODY: &[u8] = b"candidate must never be executed";
const TARGET: &str = "x86_64-unknown-linux-gnu";

fn manifest() -> Vec<u8> {
    serde_json::to_vec(&super::super::tests::fixture_manifest()).unwrap()
}

#[test]
fn authentication_precedes_binary_download_and_exact_bytes_are_required() {
    let directory = tempfile::tempdir().unwrap();
    let authenticated = Cell::new(false);
    let requests = RefCell::new(Vec::new());
    let result = stage_into(
        directory.path(),
        &Version::new(12, 1, 0),
        TARGET,
        |name, limit, path| {
            requests.borrow_mut().push(name.to_string());
            let body = if name.ends_with(".json") {
                manifest()
            } else {
                assert!(authenticated.get());
                BODY.to_vec()
            };
            write_bounded(&body[..], limit, path)
        },
        |path, parsed| {
            assert_eq!(parsed.source_commit, "a".repeat(40));
            assert_eq!(fs::read(path).unwrap(), manifest());
            authenticated.set(true);
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(requests.borrow().len(), 3);
    result.verify_files(directory.path()).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for entry in fs::read_dir(directory.path()).unwrap() {
            assert_eq!(
                entry.unwrap().metadata().unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
}

#[test]
fn failed_provenance_or_changed_manifest_never_downloads_executables() {
    for mutation in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let calls = Cell::new(0);
        let result = stage_into(
            directory.path(),
            &Version::new(12, 1, 0),
            TARGET,
            |name, limit, path| {
                calls.set(calls.get() + 1);
                assert!(name.ends_with(".json"));
                write_bounded(&manifest()[..], limit, path)
            },
            |path, _| {
                if mutation {
                    fs::write(path, b"tampered")?;
                    Ok(())
                } else {
                    Err(ArtifactError::Invalid("fixture provenance rejected"))
                }
            },
        );
        assert!(result.is_err());
        assert_eq!(calls.get(), 1);
    }
}

#[test]
fn corrupt_or_truncated_binaries_cannot_become_successful_stages() {
    for bytes in [
        &b"candidate must never be executeD"[..],
        &BODY[..BODY.len() - 1],
    ] {
        let directory = tempfile::tempdir().unwrap();
        assert!(
            stage_into(
                directory.path(),
                &Version::new(12, 1, 0),
                TARGET,
                |name, limit, path| {
                    let body = if name.ends_with(".json") {
                        manifest()
                    } else {
                        bytes.to_vec()
                    };
                    write_bounded(&body[..], limit, path)
                },
                |_, _| Ok(())
            )
            .is_err()
        );
    }
}

#[test]
fn streaming_downloads_are_bounded_and_cannot_replace_existing_files() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("asset");
    assert!(write_bounded(&b"oversized"[..], 3, &path).is_err());
    assert!(fs::metadata(&path).unwrap().len() <= 3);
    fs::write(&path, "original").unwrap();
    assert!(write_bounded(&b"new"[..], 3, &path).is_err());
    assert_eq!(fs::read(&path).unwrap(), b"original");
    let empty = directory.path().join("empty");
    assert!(write_bounded(&b""[..], 3, &empty).is_err());
}

#[test]
fn redirects_are_https_bounded_and_restricted_to_official_asset_hosts() {
    for url in [
        "https://github.com/Rullst/Rullst/releases/download/v12.1.0/file",
        "https://release-assets.githubusercontent.com/github-production-release-asset/asset?signature=opaque",
    ] {
        let url = reqwest::Url::parse(url).unwrap();
        assert!(allowed_redirect(&url, 1));
        assert!(!allowed_redirect(&url, 3));
    }
    for url in [
        "http://github.com/file",
        "https://github.com.attacker.invalid/file",
        "https://attacker.invalid/github.com/file",
        "https://localhost/file",
        "https://127.0.0.1/file",
        "https://user@github.com/file",
        "https://github.com:444/file",
    ] {
        assert!(
            !allowed_redirect(&reqwest::Url::parse(url).unwrap(), 1),
            "accepted {url}"
        );
    }
    assert_eq!(
        asset_url(&Version::new(12, 1, 0), "file"),
        "https://github.com/Rullst/Rullst/releases/download/v12.1.0/file"
    );
}

#[test]
fn http_failures_and_oversized_content_length_never_create_assets() {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        time::Duration,
    };
    for response in [
        "HTTP/1.1 404 Not Found\r\nContent-Length: 4\r\nConnection: close\r\n\r\nnope",
        "HTTP/1.1 200 OK\r\nContent-Length: 999\r\nConnection: close\r\n\r\nlarge",
    ] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("asset");
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = [0u8; 4096];
            assert!(socket.read(&mut request).unwrap() > 0);
            socket.write_all(response.as_bytes()).unwrap();
        });
        // The test transport permits loopback HTTP; production client() is HTTPS-only.
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        assert!(fetch(&client, &format!("http://{address}/asset"), 8, &path).is_err());
        server.join().unwrap();
        assert!(!path.exists());
    }
}
