// Closed passive-event validation for the generated Academy worker.

pub fn get_files() -> Vec<(&'static str, String)> {
    vec![(
        "src/services/automation_worker_event_service.rs",
        AUTOMATION_WORKER_EVENT_SERVICE.to_string(),
    )]
}

const AUTOMATION_WORKER_EVENT_SERVICE: &str = r##"use crate::services::outbox_service::ClaimedOutboxEvent;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EnrollmentActivatedV1 {
    schema_version: i32,
    actor_user_id: i32,
    subject_user_id: i32,
    course_id: i32,
    enrollment_id: i32,
    status: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CoursePublishedV1 {
    schema_version: i32,
    actor_user_id: i32,
    course_id: i32,
    course_version_id: i32,
    version_key: String,
    revision: i32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CourseRolledBackV1 {
    schema_version: i32,
    actor_user_id: i32,
    course_id: i32,
    source_version_id: i32,
    replaced_version_id: i32,
    result_version_id: i32,
    rollback_key: String,
    reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LessonCompletedV1 {
    schema_version: i32,
    actor_user_id: i32,
    subject_user_id: i32,
    lesson_id: i32,
    progress_event_key: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CourseCompletedV1 {
    schema_version: i32,
    actor_user_id: i32,
    subject_user_id: i32,
    course_id: i32,
    course_version_id: i32,
    completion_id: i32,
    ruleset_version: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CertificateRevokedV1 {
    schema_version: i32,
    actor_user_id: i32,
    subject_user_id: i32,
    course_id: i32,
    certificate_key: String,
    reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AssignmentSubmittedV1 {
    schema_version: i32,
    actor_user_id: i32,
    subject_user_id: i32,
    assignment_id: i32,
    submission_id: i32,
    submission_key: String,
    ruleset_version: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AssignmentGradedV1 {
    schema_version: i32,
    actor_user_id: i32,
    subject_user_id: i32,
    assignment_id: i32,
    submission_id: i32,
    grade_id: i32,
    grading_key: String,
    ruleset_version: String,
    points_awarded: i32,
    max_points: i32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AssignmentGradeCorrectedV1 {
    schema_version: i32,
    actor_user_id: i32,
    subject_user_id: i32,
    assignment_id: i32,
    assignment_grade_id: i32,
    correction_id: i32,
    correction_key: String,
    ruleset_version: String,
    previous_points: i32,
    corrected_points: i32,
    max_points: i32,
}

pub fn validate_passive_event(event: &ClaimedOutboxEvent) -> Result<Option<usize>, &'static str> {
    match event.event_kind.as_str() {
        "course_published" => validate_course_published(event),
        "course_rolled_back" => validate_course_rolled_back(event),
        "enrollment_activated" => validate_enrollment(event),
        "lesson_completed" => validate_lesson(event),
        "course_completed" => validate_completion(event),
        "certificate_revoked" => validate_certificate(event),
        "assignment_submitted" => validate_assignment_submission(event),
        "assignment_graded" => validate_assignment_grade(event),
        "assignment_grade_corrected" => validate_assignment_grade_correction(event),
        _ => return Ok(None),
    }?;
    Ok(Some(0))
}

fn validate_course_published(event: &ClaimedOutboxEvent) -> Result<(), &'static str> {
    let payload: CoursePublishedV1 = parse(&event.payload_json, "invalid publication event")?;
    if payload.schema_version != 1 || payload.actor_user_id <= 0 || payload.course_id <= 0
        || payload.course_version_id <= 0 || !valid_key(&payload.version_key, 96)
        || payload.revision <= 0 || event.subject_user_id != 0
    { return Err("invalid publication event"); }
    Ok(())
}

fn validate_course_rolled_back(event: &ClaimedOutboxEvent) -> Result<(), &'static str> {
    let payload: CourseRolledBackV1 = parse(&event.payload_json, "invalid rollback event")?;
    if payload.schema_version != 1 || payload.actor_user_id <= 0 || payload.course_id <= 0
        || payload.source_version_id <= 0 || payload.replaced_version_id <= 0
        || payload.result_version_id <= 0 || payload.source_version_id == payload.replaced_version_id
        || payload.result_version_id == payload.replaced_version_id
        || !valid_key(&payload.rollback_key, 64) || !valid_text(&payload.reason, 8, 512)
        || event.subject_user_id != 0
    { return Err("invalid rollback event"); }
    Ok(())
}

fn validate_enrollment(event: &ClaimedOutboxEvent) -> Result<(), &'static str> {
    let payload: EnrollmentActivatedV1 = parse(&event.payload_json, "invalid enrollment event")?;
    if payload.schema_version != 1 || payload.actor_user_id <= 0
        || payload.subject_user_id != event.subject_user_id
        || payload.actor_user_id != payload.subject_user_id || payload.course_id <= 0
        || payload.enrollment_id <= 0 || payload.status != "active"
    { return Err("invalid enrollment event"); }
    Ok(())
}

fn validate_lesson(event: &ClaimedOutboxEvent) -> Result<(), &'static str> {
    let payload: LessonCompletedV1 = parse(&event.payload_json, "invalid lesson event")?;
    if payload.schema_version != 1 || payload.actor_user_id <= 0
        || payload.subject_user_id != event.subject_user_id || payload.lesson_id <= 0
        || !valid_key(&payload.progress_event_key, 128)
    { return Err("invalid lesson event"); }
    Ok(())
}

fn validate_completion(event: &ClaimedOutboxEvent) -> Result<(), &'static str> {
    let payload: CourseCompletedV1 = parse(&event.payload_json, "invalid completion event")?;
    if payload.schema_version != 1 || payload.actor_user_id <= 0
        || payload.subject_user_id != event.subject_user_id || payload.course_id <= 0
        || payload.course_version_id <= 0 || payload.completion_id <= 0
        || !valid_key(&payload.ruleset_version, 96)
    { return Err("invalid completion event"); }
    Ok(())
}

fn validate_certificate(event: &ClaimedOutboxEvent) -> Result<(), &'static str> {
    let payload: CertificateRevokedV1 = parse(&event.payload_json, "invalid certificate event")?;
    if payload.schema_version != 1 || payload.actor_user_id <= 0
        || payload.subject_user_id != event.subject_user_id || payload.course_id <= 0
        || !valid_key(&payload.certificate_key, 64) || !valid_text(&payload.reason, 8, 256)
    { return Err("invalid certificate event"); }
    Ok(())
}

fn validate_assignment_submission(event: &ClaimedOutboxEvent) -> Result<(), &'static str> {
    let payload: AssignmentSubmittedV1 = parse(&event.payload_json, "invalid assignment submission event")?;
    if payload.schema_version != 1 || payload.actor_user_id <= 0
        || payload.actor_user_id != payload.subject_user_id
        || payload.subject_user_id != event.subject_user_id || payload.assignment_id <= 0
        || payload.submission_id <= 0 || !valid_key(&payload.submission_key, 96)
        || !valid_key(&payload.ruleset_version, 96)
    { return Err("invalid assignment submission event"); }
    Ok(())
}

fn validate_assignment_grade(event: &ClaimedOutboxEvent) -> Result<(), &'static str> {
    let payload: AssignmentGradedV1 = parse(&event.payload_json, "invalid assignment grade event")?;
    if payload.schema_version != 1 || payload.actor_user_id <= 0
        || payload.subject_user_id != event.subject_user_id
        || payload.actor_user_id == payload.subject_user_id || payload.assignment_id <= 0
        || payload.submission_id <= 0 || payload.grade_id <= 0
        || !valid_key(&payload.grading_key, 96) || !valid_key(&payload.ruleset_version, 96)
        || payload.points_awarded < 0 || payload.max_points <= 0
        || payload.points_awarded > payload.max_points
    { return Err("invalid assignment grade event"); }
    Ok(())
}

fn validate_assignment_grade_correction(event: &ClaimedOutboxEvent) -> Result<(), &'static str> {
    let payload: AssignmentGradeCorrectedV1 =
        parse(&event.payload_json, "invalid assignment grade correction event")?;
    if payload.schema_version != 1 || payload.actor_user_id <= 0
        || payload.subject_user_id != event.subject_user_id
        || payload.actor_user_id == payload.subject_user_id || payload.assignment_id <= 0
        || payload.assignment_grade_id <= 0 || payload.correction_id <= 0
        || !valid_key(&payload.correction_key, 96) || !valid_key(&payload.ruleset_version, 96)
        || payload.previous_points < 0 || payload.corrected_points < 0 || payload.max_points <= 0
        || payload.previous_points > payload.max_points || payload.corrected_points > payload.max_points
        || payload.previous_points == payload.corrected_points
    { return Err("invalid assignment grade correction event"); }
    Ok(())
}

fn parse<T: serde::de::DeserializeOwned>(payload: &str, error: &'static str) -> Result<T, &'static str> {
    serde_json::from_str(payload).map_err(|_| error)
}

fn valid_key(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.')
    })
}

fn valid_text(value: &str, minimum: usize, maximum: usize) -> bool {
    (minimum..=maximum).contains(&value.len()) && !value.chars().any(char::is_control)
}
"##;

#[cfg(test)]
mod tests {
    use super::AUTOMATION_WORKER_EVENT_SERVICE;

    #[test]
    fn passive_event_validation_is_closed_and_bounded() {
        assert!(AUTOMATION_WORKER_EVENT_SERVICE.contains("deny_unknown_fields"));
        assert!(AUTOMATION_WORKER_EVENT_SERVICE.contains("assignment_submitted"));
        assert!(AUTOMATION_WORKER_EVENT_SERVICE.contains("assignment_graded"));
        assert!(AUTOMATION_WORKER_EVENT_SERVICE.contains("assignment_grade_corrected"));
        assert!(AUTOMATION_WORKER_EVENT_SERVICE.contains("course_rolled_back"));
        assert!(AUTOMATION_WORKER_EVENT_SERVICE.contains("course_completed"));
        assert!(AUTOMATION_WORKER_EVENT_SERVICE.contains("certificate_revoked"));
        assert!(AUTOMATION_WORKER_EVENT_SERVICE.contains("Ok(None)"));
    }
}
