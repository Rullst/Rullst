//! The migration guide, the tutorial's rule classification and the catalog
//! describe the same rows, rules and counts.

use super::*;

fn repository_doc(relative: &str) -> Option<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../docs/src")
        .join(relative);
    // Packaged crates do not ship the repository documentation.
    std::fs::read_to_string(path).ok()
}

fn is_heading(line: &str) -> bool {
    let hashes = line.chars().take_while(|c| *c == '#').count();
    hashes > 0 && line[hashes..].starts_with(' ')
}

/// The cells of the table rows between `heading` and the next heading,
/// without header and separator rows.
fn table_rows(text: &str, heading: &str) -> Vec<Vec<String>> {
    let lines = text
        .lines()
        .skip_while(|line| *line != heading)
        .skip(1)
        .take_while(|line| !is_heading(line))
        .collect::<Vec<_>>();
    let separator = |line: &&str| line.starts_with("| :");
    lines
        .iter()
        .enumerate()
        .filter(|(index, line)| {
            line.starts_with("| ")
                && !separator(line)
                && !lines.get(index + 1).is_some_and(separator)
        })
        .map(|(_, line)| {
            let cells = line
                .replace("\\|", "\u{0}")
                .split('|')
                .map(|cell| cell.trim().replace('\u{0}', "|"))
                .collect::<Vec<_>>();
            cells[1..cells.len() - 1].to_vec()
        })
        .collect()
}

fn migration_rows(guide: &str) -> Vec<String> {
    table_rows(guide, "## Changes from the published 12.1.0 source")
        .into_iter()
        .map(|cells| cells[0].clone())
        .collect()
}

fn classification(tutorial: &str) -> Vec<Vec<String>> {
    table_rows(tutorial, "## v12 → v13 rule classification")
}

/// The number in the first `(N rows)` after `label`.
fn stated_rows(tutorial: &str, label: &str) -> usize {
    let start = tutorial.find(label).expect("label") + label.len();
    let rest = &tutorial[start..];
    let end = rest.find(" rows)").expect("row count");
    let open = rest[..end].rfind('(').expect("opening parenthesis");
    rest[open + 1..end].parse().expect("row count number")
}

#[test]
fn every_migration_row_is_classified_once() {
    let (Some(guide), Some(tutorial)) = (
        repository_doc("migration-v13.md"),
        repository_doc("tutorials/36-assisted-framework-upgrades.md"),
    ) else {
        return;
    };
    let rows = migration_rows(&guide);
    assert!(rows.len() > 150, "parsed {} migration rows", rows.len());
    let classified = classification(&tutorial)
        .into_iter()
        .map(|cells| cells[0].clone())
        .collect::<Vec<_>>();
    let unique = classified.iter().collect::<BTreeSet<_>>();
    assert_eq!(unique.len(), classified.len(), "a row is classified twice");
    let missing = rows
        .iter()
        .filter(|row| !unique.contains(row))
        .collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "unclassified migration rows: {missing:?}"
    );
    let stale = classified
        .iter()
        .filter(|row| !rows.contains(row))
        .collect::<Vec<_>>();
    assert!(
        stale.is_empty(),
        "classified rows missing from the guide: {stale:?}"
    );
}

#[test]
fn classified_rules_exist_and_every_rule_is_listed_on_its_row() {
    let Some(tutorial) = repository_doc("tutorials/36-assisted-framework-upgrades.md") else {
        return;
    };
    let table = classification(&tutorial);
    let codes = CATALOG
        .iter()
        .map(|rule| rule.code)
        .collect::<BTreeSet<_>>();
    for cells in &table {
        let (row, class, rules) = (&cells[0], &cells[1], &cells[2]);
        let listed = rules
            .split(',')
            .map(|code| code.trim().trim_matches('`'))
            .filter(|code| code.starts_with("V13-"))
            .collect::<Vec<_>>();
        for code in &listed {
            assert!(codes.contains(code), "{row}: unknown rule {code}");
        }
        if class == "(c) none" {
            assert_eq!(rules, "—", "{row}: a (c) row lists no rule");
        } else {
            assert!(!listed.is_empty(), "{row}: an (a)/(b) row names its rules");
        }
    }
    for rule in CATALOG {
        let row = table
            .iter()
            .find(|cells| cells[0] == rule.row)
            .unwrap_or_else(|| panic!("{}: row `{}` is not classified", rule.code, rule.row));
        assert!(
            row[2].contains(&format!("`{}`", rule.code)),
            "{} is not listed on `{}`",
            rule.code,
            rule.row
        );
    }
}

#[test]
fn the_stated_counts_match_the_classification_and_the_catalog() {
    let Some(tutorial) = repository_doc("tutorials/36-assisted-framework-upgrades.md") else {
        return;
    };
    let table = classification(&tutorial);
    let count = |class: &str| {
        table
            .iter()
            .filter(|cells| cells[1].contains(class))
            .count()
    };
    // Sentences wrap anywhere in the Markdown source.
    let prose = tutorial.split_whitespace().collect::<Vec<_>>().join(" ");
    assert_eq!(stated_rows(&prose, "**(a) must-change**"), count("(a)"));
    assert_eq!(stated_rows(&prose, "**(b) review**"), count("(b)"));
    assert_eq!(stated_rows(&prose, "**(c) none**"), count("(c)"));
    let catalog = prose
        .split("The catalog has ")
        .nth(1)
        .and_then(|rest| rest.split(' ').next())
        .and_then(|number| number.parse::<usize>().ok());
    assert_eq!(catalog, Some(CATALOG.len()));
}

#[test]
fn the_table_parser_reads_cells_and_stops_at_the_next_heading() {
    let text = "## Table\n\n| Row | Class | Rules |\n| :--- | :--- | :--- |\n| A `x \\| y` | (b) review | `V13-A` |\n| B | (c) none | — |\n\n#[not a heading]\n\n### Sub\n\n| C | D | E |\n";
    assert_eq!(
        table_rows(text, "## Table"),
        vec![
            vec![
                "A `x | y`".to_string(),
                "(b) review".to_string(),
                "`V13-A`".to_string()
            ],
            vec!["B".to_string(), "(c) none".to_string(), "—".to_string()],
        ]
    );
    assert_eq!(table_rows(text, "### Sub"), vec![vec!["C", "D", "E"]]);
    assert_eq!(
        stated_rows(
            "**(a) x**: text (with words)\n  more (12 rows);",
            "**(a) x**"
        ),
        12
    );
}
