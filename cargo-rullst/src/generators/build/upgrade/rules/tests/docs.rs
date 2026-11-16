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

/// The assisted-upgrade tutorial, which classifies the rows and dates the rules.
const TUTORIAL: &str = "tutorials/36-assisted-framework-upgrades.md";

/// A page's prose: fenced code blocks dropped and whitespace collapsed,
/// because sentences wrap anywhere in the Markdown source.
fn prose(text: &str) -> String {
    let mut fenced = false;
    text.lines()
        .filter(|line| {
            let fence = line.trim_start().starts_with("```");
            fenced ^= fence;
            !fence && !fenced
        })
        .flat_map(str::split_whitespace)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Every total the prose states: a number followed by `rules`, or a
/// parenthesized `(N rows)`.
fn stated_totals(prose: &str) -> Vec<String> {
    let words = prose.split(' ').collect::<Vec<_>>();
    words
        .windows(2)
        .filter(|pair| {
            let parenthesized = pair[0].strip_prefix('(');
            let number = parenthesized.unwrap_or(pair[0]);
            !number.is_empty()
                && number.bytes().all(|byte| byte.is_ascii_digit())
                && (pair[1].starts_with("rules")
                    || (parenthesized.is_some() && pair[1].starts_with("rows)")))
        })
        .map(|pair| pair.join(" "))
        .collect()
}

/// The `V13-*` codes written in backticks in `text`.
fn backticked_codes(text: &str) -> BTreeSet<&str> {
    text.split('`')
        .skip(1)
        .step_by(2)
        .filter(|code| code.starts_with("V13-"))
        .collect()
}

fn codes_introduced_in(release: &str) -> BTreeSet<&'static str> {
    CATALOG
        .iter()
        .filter(|rule| rule.introduced == release)
        .map(|rule| rule.code)
        .collect()
}

#[test]
fn every_migration_row_is_classified_once() {
    let (Some(guide), Some(tutorial)) =
        (repository_doc("migration-v13.md"), repository_doc(TUTORIAL))
    else {
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
    let Some(tutorial) = repository_doc(TUTORIAL) else {
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

/// Totals of rows or rules changed with every new row or rule, so parallel
/// pull requests conflicted on them; the pages state none. The tables are the
/// counts. The one number left, the alpha.1 rules of "Rules by release", is
/// checked against the catalog below.
#[test]
fn the_pages_state_no_row_or_rule_totals() {
    let alpha1 = codes_introduced_in("13.0.0-alpha.1").len();
    for (page, allowed) in [
        (TUTORIAL, vec![format!("{alpha1} rules")]),
        ("cli_reference.md", Vec::new()),
        ("migration-v13.md", Vec::new()),
    ] {
        let Some(text) = repository_doc(page) else {
            return;
        };
        let totals = stated_totals(&prose(&text))
            .into_iter()
            .map(|total| total.trim_end_matches(['.', ',', ';', '|']).to_string())
            .collect::<Vec<_>>();
        assert_eq!(
            totals, allowed,
            "{page}: state no row or rule totals; the tables and the catalog are the counts"
        );
    }
}

#[test]
fn the_rules_by_release_table_lists_the_rules_of_each_release() {
    let Some(tutorial) = repository_doc(TUTORIAL) else {
        return;
    };
    let table = table_rows(&tutorial, "### Rules by release");
    let releases = table
        .iter()
        .map(|cells| cells[0].as_str())
        .collect::<BTreeSet<_>>();
    for rule in CATALOG {
        assert!(
            releases.contains(rule.introduced),
            "{}: release {} has no row",
            rule.code,
            rule.introduced
        );
    }
    for cells in &table {
        let (release, listed) = (cells[0].as_str(), cells[1].as_str());
        let introduced = codes_introduced_in(release);
        if release == "13.0.0-alpha.1" {
            assert_eq!(listed, format!("The other {} rules", introduced.len()));
        } else {
            assert_eq!(backticked_codes(listed), introduced, "{release}");
        }
    }
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
}

#[test]
fn the_helpers_find_totals_and_codes_outside_code_blocks() {
    let text = "The catalog has\n  164 rules. Four rules,\n(34 rows); 26 rows tall (x rows)\n```text\nskipped: 100 rules\n```\n(7\nrules)";
    assert_eq!(
        prose(text),
        "The catalog has 164 rules. Four rules, (34 rows); 26 rows tall (x rows) (7 rules)"
    );
    assert_eq!(
        stated_totals(&prose(text)),
        ["164 rules.", "(34 rows);", "(7 rules)"]
    );
    assert_eq!(
        backticked_codes("`V13-A` (reworded: `rullst-orm`), `V13-B`, V13-C"),
        BTreeSet::from(["V13-A", "V13-B"])
    );
}
