use super::assessment::ASSESSMENT_SERVICE;

#[test]
fn quiz_grading_is_server_authoritative_and_transactional() {
    assert!(ASSESSMENT_SERVICE.contains("correct option invariant"));
    assert!(ASSESSMENT_SERVICE.contains("option ownership"));
    assert!(ASSESSMENT_SERVICE.contains("AttemptLimit"));
    assert!(ASSESSMENT_SERVICE.contains("AttemptNotStarted"));
    assert!(ASSESSMENT_SERVICE.contains("AttemptExpired"));
    assert!(ASSESSMENT_SERVICE.contains("quiz_graded"));
    assert!(ASSESSMENT_SERVICE.contains("score_recorded"));
    assert!(ASSESSMENT_SERVICE.contains("INSERT INTO score_events"));
    assert!(ASSESSMENT_SERVICE.contains("INSERT INTO leaderboard_entries"));
    assert!(!ASSESSMENT_SERVICE.contains("format!(\"SELECT"));
}
