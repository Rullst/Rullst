//! Bounds of a monitor document's lists and labels.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::document::MonitorDocument;
use super::server_test::document;
use serde_json::{Value, json};

fn validate(value: &Value) -> Result<(), &'static str> {
    serde_json::from_value::<MonitorDocument>(value.clone())
        .unwrap()
        .validate()
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
