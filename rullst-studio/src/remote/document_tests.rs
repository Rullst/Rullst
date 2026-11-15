//! Bounds of a monitor document's lists and labels.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::document::MonitorDocument;
use super::server_test::document;
use serde_json::{Value, json};

fn validate(value: &Value) -> Result<(), &'static str> {
    serde_json::from_value::<MonitorDocument>(value.clone())
        .unwrap()
        .validate("rullst.monitor.v1")
}

fn repeated(value: &mut Value, pointer: &str, count: usize) {
    let list = value.pointer_mut(pointer).unwrap();
    let first = list[0].clone();
    *list = Value::Array(vec![first; count]);
}

#[test]
fn lists_may_fill_but_not_exceed_their_bounds() {
    for (pointer, bound) in [
        ("/http/routes", 512),
        ("/queues", 32),
        ("/database/n_plus_one/findings", 128),
    ] {
        let mut value = document("/orders/{id}");
        repeated(&mut value, pointer, bound);
        assert_eq!(validate(&value), Ok(()), "{pointer}");
        repeated(&mut value, pointer, bound + 1);
        assert_eq!(validate(&value), Err("too many entries"), "{pointer}");
    }
}

#[test]
fn labels_may_fill_but_not_exceed_512_characters() {
    let longest = "é".repeat(512);
    assert_eq!(validate(&document(&longest)), Ok(()));
    assert_eq!(
        validate(&document(&format!("{longest}x"))),
        Err("label too long")
    );
    let mut unavailable = document("/");
    unavailable["database"] = json!({"state": "unavailable", "reason": "x".repeat(513)});
    assert_eq!(validate(&unavailable), Err("label too long"));
}

fn validate_v2(value: &Value) -> Result<(), &'static str> {
    serde_json::from_value::<MonitorDocument>(value.clone())
        .unwrap()
        .validate("rullst.monitor.v2")
}

#[test]
fn version_2_lists_may_fill_but_not_exceed_their_bounds() {
    for (pointer, bound) in [
        ("/traces/requests", 64),
        ("/traces/spans/kinds", 8),
        ("/security/events", 64),
    ] {
        let mut value = super::server_test::document_v2("/orders/{id}");
        repeated(&mut value, pointer, bound);
        assert_eq!(validate_v2(&value), Ok(()), "{pointer}");
        repeated(&mut value, pointer, bound + 1);
        assert_eq!(validate_v2(&value), Err("too many entries"), "{pointer}");
    }
    let mut value = super::server_test::document_v2("/");
    let counters = |count: usize| {
        Value::Object(
            (0..count)
                .map(|index| (format!("c{index}"), json!(1)))
                .collect(),
        )
    };
    value["security"]["counters"] = counters(64);
    assert_eq!(validate_v2(&value), Ok(()));
    value["security"]["counters"] = counters(65);
    assert_eq!(validate_v2(&value), Err("too many entries"));
}

#[test]
fn version_2_labels_are_bounded_and_its_sections_required() {
    let longest = "é".repeat(512);
    assert_eq!(
        validate_v2(&super::server_test::document_v2(&longest)),
        Ok(())
    );
    for pointer in [
        "/traces/requests/1/route",
        "/traces/requests/1/method",
        "/traces/spans/kinds/0/kind",
        "/security/events/0/type",
    ] {
        let mut value = super::server_test::document_v2("/");
        *value.pointer_mut(pointer).unwrap() = json!(format!("{longest}x"));
        assert_eq!(validate_v2(&value), Err("label too long"), "{pointer}");
    }
    let mut counter = super::server_test::document_v2("/");
    counter["security"]["counters"][format!("{longest}x")] = json!(1);
    assert_eq!(validate_v2(&counter), Err("label too long"));
    for section in ["traces", "security"] {
        let mut value = super::server_test::document_v2("/");
        value.as_object_mut().unwrap().remove(section);
        assert_eq!(
            validate_v2(&value),
            Err("missing version 2 section"),
            "{section}"
        );
    }
    // Each version is read only at its own path.
    assert_eq!(
        validate(&super::server_test::document_v2("/")),
        Err("unsupported schema")
    );
    assert_eq!(
        validate_v2(&super::server_test::document("/")),
        Err("unsupported schema")
    );
}
