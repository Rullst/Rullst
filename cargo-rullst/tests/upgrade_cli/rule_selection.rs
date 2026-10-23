//! A plan for an application already on a 13.0 prerelease applies only the
//! rules introduced after that prerelease, unless `--all-rules` is given.

use super::{Fixture, output_text};

/// Findings of an alpha.1 rule (`render_page`), an alpha.2 rule (a
/// `DatabaseError` match) and an alpha.3 rule (`Orm::transaction` in a file
/// that mentions `DatabaseError`).
const SOURCE: &str = "fn main() {}\nfn page(htmx: &Htmx) -> String {\n    render_page(htmx, \"Home\", String::new())\n}\nasync fn save() {\n    let _ = Orm::transaction(|tx| async move { Err(Error::DatabaseError(String::new())) }).await;\n}\n";

const ALPHA_1_RULE: &str = "V13-RENDER-PAGE-LANGUAGE";
const ALPHA_2_RULE: &str = "V13-ORM-CONSTRAINT-ERRORS";
const ALPHA_3_RULE: &str = "V13-ORM-TRANSACTION-ERRORS";

fn plan(fixture: &Fixture, extra: &[&str]) -> serde_json::Value {
    let mut arguments = vec!["upgrade", "--dry-run", "--json"];
    arguments.extend_from_slice(extra);
    let output = fixture.run(&arguments);
    assert!(output.status.success(), "{}", output_text(&output));
    serde_json::from_slice(&output.stdout).expect("stdout must contain only JSON")
}

fn codes(report: &serde_json::Value) -> Vec<String> {
    let mut codes = report["source_findings"]
        .as_array()
        .expect("findings")
        .iter()
        .map(|finding| finding["code"].as_str().expect("code").to_string())
        .collect::<Vec<_>>();
    codes.sort();
    codes
}

fn sorted(codes: &[&str]) -> Vec<String> {
    let mut codes = codes.iter().map(ToString::to_string).collect::<Vec<_>>();
    codes.sort();
    codes
}

#[test]
fn an_alpha_2_application_sees_only_the_alpha_3_rules() {
    let fixture = Fixture::new("alpha2", "=13.0.0-alpha.2", "13.0.0-alpha.2", SOURCE);
    let report = plan(&fixture, &[]);
    assert_eq!(codes(&report), sorted(&[ALPHA_3_RULE]));
    assert_eq!(
        report["source_findings"][0]["introduced_in"],
        "13.0.0-alpha.3"
    );
    assert_eq!(
        report["rule_selection"],
        serde_json::json!({
            "current_version": "13.0.0-alpha.2",
            "all_rules": false,
            "skipped_rules": 100,
            "hidden_findings": 2
        })
    );
    let human = output_text(&fixture.run(&["upgrade", "--dry-run"]));
    assert!(
        human.contains("Rules: introduced after 13.0.0-alpha.2, the Rullst version this project uses; earlier rules skipped: 100 (findings hidden: 2); --all-rules applies every rule"),
        "{human}"
    );
    assert!(!human.contains(ALPHA_1_RULE), "{human}");
}

#[test]
fn an_alpha_1_application_sees_the_alpha_2_and_alpha_3_rules() {
    let fixture = Fixture::new("alpha1", "13.0.0-alpha.1", "13.0.0-alpha.1", SOURCE);
    let report = plan(&fixture, &[]);
    assert_eq!(codes(&report), sorted(&[ALPHA_2_RULE, ALPHA_3_RULE]));
    assert_eq!(
        report["rule_selection"]["current_version"],
        "13.0.0-alpha.1"
    );
    assert_eq!(report["rule_selection"]["skipped_rules"], 95);
    assert_eq!(report["rule_selection"]["hidden_findings"], 1);
}

#[test]
fn a_v12_application_and_the_all_rules_flag_see_every_rule() {
    let every = sorted(&[ALPHA_1_RULE, ALPHA_2_RULE, ALPHA_3_RULE]);
    let v12 = Fixture::new("v12-all", "12", "12.0.0", SOURCE);
    let report = plan(&v12, &[]);
    assert_eq!(codes(&report), every);
    assert_eq!(report["rule_selection"]["skipped_rules"], 0);

    let alpha2 = Fixture::new("alpha2-all", "=13.0.0-alpha.2", "13.0.0-alpha.2", SOURCE);
    let report = plan(&alpha2, &["--all-rules"]);
    assert_eq!(codes(&report), every);
    assert_eq!(
        report["rule_selection"],
        serde_json::json!({
            "current_version": "13.0.0-alpha.2",
            "all_rules": true,
            "skipped_rules": 0,
            "hidden_findings": 0
        })
    );
    let human = output_text(&alpha2.run(&["upgrade", "--dry-run", "--all-rules"]));
    assert!(human.contains(ALPHA_1_RULE), "{human}");
    assert!(!human.contains("earlier rules skipped"), "{human}");
}
