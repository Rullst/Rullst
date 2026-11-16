//! Bounds of the project list's file format: unknown fields per object,
//! known fields, the file size (`TM-STUDIO-3`, `REMOTE-03`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::projects::{ProjectError, ProjectStore};
use super::stored::{MAX_FILE_BYTES, StoredFile};
use super::stored_tests::{edited, file_with_unknown_fields, store_with_shop, write_json};
use serde_json::{Value, json};

/// Where unknown fields can be, as JSON pointers into
/// [`file_with_unknown_fields`].
const OBJECTS: [&str; 6] = [
    "",
    "/projects/0",
    "/projects/0/rules",
    "/settings",
    "/settings/notifications",
    "/settings/proxy",
];

fn with_fields(token: &str, pointer: &str, fields: &[(String, Value)]) -> Value {
    let mut file = file_with_unknown_fields(token);
    let object = file.pointer_mut(pointer).unwrap().as_object_mut().unwrap();
    // Only the fields under test are unknown in this object.
    object.retain(|name, _| {
        matches!(
            name.as_str(),
            "version"
                | "projects"
                | "settings"
                | "name"
                | "base_url"
                | "token"
                | "rules"
                | "p95_ms"
                | "cooldown_minutes"
                | "notifications"
                | "proxy"
                | "enabled"
                | "quiet_minutes"
                | "mode"
        )
    });
    for (name, value) in fields {
        object.insert(name.clone(), value.clone());
    }
    file
}

fn loads(store: &ProjectStore, file: &Value) -> bool {
    write_json(store, file);
    match store.load() {
        Ok(_) => true,
        Err(error) => {
            assert_eq!(error, ProjectError::InvalidFile);
            false
        }
    }
}

#[test]
fn unknown_fields_are_bounded_in_every_object() {
    let (_directory, store, token) = store_with_shop("extra-bounds");
    let numbered = |count: usize| -> Vec<(String, Value)> {
        (0..count)
            .map(|index| (format!("f{index}"), json!(index)))
            .collect()
    };
    // `{"pad":"…"}` is 10 bytes plus the padding.
    let padded = |bytes: usize| vec![("pad".to_string(), json!("x".repeat(bytes - 10)))];
    for pointer in OBJECTS {
        let file = |fields: &[(String, Value)]| with_fields(&token, pointer, fields);
        assert!(loads(&store, &file(&numbered(32))), "{pointer}");
        assert!(!loads(&store, &file(&numbered(33))), "{pointer}");
        assert!(
            loads(&store, &file(&[("n".repeat(64), json!(1))])),
            "{pointer}"
        );
        assert!(
            !loads(&store, &file(&[("n".repeat(65), json!(1))])),
            "{pointer}"
        );
        assert!(loads(&store, &file(&padded(16 * 1024))), "{pointer}");
        assert!(!loads(&store, &file(&padded(16 * 1024 + 1))), "{pointer}");
    }
}

#[test]
fn known_fields_keep_their_types_and_bounds() {
    let (_directory, store, token) = store_with_shop("known-bounds");
    let base = file_with_unknown_fields(&token);
    assert!(loads(&store, &base));
    for (pointer, value) in [
        ("/projects/0/name", json!("-shop")),
        ("/projects/0/base_url", json!("http://shop.example/")),
        ("/projects/0/watch", json!("yes")),
        ("/projects/0/rules/p95_ms", json!(-1)),
        ("/projects/0/rules/cooldown_minutes", json!(0)),
        ("/settings/notifications/quiet_minutes", json!(0)),
        ("/settings/notifications/quiet_minutes", json!(2.5)),
        ("/settings/proxy/mode", json!(5)),
        ("/settings/proxy/no_proxy", json!(["a"])),
    ] {
        assert!(!loads(&store, &edited(&base, pointer, value)), "{pointer}");
    }
    let shop = base["projects"][0].clone();
    let many = |count: usize| {
        let mut file = base.clone();
        file["projects"] = Value::Array(
            (0..count)
                .map(|index| {
                    let mut project = shop.clone();
                    project["name"] = json!(format!("p{index}"));
                    project
                })
                .collect(),
        );
        file
    };
    assert!(loads(&store, &many(32)));
    assert!(!loads(&store, &many(33)));
}

#[test]
fn the_writer_refuses_a_file_too_large_to_read_back() {
    let file_with = |padding: usize| -> StoredFile {
        serde_json::from_value(json!({
            "version": 1,
            "projects": [],
            "pad": "x".repeat(padding)
        }))
        .unwrap()
    };
    let small = file_with(0).encode().unwrap().len();
    // The reader's bound: a larger file could not be read back.
    let largest = file_with(MAX_FILE_BYTES - small).encode().unwrap();
    assert_eq!(largest.len(), MAX_FILE_BYTES);
    assert_eq!(MAX_FILE_BYTES, 256 * 1024);
    assert_eq!(
        file_with(MAX_FILE_BYTES - small + 1).encode().unwrap_err(),
        ProjectError::Storage
    );
}
