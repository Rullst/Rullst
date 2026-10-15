//! The database and feature choices as an 80-column terminal shows them.

use super::*;
use crate::ui::screen::{Choice, Cursor, Screen, Selection, frame};

/// The rows of a choice screen drawn at 80 × 40, as plain text.
fn rows(options: &[(&str, &str)], selection: Selection) -> Vec<String> {
    let screen = Screen {
        title: "New Rullst app".to_string(),
        crumb: String::new(),
        body: Vec::new(),
        question: String::new(),
        choices: options
            .iter()
            .map(|(name, hint)| Choice::new(*name, *hint))
            .collect(),
        selection,
        can_go_back: true,
        echo: None,
    };
    frame(&screen, &Cursor::new(&screen), 80, 40)
        .iter()
        .map(|line| line.text())
        .collect()
}

fn row<'a>(rows: &'a [String], label: &str) -> &'a str {
    rows.iter()
        .find(|row| row.contains(label))
        .unwrap_or_else(|| panic!("no {label} row in {rows:#?}"))
}

#[test]
fn the_turso_primary_states_its_limits_in_full_at_80_columns() {
    let options: Vec<(&str, &str)> = database_options(BLANK_BLUEPRINT_ID)
        .iter()
        .map(|option| (option.name, option.hint))
        .collect();
    let rows = rows(&options, Selection::One { initial: 0 });
    assert!(
        row(&rows, "Turso / libSQL")
            .ends_with("Turso / libSQL  Blank/API only; no Nexus, Studio, Schema or SQLx ORM"),
        "{rows:#?}"
    );
}

#[test]
fn every_side_store_says_so_in_full_at_80_columns() {
    let options: Vec<(&str, &str)> = FEATURES
        .iter()
        .map(|(_, name, hint)| (*name, *hint))
        .collect();
    let rows = rows(
        &options,
        Selection::Many {
            checked: vec![false; FEATURES.len()],
        },
    );
    for (label, hint) in [
        (
            "Redis adapters",
            "queues, cache and a Redis-structures side store",
        ),
        (
            "Turso / libSQL add-on",
            "side store: edge SQL; not the primary database",
        ),
        ("MongoDB", "side store: document CRUD"),
        ("DuckDB", "side store: analytics; longer first build"),
        (
            "SurrealDB",
            "side store: documents, read-only graph queries",
        ),
        ("Qdrant", "side store: dense-vector search"),
    ] {
        assert!(row(&rows, label).ends_with(hint), "{label}: {rows:#?}");
    }
}
