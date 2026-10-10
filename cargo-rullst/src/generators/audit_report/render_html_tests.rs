use super::*;
use crate::generators::audit_compliance::EvidenceStatus;
use crate::generators::audit_report::catalog::{FORM_LABELS, IDOR, IMG_ALT, PERSONAL_DATA};
use crate::generators::audit_report::model::{Finding, PersonalData, PersonalField};

fn report(checks: Vec<Check>, fields: Vec<PersonalField>) -> Report {
    Report {
        generated_at: "2026-10-09T00:00:00Z".to_string(),
        checks,
        personal_data: PersonalData {
            status: EvidenceStatus::Observed(fields.len()),
            fields,
            detail: "inventory".to_string(),
        },
    }
}

#[test]
fn the_status_colour_follows_the_evidence() {
    let class = |status| status_class(&Check::with_status(&IDOR, status, Vec::new(), "d"));
    assert_eq!(class(EvidenceStatus::NoFindings), "ok");
    assert_eq!(class(EvidenceStatus::Findings(2)), "bad");
    assert_eq!(class(EvidenceStatus::Error("boom".to_string())), "bad");
    assert_eq!(class(EvidenceStatus::NotChecked("no sources")), "warn");
    assert_eq!(class(EvidenceStatus::Observed(1)), "warn");
}

#[test]
fn the_mapping_lists_asvs_chapter_and_wcag_references() {
    let check = |spec| Check::with_status(spec, EvidenceStatus::NoFindings, Vec::new(), "d");
    assert_eq!(
        mapping(&check(&IDOR)),
        "V8.2.1 (L1, V8.2 General Authorization Design)<br>V8.2.2 (L1, V8.2 General Authorization Design)"
    );
    assert_eq!(
        mapping(&check(&PERSONAL_DATA)),
        "V14.1.1 (L2, V14.1 Data Protection Documentation)<br>chapter V14 Data Protection"
    );
    assert_eq!(
        mapping(&check(&FORM_LABELS)),
        "WCAG 1.3.1 Info and Relationships (A)<br>WCAG 3.3.2 Labels or Instructions (A)<br>WCAG 4.1.2 Name, Role, Value (A)"
    );
}

#[test]
fn a_section_lists_only_the_checks_of_its_group() {
    let report = report(
        vec![
            Check::from_findings(
                &IDOR,
                vec![Finding::new("src/routes.rs", Some(3), "unclassified")],
                "d",
            ),
            Check::from_findings(
                &IMG_ALT,
                vec![Finding::new("page.html", Some(1), "no alt")],
                "d",
            ),
        ],
        Vec::new(),
    );
    let mut security = String::new();
    checks_section(&mut security, &report, Group::Security);
    assert!(security.contains(IDOR.title));
    assert!(security.contains("src/routes.rs:3"));
    assert!(!security.contains(IMG_ALT.title));
    assert!(!security.contains("page.html"));
}

#[test]
fn the_personal_data_table_appears_only_with_fields() {
    let empty = render(&report(Vec::new(), Vec::new()));
    assert!(!empty.contains("<th>Classification</th>"));
    let field = PersonalField {
        model: "User".to_string(),
        field: "email".to_string(),
        file: "src/models/user.rs".to_string(),
        line: 7,
        classification: "personal".to_string(),
        encrypted: false,
    };
    let listed = render(&report(Vec::new(), vec![field]));
    assert!(listed.contains("<th>Classification</th>"));
    assert!(listed.contains("<code>src/models/user.rs:7</code>"));
}
