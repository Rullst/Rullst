//! 13.0.0-alpha.6 reads messaging schedules with Core's POSIX cron syntax:
//! weekdays 0 to 7, either day field when both are restricted, and fields
//! valid on their own.

use super::super::alpha6_cron::advice;
use super::*;

const CRON: &str = "V13-MESSAGING-CRON-POSIX";

/// The lines and notes of `CRON` in `source`.
fn notes(source: &str) -> Vec<(usize, Option<String>)> {
    let context = rust::FileContext {
        text: source,
        file_name: "lib.rs",
        path: Path::new("lib.rs"),
        facts: &WorkspaceFacts::default(),
    };
    let mut collector = Collector::default();
    rust::scan_source(&context, &mut collector).unwrap();
    let mut findings = Vec::new();
    collector.finish(Path::new("lib.rs"), &mut findings);
    findings
        .into_iter()
        .filter(|finding| finding.rule.code == CRON)
        .map(|finding| (finding.line, finding.note))
        .collect()
}

#[test]
fn the_rule_is_an_alpha6_must_change_rule() {
    let rule = CATALOG.iter().find(|rule| rule.code == CRON).unwrap();
    assert_eq!(rule.introduced, "13.0.0-alpha.6");
    assert_eq!(rule.kind, FindingKind::MustChange);
    assert_eq!(
        rule.row,
        "Messaging schedules use the scheduler's cron syntax"
    );
    assert!(
        rule.guidance
            .contains("`2-6`, Monday to Friday, becomes `1-5`")
    );
    assert!(rule.guidance.contains("`RecurringError::LegacyDefinition`"));
    assert!(rule.guidance.contains("`CronSyntax::Legacy`"));
    assert!(
        rule.guidance
            .contains("`RecurringDefinition::posix_equivalent()`")
    );
    assert!(
        rule.message
            .ends_with("Review the schedule this call creates")
    );
}

#[test]
fn weekday_numbers_move_down_by_one() {
    let posix = |expression: &str| {
        format!(
            "POSIX form for a schedule not yet stored: `{expression}` (weekday numbers move down by one: Sunday is 0 or 7, Monday 1, Saturday 6)"
        )
    };
    for (old, new) in [
        ("0 9 * * 2-6", "0 9 * * 1-5"),
        ("0  9 * * 1,7", "0 9 * * 0,6"),
        // From Monday to Saturday every third day: Monday and Thursday.
        ("0 9 * * 2/3", "0 9 * * 1-6/3"),
        ("0 9 * * 1-7/2", "0 9 * * 0-6/2"),
        ("0 9 * * MON-5", "0 9 * * MON-4"),
        ("0 9 ? * 2", "0 9 ? * 1"),
        ("*/15 8-18 */2 jan-jun 3", "*/15 8-18 */2 jan-jun 2"),
    ] {
        assert_eq!(advice(old), Some(posix(new)), "{old}");
    }
}

#[test]
fn expressions_that_mean_the_same_need_no_note() {
    for expression in [
        "*/5 * * * *",
        "0 9 * * MON-FRI",
        "0 9 * * */2",
        "0 9 1 * *",
        "0 9 * * ?",
        "30 4 1,15 * *",
        // Four fields failed before and fail now, at run time.
        "0 9 * *",
    ] {
        assert_eq!(advice(expression), None, "{expression}");
    }
}

#[test]
fn both_day_fields_joined_fields_and_bad_weekdays_are_explained() {
    let both = advice("0 9 1 * 2").unwrap();
    assert_eq!(
        both,
        "it restricts both the day of the month (`1`) and the weekday (`2`): a schedule already stored keeps running on days that match both; one created after the upgrade runs on days that match either, so restrict one field and check the other when the message is handled"
    );
    let named = advice("0 9 1-7 * MON").unwrap();
    assert!(
        named.starts_with("it restricts both the day of the month (`1-7`) and the weekday (`MON`)"),
        "{named}"
    );
    assert!(!named.contains("POSIX numbers"), "{named}");
    // A mechanical form that outgrows the 128 bytes `new` accepts.
    let mut minutes = String::from("0");
    let mut next = 1;
    while minutes.len() + " 9 * * 2/2".len() < 127 {
        minutes.push_str(&format!(",{}", next % 60));
        next += 1;
    }
    let long = format!("{minutes} 9 * * 2/2");
    let note = advice(&long).unwrap();
    assert!(long.len() <= 128, "{long}");
    assert!(note.ends_with("is longer than the 128 bytes `RecurringDefinition::new` accepts; write a schedule not yet stored by hand"), "{note}");
    // `*/2` and `?` leave a day field unrestricted.
    assert_eq!(advice("0 9 */2 * MON"), None);

    let joined = advice("1, 2 * * *").unwrap();
    assert!(
        joined.starts_with(
            "`RecurringDefinition::new` now refuses `1, 2 * * *`: a field starts or ends with `,`, `-` or `/`, or runs a number into a name (`1jan`)"
        ) && joined.ends_with("cancel it and create a POSIX expression under a new name"),
        "{joined}"
    );
    // Joined with a neighbour (seconds steps through `/5`, `-5`, `,30`) or
    // split where a number and a name touch.
    for shifted in [
        "0 9 * * 1 -5",
        "/5 * * * *",
        "-5 * * * *",
        ",30 * * * *",
        "0 9 1jan * *",
        "0 9 * jan1 *",
        "0 9 * * 1- 5",
    ] {
        assert!(
            advice(shifted)
                .unwrap()
                .contains("so the `cron` crate joined or split fields"),
            "{shifted}"
        );
    }
    // Names next to separators are not split.
    assert_eq!(advice("0 9 * jan-jun/2 MON-FRI"), None);

    assert_eq!(
        advice("0 9 * * 8").unwrap(),
        "`0 9 * * 8` has a weekday number outside 1 to 7, which neither syntax accepts; write the POSIX expression by hand"
    );
    assert_eq!(
        advice("0 9 * * 6-2").unwrap(),
        "no mechanical POSIX form: `0 9 * * 5-1`, its weekday numbers moved down by one, is refused; write a schedule not yet stored by hand"
    );
    // Core's evaluator takes a step only on a range; the `cron` crate also
    // took `*/2` in a list.
    assert_eq!(
        advice("0 9 * * */2,MON").unwrap(),
        "`RecurringDefinition::new` now refuses `0 9 * * */2,MON`. A schedule already stored with it keeps running, but this call fails: stop creating it here, or cancel it and create a POSIX expression under a new name"
    );
    assert_eq!(advice("0 9 * * thurs"), None);
}

#[test]
fn each_new_call_in_a_messaging_file_is_reported_with_its_note() {
    let source = r#"
use rullst::messaging::schedules::{MissedRunPolicy, RecurringDefinition};
fn create(expression: &str) {
    let a = RecurringDefinition::new("weekly", "0 9 * * 2-6", 0, MissedRunPolicy::Skip, message());
    let b = RecurringDefinition::new("daily", "0 9 * * *", 0, MissedRunPolicy::Skip, message());
    let c = RecurringDefinition::new("custom", expression, 0, MissedRunPolicy::Skip, message());
    let d = rullst_messaging::schedules::RecurringDefinition::new("monthly", "0 9 1 * 2", 0, MissedRunPolicy::Skip, message());
    let e = RecurringDefinition::new("joined", concat!("0 9 * * ", "2"), 0, MissedRunPolicy::Skip, message());
}
"#;
    let found = notes(source);
    assert_eq!(
        found.iter().map(|(line, _)| *line).collect::<Vec<_>>(),
        [4, 6, 7, 8]
    );
    assert_eq!(
        found[0].1.as_deref(),
        Some(
            "POSIX form for a schedule not yet stored: `0 9 * * 1-5` (weekday numbers move down by one: Sunday is 0 or 7, Monday 1, Saturday 6)"
        )
    );
    assert_eq!(found[1].1, None);
    assert!(
        found[2]
            .1
            .as_deref()
            .unwrap()
            .starts_with("it restricts both")
    );
    assert_eq!(found[3].1, None);

    // An application's own `RecurringDefinition`, in a file that does not
    // name Rullst's Messaging.
    let own = "fn create() {\n    let a = RecurringDefinition::new(\"weekly\", \"0 9 * * 2-6\", 0, policy, message());\n}\n";
    assert!(notes(own).is_empty());
}
