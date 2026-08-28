// Optional repository-pattern templates for the LMS starter.

use crate::blueprints::common;

const MODELS: [(&str, &str); 38] = [
    ("Category", "categories"),
    ("Course", "courses"),
    ("CourseModule", "course_modules"),
    ("CourseVersion", "course_versions"),
    ("PublicationRollback", "course_publication_rollbacks"),
    ("CourseCompletion", "course_completions"),
    ("Certificate", "certificates"),
    ("RoleAssignment", "role_assignments"),
    ("DomainEvent", "academy_outbox"),
    ("Lesson", "lessons"),
    ("User", "users"),
    ("Enrollment", "enrollments"),
    ("LessonProgress", "lesson_progress"),
    ("LessonProgressEvent", "lesson_progress_events"),
    ("LessonReleaseRule", "lesson_release_rules"),
    ("Notification", "notifications"),
    ("NotificationPreference", "notification_preferences"),
    ("SchedulerLease", "scheduler_leases"),
    ("Quiz", "quizzes"),
    ("QuizQuestion", "quiz_questions"),
    ("QuizOption", "quiz_options"),
    ("QuizAttempt", "quiz_attempts"),
    ("QuizAttemptSession", "quiz_attempt_sessions"),
    ("QuizAnswer", "quiz_answers"),
    ("Activity", "activities"),
    ("Assignment", "assignments"),
    ("RubricCriterion", "rubric_criteria"),
    ("AssignmentSubmission", "assignment_submissions"),
    ("AssignmentGrade", "assignment_grades"),
    ("AssignmentGradeCorrection", "assignment_grade_corrections"),
    ("RubricScore", "rubric_scores"),
    ("Achievement", "achievements"),
    ("LeaderboardEntry", "leaderboard_entries"),
    ("AutomationRule", "automation_rules"),
    ("AutomationExecution", "automation_executions"),
    ("UserAchievement", "user_achievements"),
    ("ScoreEvent", "score_events"),
    ("ScoreCorrection", "score_corrections"),
];

pub fn extend_manifest(manifest: &mut Vec<(&'static str, String)>, repository_pattern: bool) {
    if !repository_pattern {
        return;
    }

    for (model, table) in MODELS {
        let path = match model {
            "Category" => "src/repositories/category_repository.rs",
            "Course" => "src/repositories/course_repository.rs",
            "CourseModule" => "src/repositories/course_module_repository.rs",
            "CourseVersion" => "src/repositories/course_version_repository.rs",
            "PublicationRollback" => "src/repositories/publication_rollback_repository.rs",
            "CourseCompletion" => "src/repositories/course_completion_repository.rs",
            "Certificate" => "src/repositories/certificate_repository.rs",
            "RoleAssignment" => "src/repositories/role_assignment_repository.rs",
            "DomainEvent" => "src/repositories/domain_event_repository.rs",
            "Lesson" => "src/repositories/lesson_repository.rs",
            "User" => "src/repositories/user_repository.rs",
            "Enrollment" => "src/repositories/enrollment_repository.rs",
            "LessonProgress" => "src/repositories/lesson_progress_repository.rs",
            "LessonProgressEvent" => "src/repositories/lesson_progress_event_repository.rs",
            "LessonReleaseRule" => "src/repositories/lesson_release_rule_repository.rs",
            "Notification" => "src/repositories/notification_repository.rs",
            "NotificationPreference" => "src/repositories/notification_preference_repository.rs",
            "SchedulerLease" => "src/repositories/scheduler_lease_repository.rs",
            "Quiz" => "src/repositories/quiz_repository.rs",
            "QuizQuestion" => "src/repositories/quiz_question_repository.rs",
            "QuizOption" => "src/repositories/quiz_option_repository.rs",
            "QuizAttempt" => "src/repositories/quiz_attempt_repository.rs",
            "QuizAttemptSession" => "src/repositories/quiz_attempt_session_repository.rs",
            "QuizAnswer" => "src/repositories/quiz_answer_repository.rs",
            "Activity" => "src/repositories/activity_repository.rs",
            "Assignment" => "src/repositories/assignment_repository.rs",
            "RubricCriterion" => "src/repositories/rubric_criterion_repository.rs",
            "AssignmentSubmission" => "src/repositories/assignment_submission_repository.rs",
            "AssignmentGrade" => "src/repositories/assignment_grade_repository.rs",
            "AssignmentGradeCorrection" => {
                "src/repositories/assignment_grade_correction_repository.rs"
            }
            "RubricScore" => "src/repositories/rubric_score_repository.rs",
            "Achievement" => "src/repositories/achievement_repository.rs",
            "LeaderboardEntry" => "src/repositories/leaderboard_entry_repository.rs",
            "AutomationRule" => "src/repositories/automation_rule_repository.rs",
            "AutomationExecution" => "src/repositories/automation_execution_repository.rs",
            "UserAchievement" => "src/repositories/user_achievement_repository.rs",
            "ScoreEvent" => "src/repositories/score_event_repository.rs",
            "ScoreCorrection" => "src/repositories/score_correction_repository.rs",
            _ => continue,
        };
        manifest.push((path, common::generate_repository(model, table)));
    }

    manifest.push((
        "src/repositories/mod.rs",
        common::generate_repositories_mod(&MODELS.map(|(model, _)| model)),
    ));
}

#[cfg(test)]
mod tests {
    use super::extend_manifest;

    #[test]
    fn repository_mode_covers_every_lms_domain_model() {
        let mut manifest = Vec::new();
        extend_manifest(&mut manifest, true);

        assert_eq!(manifest.len(), 39);
        assert!(
            manifest
                .iter()
                .any(|(path, _)| *path == "src/repositories/automation_rule_repository.rs")
        );
    }
}
