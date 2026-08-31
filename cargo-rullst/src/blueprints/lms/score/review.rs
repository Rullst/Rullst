//! Deterministic server-authoritative spaced-review templates.

pub(super) fn get_files() -> Vec<(&'static str, String)> {
    vec![
        (
            "src/services/score_service/review.rs",
            REVIEW_SERVICE.to_string(),
        ),
        (
            "src/models/activity_review_policy.rs",
            REVIEW_POLICY_MODEL.to_string(),
        ),
        (
            "src/models/activity_review_state.rs",
            REVIEW_STATE_MODEL.to_string(),
        ),
        (
            "src/controllers/review_controller.rs",
            REVIEW_CONTROLLER.to_string(),
        ),
    ]
}

pub(super) const REVIEW_POLICY_MODEL: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "activity_review_policies")]
pub struct ActivityReviewPolicy {
    pub id: i32,
    pub activity_id: i32,
    pub algorithm_version: String,
    pub passing_ratio_milli: i32,
    pub first_interval_seconds: i64,
    pub lapse_interval_seconds: i64,
    pub maximum_interval_seconds: i64,
    pub enabled: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for ActivityReviewPolicy {
    fn nexus_table() -> &'static str { "activity_review_policies" }
    fn nexus_label() -> &'static str { "Activity Review Policies" }
    fn nexus_icon() -> &'static str { "🧠" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "activity_id", label: "Activity", kind: FieldKind::ForeignKey { table: "activities", label_col: "title" }, hidden: false, readonly: true },
            FieldMeta { name: "algorithm_version", label: "Algorithm", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "passing_ratio_milli", label: "Passing Ratio (‰)", kind: FieldKind::Number, hidden: false, readonly: false },
            FieldMeta { name: "first_interval_seconds", label: "First Interval", kind: FieldKind::Number, hidden: false, readonly: false },
            FieldMeta { name: "lapse_interval_seconds", label: "Lapse Interval", kind: FieldKind::Number, hidden: false, readonly: false },
            FieldMeta { name: "maximum_interval_seconds", label: "Maximum Interval", kind: FieldKind::Number, hidden: false, readonly: false },
            FieldMeta { name: "enabled", label: "Enabled", kind: FieldKind::Boolean, hidden: false, readonly: false },
            FieldMeta { name: "created_at", label: "Created At", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "updated_at", label: "Updated At", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
"##;

pub(super) const REVIEW_STATE_MODEL: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "activity_review_states")]
pub struct ActivityReviewState {
    pub id: i32,
    pub school_id: i32,
    pub subject_user_id: i32,
    pub course_id: i32,
    pub activity_id: i32,
    pub algorithm_version: String,
    pub repetitions: i32,
    pub lapses: i32,
    pub ease_milli: i32,
    pub interval_seconds: i64,
    pub due_at_epoch: i64,
    pub last_attempt_key: String,
    pub last_points: i32,
    pub last_max_score: i32,
    pub last_reviewed_at_epoch: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for ActivityReviewState {
    fn nexus_table() -> &'static str { "activity_review_states" }
    fn nexus_label() -> &'static str { "Activity Review States" }
    fn nexus_icon() -> &'static str { "🗓️" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "school_id", label: "School", kind: FieldKind::ForeignKey { table: "schools", label_col: "name" }, hidden: false, readonly: true },
            FieldMeta { name: "subject_user_id", label: "Learner", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "course_id", label: "Course", kind: FieldKind::ForeignKey { table: "courses", label_col: "title" }, hidden: false, readonly: true },
            FieldMeta { name: "activity_id", label: "Activity", kind: FieldKind::ForeignKey { table: "activities", label_col: "title" }, hidden: false, readonly: true },
            FieldMeta { name: "algorithm_version", label: "Algorithm", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "repetitions", label: "Repetitions", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "lapses", label: "Lapses", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "ease_milli", label: "Ease (‰)", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "interval_seconds", label: "Interval", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "due_at_epoch", label: "Due Epoch", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "last_attempt_key", label: "Last Attempt", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "last_points", label: "Last Points", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "last_max_score", label: "Last Maximum", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "last_reviewed_at_epoch", label: "Last Reviewed", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "created_at", label: "Created At", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "updated_at", label: "Updated At", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
"##;

pub(super) const REVIEW_SERVICE: &str = r##"use super::{ScoreError, ScoreSubmission};
use crate::services::school_service;
use rullst_security::{RbacGuard, UserContext};
use serde::Serialize;

const REVIEW_ALGORITHM_V1: &str = "rullst-box-v1";
const MIN_EASE_MILLI: i32 = 1_300;
const MAX_EASE_MILLI: i32 = 3_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReviewPolicy {
    passing_ratio_milli: i32,
    first_interval_seconds: i64,
    lapse_interval_seconds: i64,
    maximum_interval_seconds: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReviewSnapshot {
    repetitions: i32,
    lapses: i32,
    ease_milli: i32,
    interval_seconds: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReviewTransition {
    repetitions: i32,
    lapses: i32,
    ease_milli: i32,
    interval_seconds: i64,
    due_at_epoch: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DueReview {
    pub activity_id: i32,
    pub course_id: i32,
    pub title: String,
    pub due_at_epoch: i64,
    pub repetitions: i32,
    pub lapses: i32,
}

fn validate_policy(policy: ReviewPolicy) -> Result<(), ScoreError> {
    const MAX_INTERVAL: i64 = 5 * 366 * 24 * 60 * 60;
    if !(500..=1_000).contains(&policy.passing_ratio_milli)
        || !(3_600..=31 * 24 * 60 * 60).contains(&policy.first_interval_seconds)
        || !(60..=policy.first_interval_seconds).contains(&policy.lapse_interval_seconds)
        || !(policy.first_interval_seconds..=MAX_INTERVAL)
            .contains(&policy.maximum_interval_seconds)
    {
        return Err(ScoreError::InvalidField("review policy"));
    }
    Ok(())
}

fn transition(
    policy: ReviewPolicy,
    previous: Option<ReviewSnapshot>,
    points: i32,
    max_score: i32,
    reviewed_at_epoch: i64,
) -> Result<ReviewTransition, ScoreError> {
    validate_policy(policy)?;
    if points < 0 || max_score <= 0 || points > max_score || reviewed_at_epoch <= 0 {
        return Err(ScoreError::InvalidField("review result"));
    }
    let ratio = i64::from(points)
        .saturating_mul(1_000)
        .checked_div(i64::from(max_score))
        .ok_or(ScoreError::InvalidField("review result"))?;
    let previous = previous.unwrap_or(ReviewSnapshot {
        repetitions: 0,
        lapses: 0,
        ease_milli: 2_000,
        interval_seconds: policy.first_interval_seconds,
    });
    if previous.repetitions < 0
        || previous.lapses < 0
        || !(MIN_EASE_MILLI..=MAX_EASE_MILLI).contains(&previous.ease_milli)
        || previous.interval_seconds <= 0
    {
        return Err(ScoreError::InvalidField("review state"));
    }
    let passed = ratio >= i64::from(policy.passing_ratio_milli);
    let (repetitions, lapses, ease_milli, interval_seconds) = if passed {
        let ease = if ratio == 1_000 {
            previous.ease_milli.saturating_add(100).min(MAX_EASE_MILLI)
        } else {
            previous.ease_milli
        };
        let interval = if previous.repetitions == 0 {
            policy.first_interval_seconds
        } else {
            previous
                .interval_seconds
                .saturating_mul(i64::from(ease))
                .checked_div(1_000)
                .ok_or(ScoreError::InvalidField("review interval"))?
                .max(policy.first_interval_seconds)
                .min(policy.maximum_interval_seconds)
        };
        (
            previous.repetitions.saturating_add(1),
            previous.lapses,
            ease,
            interval,
        )
    } else {
        (
            0,
            previous.lapses.saturating_add(1),
            previous.ease_milli.saturating_sub(200).max(MIN_EASE_MILLI),
            policy.lapse_interval_seconds,
        )
    };
    let due_at_epoch = reviewed_at_epoch
        .checked_add(interval_seconds)
        .ok_or(ScoreError::InvalidField("review due time"))?;
    Ok(ReviewTransition {
        repetitions,
        lapses,
        ease_milli,
        interval_seconds,
        due_at_epoch,
    })
}

pub(super) async fn apply_review_schedule(
    transaction: &mut rullst::db::sqlx::Transaction<'_, rullst::db::sqlx::Any>,
    driver: &str,
    school_id: i32,
    value: &ScoreSubmission,
) -> Result<(), ScoreError> {
    let policy_sql = match driver {
        "postgres" => "SELECT algorithm_version, passing_ratio_milli, first_interval_seconds, lapse_interval_seconds, maximum_interval_seconds, enabled FROM activity_review_policies WHERE activity_id = $1 FOR UPDATE",
        "mysql" => "SELECT algorithm_version, passing_ratio_milli, first_interval_seconds, lapse_interval_seconds, maximum_interval_seconds, enabled FROM activity_review_policies WHERE activity_id = ? FOR UPDATE",
        _ => "SELECT algorithm_version, passing_ratio_milli, first_interval_seconds, lapse_interval_seconds, maximum_interval_seconds, enabled FROM activity_review_policies WHERE activity_id = ?",
    };
    let row = rullst::db::sqlx::query_as::<_, (String, i32, i64, i64, i64, i32)>(policy_sql)
        .bind(value.activity_id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(|error| ScoreError::Database(error.into()))?;
    let Some(row) = row else { return Ok(()); };
    if row.5 == 0 { return Ok(()); }
    if row.0 != REVIEW_ALGORITHM_V1 || row.5 != 1 {
        return Err(ScoreError::InvalidField("review policy"));
    }
    let policy = ReviewPolicy {
        passing_ratio_milli: row.1,
        first_interval_seconds: row.2,
        lapse_interval_seconds: row.3,
        maximum_interval_seconds: row.4,
    };
    validate_policy(policy)?;
    let state_sql = match driver {
        "postgres" => "SELECT repetitions, lapses, ease_milli, interval_seconds, algorithm_version FROM activity_review_states WHERE subject_user_id = $1 AND activity_id = $2 FOR UPDATE",
        "mysql" => "SELECT repetitions, lapses, ease_milli, interval_seconds, algorithm_version FROM activity_review_states WHERE subject_user_id = ? AND activity_id = ? FOR UPDATE",
        _ => "SELECT repetitions, lapses, ease_milli, interval_seconds, algorithm_version FROM activity_review_states WHERE subject_user_id = ? AND activity_id = ?",
    };
    let state = rullst::db::sqlx::query_as::<_, (i32, i32, i32, i64, String)>(state_sql)
        .bind(value.subject_user_id)
        .bind(value.activity_id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(|error| ScoreError::Database(error.into()))?;
    if state.as_ref().is_some_and(|state| state.4 != REVIEW_ALGORITHM_V1) {
        return Err(ScoreError::InvalidField("review algorithm migration"));
    }
    let previous = state.map(|state| ReviewSnapshot {
        repetitions: state.0,
        lapses: state.1,
        ease_milli: state.2,
        interval_seconds: state.3,
    });
    let next = transition(
        policy,
        previous,
        value.points,
        value.max_score,
        value.finished_at_epoch,
    )?;
    let upsert_sql = match driver {
        "postgres" => "INSERT INTO activity_review_states (school_id, subject_user_id, course_id, activity_id, algorithm_version, repetitions, lapses, ease_milli, interval_seconds, due_at_epoch, last_attempt_key, last_points, last_max_score, last_reviewed_at_epoch, created_at, updated_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP) ON CONFLICT (subject_user_id, activity_id) DO UPDATE SET school_id = EXCLUDED.school_id, course_id = EXCLUDED.course_id, algorithm_version = EXCLUDED.algorithm_version, repetitions = EXCLUDED.repetitions, lapses = EXCLUDED.lapses, ease_milli = EXCLUDED.ease_milli, interval_seconds = EXCLUDED.interval_seconds, due_at_epoch = EXCLUDED.due_at_epoch, last_attempt_key = EXCLUDED.last_attempt_key, last_points = EXCLUDED.last_points, last_max_score = EXCLUDED.last_max_score, last_reviewed_at_epoch = EXCLUDED.last_reviewed_at_epoch, updated_at = CURRENT_TIMESTAMP",
        "mysql" => "INSERT INTO activity_review_states (school_id, subject_user_id, course_id, activity_id, algorithm_version, repetitions, lapses, ease_milli, interval_seconds, due_at_epoch, last_attempt_key, last_points, last_max_score, last_reviewed_at_epoch, created_at, updated_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP) ON DUPLICATE KEY UPDATE school_id = VALUES(school_id), course_id = VALUES(course_id), algorithm_version = VALUES(algorithm_version), repetitions = VALUES(repetitions), lapses = VALUES(lapses), ease_milli = VALUES(ease_milli), interval_seconds = VALUES(interval_seconds), due_at_epoch = VALUES(due_at_epoch), last_attempt_key = VALUES(last_attempt_key), last_points = VALUES(last_points), last_max_score = VALUES(last_max_score), last_reviewed_at_epoch = VALUES(last_reviewed_at_epoch), updated_at = CURRENT_TIMESTAMP",
        _ => "INSERT INTO activity_review_states (school_id, subject_user_id, course_id, activity_id, algorithm_version, repetitions, lapses, ease_milli, interval_seconds, due_at_epoch, last_attempt_key, last_points, last_max_score, last_reviewed_at_epoch, created_at, updated_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP) ON CONFLICT (subject_user_id, activity_id) DO UPDATE SET school_id = excluded.school_id, course_id = excluded.course_id, algorithm_version = excluded.algorithm_version, repetitions = excluded.repetitions, lapses = excluded.lapses, ease_milli = excluded.ease_milli, interval_seconds = excluded.interval_seconds, due_at_epoch = excluded.due_at_epoch, last_attempt_key = excluded.last_attempt_key, last_points = excluded.last_points, last_max_score = excluded.last_max_score, last_reviewed_at_epoch = excluded.last_reviewed_at_epoch, updated_at = CURRENT_TIMESTAMP",
    };
    rullst::db::sqlx::query(upsert_sql)
        .bind(school_id).bind(value.subject_user_id).bind(value.course_id)
        .bind(value.activity_id).bind(REVIEW_ALGORITHM_V1)
        .bind(next.repetitions).bind(next.lapses).bind(next.ease_milli)
        .bind(next.interval_seconds).bind(next.due_at_epoch)
        .bind(&value.attempt_key).bind(value.points).bind(value.max_score)
        .bind(value.finished_at_epoch)
        .execute(&mut **transaction).await
        .map_err(|error| ScoreError::Database(error.into()))?;
    Ok(())
}

pub async fn due_reviews_at(
    context: &UserContext,
    subject_user_id: i32,
    observed_at_epoch: i64,
    limit: u32,
) -> Result<Vec<DueReview>, ScoreError> {
    RbacGuard::authorize_owner_or_role(context, &subject_user_id.to_string(), "admin")
        .map_err(|_| ScoreError::Forbidden)?;
    if subject_user_id <= 0 || observed_at_epoch <= 0 || !(1..=50).contains(&limit) {
        return Err(ScoreError::InvalidField("due review query"));
    }
    let school_id = school_service::authorize_school_membership_at(
        context, subject_user_id, observed_at_epoch,
    ).await.map_err(|error| match error {
        school_service::SchoolError::Database(error) => ScoreError::Database(error),
        _ => ScoreError::Forbidden,
    })?;
    let driver = rullst::db::Orm::driver()?;
    let sql = match driver {
        "postgres" => "SELECT rs.activity_id, rs.course_id, a.title, rs.due_at_epoch, rs.repetitions, rs.lapses FROM activity_review_states rs INNER JOIN activities a ON a.id = rs.activity_id INNER JOIN lessons l ON l.id = a.lesson_id AND l.course_id = rs.course_id INNER JOIN course_school_scopes css ON css.course_id = rs.course_id AND css.school_id = rs.school_id INNER JOIN enrollments e ON e.user_id = rs.subject_user_id AND e.course_id = rs.course_id WHERE rs.school_id = $1 AND rs.subject_user_id = $2 AND rs.due_at_epoch <= $3 AND e.status = $4 ORDER BY rs.due_at_epoch ASC, rs.activity_id ASC LIMIT $5",
        _ => "SELECT rs.activity_id, rs.course_id, a.title, rs.due_at_epoch, rs.repetitions, rs.lapses FROM activity_review_states rs INNER JOIN activities a ON a.id = rs.activity_id INNER JOIN lessons l ON l.id = a.lesson_id AND l.course_id = rs.course_id INNER JOIN course_school_scopes css ON css.course_id = rs.course_id AND css.school_id = rs.school_id INNER JOIN enrollments e ON e.user_id = rs.subject_user_id AND e.course_id = rs.course_id WHERE rs.school_id = ? AND rs.subject_user_id = ? AND rs.due_at_epoch <= ? AND e.status = ? ORDER BY rs.due_at_epoch ASC, rs.activity_id ASC LIMIT ?",
    };
    let rows = rullst::db::sqlx::query_as::<_, (i32, i32, String, i64, i32, i32)>(sql)
        .bind(school_id).bind(subject_user_id).bind(observed_at_epoch).bind("active")
        .bind(i64::from(limit)).fetch_all(rullst::db::Orm::pool()?).await
        .map_err(|error| ScoreError::Database(error.into()))?;
    let mut reviews = Vec::with_capacity(rows.len());
    for row in rows {
        let authorized = school_service::authorize_course_enrollment_at(
            context, subject_user_id, row.1, observed_at_epoch,
        ).await;
        match authorized {
            Ok(authorized_school) if authorized_school == school_id => reviews.push(DueReview {
                activity_id: row.0,
                course_id: row.1,
                title: row.2,
                due_at_epoch: row.3,
                repetitions: row.4,
                lapses: row.5,
            }),
            Ok(_) | Err(school_service::SchoolError::Forbidden) => {}
            Err(school_service::SchoolError::Database(error)) => return Err(ScoreError::Database(error)),
            Err(_) => return Err(ScoreError::Forbidden),
        }
    }
    Ok(reviews)
}

pub async fn due_reviews(
    context: &UserContext,
    subject_user_id: i32,
    limit: u32,
) -> Result<Vec<DueReview>, ScoreError> {
    due_reviews_at(context, subject_user_id, super::unix_now()?, limit).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> ReviewPolicy {
        ReviewPolicy {
            passing_ratio_milli: 800,
            first_interval_seconds: 86_400,
            lapse_interval_seconds: 600,
            maximum_interval_seconds: 15_552_000,
        }
    }

    #[test]
    fn review_transition_is_bounded_deterministic_and_lapse_aware() {
        let first = transition(policy(), None, 80, 80, 1_000).expect("first review");
        assert_eq!((first.repetitions, first.lapses), (1, 0));
        assert_eq!(first.interval_seconds, 86_400);
        let second = transition(
            policy(),
            Some(ReviewSnapshot {
                repetitions: first.repetitions,
                lapses: first.lapses,
                ease_milli: first.ease_milli,
                interval_seconds: first.interval_seconds,
            }),
            80, 80, 100_000,
        ).expect("second review");
        assert!(second.interval_seconds > first.interval_seconds);
        let lapse = transition(
            policy(),
            Some(ReviewSnapshot {
                repetitions: second.repetitions,
                lapses: second.lapses,
                ease_milli: second.ease_milli,
                interval_seconds: second.interval_seconds,
            }),
            0, 80, 200_000,
        ).expect("lapse review");
        assert_eq!((lapse.repetitions, lapse.lapses), (0, 1));
        assert_eq!(lapse.interval_seconds, 600);
        assert_eq!(lapse.due_at_epoch, 200_600);
    }
}
"##;

pub(super) const REVIEW_CONTROLLER: &str = r##"use crate::services::score_service::{ScoreError, due_reviews};
use rullst::server::{Extension, IntoResponse, Json, Query, Response, StatusCode};
use rullst_security::UserContext;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DueReviewQuery {
    pub limit: Option<u32>,
}

pub async fn index(
    Extension(user_id): Extension<i32>,
    Extension(context): Extension<UserContext>,
    Query(query): Query<DueReviewQuery>,
) -> Response {
    match due_reviews(&context, user_id, query.limit.unwrap_or(20)).await {
        Ok(reviews) => Json(reviews).into_response(),
        Err(ScoreError::Database(error)) => {
            eprintln!("Due review query failed: {error}");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
        Err(ScoreError::Cache(error)) => {
            eprintln!("Due review cache failed: {error}");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
        Err(ScoreError::Forbidden | ScoreError::InvalidIdentity) => {
            StatusCode::FORBIDDEN.into_response()
        }
        Err(ScoreError::InvalidField(_)) => StatusCode::UNPROCESSABLE_ENTITY.into_response(),
        Err(ScoreError::UnsupportedSchemaVersion(_)) => StatusCode::CONFLICT.into_response(),
    }
}
"##;

#[cfg(test)]
mod tests {
    use super::{REVIEW_CONTROLLER, REVIEW_SERVICE};

    #[test]
    fn review_templates_are_server_timed_owner_bound_and_transactional() {
        assert!(REVIEW_SERVICE.contains("FOR UPDATE"));
        assert!(REVIEW_SERVICE.contains("apply_review_schedule"));
        assert!(REVIEW_SERVICE.contains("authorize_course_enrollment_at"));
        assert!(REVIEW_CONTROLLER.contains("Extension(user_id)"));
        assert!(!REVIEW_CONTROLLER.contains("subject_user_id"));
    }
}
