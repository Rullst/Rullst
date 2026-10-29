//! The project list on disk: validation, encryption at rest and permissions
//! (`TM-STUDIO-3`, `REMOTE-03`, `REMOTE-04`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::crypto::SecretKey;
use super::projects::{
    ProjectError, ProjectStore, token_context, validate_base_url, validate_name, webhook_context,
};
use super::server_test::{TOKEN, TempDir};

#[test]
fn names_are_short_plain_labels() {
    for name in ["shop", "shop-production", "a.b_c-1", &"x".repeat(48)] {
        assert_eq!(validate_name(name), Ok(()), "{name}");
    }
    for name in [
        "",
        "-shop",
        ".hidden",
        "with space",
        "<b>",
        "ação",
        &"x".repeat(49),
    ] {
        assert_eq!(
            validate_name(name),
            Err(ProjectError::InvalidName),
            "{name}"
        );
    }
}

#[test]
fn base_urls_are_https_or_loopback_http_without_extras() {
    for (input, normalized) in [
        ("https://shop.example", "https://shop.example/"),
        ("https://shop.example/app", "https://shop.example/app/"),
        ("http://127.0.0.1:9000", "http://127.0.0.1:9000/"),
        ("http://localhost:9000/", "http://localhost:9000/"),
        ("http://[::1]:9000", "http://[::1]:9000/"),
    ] {
        assert_eq!(validate_base_url(input).unwrap().as_str(), normalized);
    }
    for input in [
        "http://shop.example",
        "http://10.0.0.5",
        "https://user:pass@shop.example",
        "https://shop.example/?debug=1",
        "https://shop.example/#top",
        "ftp://shop.example",
        "file:///etc/passwd",
        "javascript:alert(1)",
        "shop.example",
        "",
    ] {
        assert_eq!(
            validate_base_url(input),
            Err(ProjectError::InvalidUrl),
            "{input}"
        );
    }
    assert_eq!(
        validate_base_url(&format!("https://shop.example/{}", "a".repeat(600))),
        Err(ProjectError::InvalidUrl)
    );
}

#[test]
fn a_token_is_stored_encrypted_and_read_back() {
    let directory = TempDir::new("store");
    let store = ProjectStore::new(directory.paths());
    assert!(store.load().unwrap().is_empty());
    store.add("shop", "https://shop.example", TOKEN).unwrap();

    let file = std::fs::read_to_string(&directory.paths().projects).unwrap();
    assert!(file.contains("\"shop\""));
    assert!(!file.contains(TOKEN));
    let projects = store.load().unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].token.as_ref().unwrap().expose(), TOKEN);
    assert_eq!(
        projects[0].monitor_url().unwrap().as_str(),
        "https://shop.example/_rullst/monitor/v1"
    );
    assert!(!format!("{:?}", projects[0]).contains(TOKEN));
}

#[test]
fn duplicates_bad_input_and_a_full_list_are_refused() {
    let directory = TempDir::new("limits");
    let store = ProjectStore::new(directory.paths());
    store.add("shop", "https://shop.example", TOKEN).unwrap();
    assert_eq!(
        store.add("shop", "https://other.example", TOKEN),
        Err(ProjectError::Duplicate)
    );
    assert_eq!(
        store.add("x", "https://x.example", ""),
        Err(ProjectError::InvalidToken)
    );
    assert_eq!(
        store.add("x", "https://x.example", "has space"),
        Err(ProjectError::InvalidToken)
    );
    for index in 1..32 {
        store
            .add(&format!("p{index}"), "https://shop.example", TOKEN)
            .unwrap();
    }
    assert_eq!(
        store.add("one-more", "https://shop.example", TOKEN),
        Err(ProjectError::TooMany)
    );
    store.remove("p1").unwrap();
    assert_eq!(store.remove("p1"), Err(ProjectError::NotFound));
    assert_eq!(store.load().unwrap().len(), 31);
}

#[test]
fn an_edited_url_or_name_or_a_missing_key_makes_the_token_unreadable() {
    let directory = TempDir::new("tamper");
    let paths = directory.paths();
    let store = ProjectStore::new(paths.clone());
    store.add("shop", "https://shop.example", TOKEN).unwrap();
    store.add("blog", "https://blog.example", TOKEN).unwrap();

    // Point "shop" at another origin by editing the file.
    let original = std::fs::read_to_string(&paths.projects).unwrap();
    let edited = original.replace("https://shop.example/", "https://attacker.example/");
    std::fs::write(&paths.projects, &edited).unwrap();
    let projects = store.load().unwrap();
    assert!(projects[0].token.is_none());
    assert!(projects[1].token.is_some());

    // Swap the names: each sealed token is bound to its own name.
    let swapped = original
        .replace("\"shop\"", "\"tmp\"")
        .replace("\"blog\"", "\"shop\"")
        .replace("\"tmp\"", "\"blog\"");
    std::fs::write(&paths.projects, &swapped).unwrap();
    assert!(
        store
            .load()
            .unwrap()
            .iter()
            .all(|project| project.token.is_none())
    );

    std::fs::write(&paths.projects, &original).unwrap();
    std::fs::remove_file(&paths.key).unwrap();
    assert!(
        store
            .load()
            .unwrap()
            .iter()
            .all(|project| project.token.is_none())
    );
}

#[test]
fn sealed_values_are_bound_to_their_context_and_key() {
    let directory = TempDir::new("crypto");
    let key = SecretKey::load_or_create(&directory.path().join("k")).unwrap();
    let sealed = key.seal(TOKEN, b"context").unwrap();
    assert!(sealed.starts_with("v1."));
    assert_ne!(key.seal(TOKEN, b"context").unwrap(), sealed);
    assert_eq!(key.open(&sealed, b"context").unwrap().expose(), TOKEN);
    assert!(key.open(&sealed, b"other").is_err());
    // Change the first character of the ciphertext.
    let (head, ciphertext) = sealed.rsplit_once('.').unwrap();
    let first = if ciphertext.starts_with('A') {
        'B'
    } else {
        'A'
    };
    let tampered = format!("{head}.{first}{}", &ciphertext[1..]);
    assert!(key.open(&tampered, b"context").is_err());
    assert!(key.open("v2.abc.def", b"context").is_err());
    let other = SecretKey::load_or_create(&directory.path().join("other")).unwrap();
    assert!(other.open(&sealed, b"context").is_err());
    // The same key file gives the same key.
    let again = SecretKey::load_or_create(&directory.path().join("k")).unwrap();
    assert_eq!(again.open(&sealed, b"context").unwrap().expose(), TOKEN);
}

#[test]
fn a_malformed_or_oversized_file_is_reported_without_its_contents() {
    let directory = TempDir::new("malformed");
    let paths = directory.paths();
    let store = ProjectStore::new(paths.clone());
    store.add("shop", "https://shop.example", TOKEN).unwrap();
    std::fs::write(
        &paths.projects,
        "{\"version\": 1, \"projects\": [], \"extra\": true}",
    )
    .unwrap();
    assert_eq!(store.load().unwrap_err(), ProjectError::InvalidFile);
    std::fs::write(&paths.projects, " ".repeat(300 * 1024)).unwrap();
    assert_eq!(store.load().unwrap_err(), ProjectError::Storage);
    assert!(!ProjectError::InvalidFile.to_string().contains("extra"));
}

#[cfg(unix)]
#[test]
fn files_are_private_and_links_are_refused() {
    use std::os::unix::fs::PermissionsExt;
    let directory = TempDir::new("permissions");
    let paths = directory.paths();
    let store = ProjectStore::new(paths.clone());
    store.add("shop", "https://shop.example", TOKEN).unwrap();
    let mode =
        |path: &std::path::Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&paths.projects), 0o600);
    assert_eq!(mode(&paths.key), 0o600);
    assert_eq!(mode(paths.projects.parent().unwrap()), 0o700);
    assert_eq!(mode(paths.key.parent().unwrap()), 0o700);
    assert!(!store.readable_by_others());

    std::fs::set_permissions(&paths.projects, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(store.readable_by_others());

    let target = directory.path().join("elsewhere.json");
    std::fs::write(&target, "{\"version\": 1, \"projects\": []}").unwrap();
    std::fs::remove_file(&paths.projects).unwrap();
    std::os::unix::fs::symlink(&target, &paths.projects).unwrap();
    assert_eq!(store.load().unwrap_err(), ProjectError::Storage);
    assert_eq!(
        store.add("blog", "https://blog.example", TOKEN),
        Err(ProjectError::Storage)
    );
}

#[test]
fn secrets_never_print_and_sealed_values_are_bounded() {
    let directory = TempDir::new("crypto-bound");
    let key = SecretKey::load_or_create(&directory.path().join("k")).unwrap();
    // 3041 bytes seal to exactly the 4096-character bound.
    let largest = "s".repeat(3041);
    let sealed = key.seal(&largest, b"context").unwrap();
    assert_eq!(sealed.len(), 4096);
    let opened = key.open(&sealed, b"context").unwrap();
    assert_eq!(opened.expose(), largest);
    assert_eq!(format!("{opened:?}"), "[REDACTED]");
    let larger = key.seal(&format!("{largest}s"), b"context").unwrap();
    assert!(larger.len() > 4096);
    assert!(key.open(&larger, b"context").is_err());
}

#[test]
fn errors_and_sealing_contexts_are_fixed_text() {
    assert_eq!(
        ProjectError::NotFound.to_string(),
        "no project has this name"
    );
    assert_eq!(
        ProjectError::Storage.to_string(),
        "the project list or its key could not be read or written"
    );
    assert_eq!(
        webhook_context("shop", "https://shop.example/"),
        b"rullst-studio/webhook\nshop\nhttps://shop.example/".to_vec()
    );
    assert_eq!(
        token_context("shop", "https://shop.example/"),
        b"rullst-studio/token\nshop\nhttps://shop.example/".to_vec()
    );
}

#[test]
fn a_base_url_may_fill_512_characters_and_never_carries_credentials() {
    let longest = format!("https://shop.example/{}", "a".repeat(491));
    assert_eq!(longest.len(), 512);
    assert!(validate_base_url(&longest).is_ok());
    assert_eq!(
        validate_base_url(&format!("{longest}a")).unwrap_err(),
        ProjectError::InvalidUrl
    );
    for input in ["https://user@shop.example", "https://:x@shop.example"] {
        assert_eq!(
            validate_base_url(input).unwrap_err(),
            ProjectError::InvalidUrl,
            "{input}"
        );
    }
}

#[test]
fn a_file_with_another_version_or_an_invalid_name_is_malformed() {
    let directory = TempDir::new("stored-fields");
    let paths = directory.paths();
    let store = ProjectStore::new(paths.clone());
    store.add("shop", "https://shop.example", TOKEN).unwrap();
    let original: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&paths.projects).unwrap()).unwrap();
    for (pointer, value) in [
        ("/version", serde_json::json!(2)),
        ("/projects/0/name", serde_json::json!("-shop")),
    ] {
        let mut edited = original.clone();
        *edited.pointer_mut(pointer).unwrap() = value;
        std::fs::write(&paths.projects, serde_json::to_vec(&edited).unwrap()).unwrap();
        assert_eq!(
            store.load().unwrap_err(),
            ProjectError::InvalidFile,
            "{pointer}"
        );
    }
    std::fs::write(&paths.projects, serde_json::to_vec(&original).unwrap()).unwrap();
    assert_eq!(store.load().unwrap().len(), 1);
}
