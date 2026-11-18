//! `V13-MESSAGING-CRON-POSIX`'s notes against the `cron` crate's recorded
//! occurrences in messaging's golden corpus, the evidence messaging's
//! `posix_equivalent()` is also checked against: the POSIX form a note gives
//! for an expression, or the expression itself when the rule reports
//! nothing, reproduces every recorded start under Core's evaluator. Notes
//! without a form (both day fields, joined fields) make no claim.

use super::super::alpha6_cron::advice;
use super::*;

/// Messaging's corpus (`cron_expression_vectors.txt`): expressions the
/// `cron` crate accepted, each with the occurrences it recorded after a few
/// starts. Packaged crates do not ship it.
fn corpus() -> Option<Vec<(String, Vec<(i64, Vec<Option<i64>>)>)>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../rullst-messaging/src/schedules/cron_expression_vectors.txt");
    let text = std::fs::read_to_string(path).ok()?;
    let mut corpus: Vec<(String, Vec<(i64, Vec<Option<i64>>)>)> = Vec::new();
    let mut accepted = false;
    for line in text
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
    {
        if let Some(rest) = line.strip_prefix('\t') {
            let (start, results) = rest.split_once('\t').unwrap();
            if !accepted || results.trim() == "error" {
                continue;
            }
            let mut recorded = Vec::new();
            for token in results.split_whitespace() {
                match token {
                    "end" => {
                        recorded.push(None);
                        break;
                    }
                    "error" => break,
                    _ => recorded.push(Some(token.parse().unwrap())),
                }
            }
            corpus
                .last_mut()
                .unwrap()
                .1
                .push((start.parse().unwrap(), recorded));
            continue;
        }
        let (expression, verdict) = line.rsplit_once('\t').unwrap();
        accepted = verdict == "ok";
        if accepted {
            corpus.push((expression.to_string(), Vec::new()));
        }
    }
    Some(corpus)
}

/// The POSIX expression a note gives, or the expression itself when the
/// rule reports nothing; `None` when the note gives no form.
fn claimed(expression: &str) -> Option<String> {
    match advice(expression) {
        None => Some(expression.split_whitespace().collect::<Vec<_>>().join(" ")),
        Some(note) => note
            .split("POSIX form for a schedule not yet stored: `")
            .nth(1)
            .and_then(|rest| rest.split('`').next())
            .map(str::to_string),
    }
}

#[test]
fn every_posix_form_a_note_claims_keeps_the_recorded_occurrences() {
    let Some(corpus) = corpus() else {
        return;
    };
    // jiff's range: the corpus also records starts beyond it.
    let range = -377_705_023_201_000..=253_402_207_200_000_i64;
    let mut checked = 0;
    let mut wrong = Vec::new();
    for (expression, starts) in &corpus {
        let Some(posix) = claimed(expression) else {
            continue;
        };
        checked += 1;
        for (start, recorded) in starts.iter().filter(|(start, _)| range.contains(start)) {
            let mut after = *start;
            for expected in recorded {
                let found =
                    rullst_core::scheduler::__test_hooks::cron_next_after(&posix, after).unwrap();
                if found != *expected {
                    wrong.push(format!("{expression:?} as {posix:?} after {start}"));
                    break;
                }
                match found {
                    Some(at) => after = at,
                    None => break,
                }
            }
        }
    }
    assert!(corpus.len() > 1_000, "{} expressions", corpus.len());
    assert!(checked > 400, "{checked} forms checked");
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
