//! Golden outputs of `html!`: the exact bytes every form of the grammar and
//! every interpolated value type writes, recorded from the 13.0.0-rc.1
//! expansion before 13.1 changed how the expansion builds its string. A change
//! to the generated code must keep each case byte for byte; a case that
//! changes on purpose needs its new expected text in `html_golden/expected.rs`.
//!
//! The cases are grouped by module: text and elements, attributes, values,
//! control blocks, and pages copied from the workspace (the Benchmarks
//! endpoints, Studio, the UI kit and the book). A case missing from the
//! expected table fails with the line to add, written as Rust.
#![allow(clippy::unwrap_used, clippy::expect_used)]
// The cases write unusual Rust on purpose (values borrowed twice, closures
// called in place, constant conditions).
#![allow(
    clippy::needless_borrow,
    clippy::const_is_empty,
    clippy::redundant_closure_call
)]

extern crate rullst_core as rullst;

use std::collections::BTreeMap;

use rullst::html::Markup;

#[path = "html_golden/attributes.rs"]
mod attributes;
#[path = "html_golden/control.rs"]
mod control;
#[path = "html_golden/expected.rs"]
mod expected;
#[path = "html_golden/pages.rs"]
mod pages;
#[path = "html_golden/text.rs"]
mod text;
#[path = "html_golden/values.rs"]
mod values;

/// One named output.
pub type Case = (&'static str, Markup);

fn all_cases() -> Vec<Case> {
    let mut cases = Vec::new();
    cases.extend(text::cases());
    cases.extend(attributes::cases());
    cases.extend(values::cases());
    cases.extend(control::cases());
    cases.extend(pages::cases());
    cases
}

#[test]
fn every_case_writes_its_golden_bytes() {
    let expected: BTreeMap<&str, &str> = expected::EXPECTED.iter().copied().collect();
    assert_eq!(
        expected.len(),
        expected::EXPECTED.len(),
        "the expected table names a case twice"
    );
    let cases = all_cases();
    let mut seen = BTreeMap::new();
    let mut failures = Vec::new();
    for (name, markup) in &cases {
        assert!(
            seen.insert(*name, ()).is_none(),
            "two cases are named {name}"
        );
        match expected.get(name) {
            Some(bytes) if *bytes == markup.as_str() => {}
            Some(bytes) => failures.push(format!(
                "{name}:\n  expected {bytes:?}\n  written  {:?}",
                markup.as_str()
            )),
            None => failures.push(format!("    ({name:?}, {:?}),", markup.as_str())),
        }
    }
    for name in expected.keys() {
        if !seen.contains_key(name) {
            failures.push(format!("{name}: in the expected table but not a case"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
