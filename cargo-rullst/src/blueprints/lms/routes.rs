// Application entrypoints and route boundaries for the LMS blueprint.

use crate::blueprints::common;

pub fn get_routes(
    project_name_safe: &str,
    hot_reload: bool,
    orm_pattern: &str,
) -> Vec<(&'static str, String)> {
    let mut manifest = Vec::new();
    let repo_decl = common::repo_mod_decl(orm_pattern);

    if hot_reload {
        let lib_rs = format!(
            r##"use rullst::{{routes, Router}};

pub mod migrations;
pub mod models;
{repo_decl}pub mod controllers;
pub mod middlewares;
pub mod pages;
pub mod services;

pub fn router() -> Result<Router, Box<dyn std::error::Error>> {{
    let nexus_auth = rullst::nexus::NexusAuthPolicy::local_development_or_basic_from_env()?;
    let nexus = rullst::nexus::Nexus::new()
        .with_auth_policy(nexus_auth)
        .with_brand("LMS Admin")
        .register::<models::category::Category>()
        .register::<models::course::Course>()
        .register::<models::course_module::CourseModule>()
        .register::<models::course_version::CourseVersion>()
        .register::<models::publication_rollback::PublicationRollback>()
        .register::<models::course_completion::CourseCompletion>()
        .register::<models::certificate::Certificate>()
        .register::<models::role_assignment::RoleAssignment>()
        .register::<models::domain_event::DomainEvent>()
        .register::<models::lesson::Lesson>()
        .register::<models::user::User>()
        .register::<models::enrollment::Enrollment>()
        .register::<models::lesson_progress::LessonProgress>()
        .register::<models::lesson_progress_event::LessonProgressEvent>()
        .register::<models::lesson_release_rule::LessonReleaseRule>()
        .register::<models::notification::Notification>()
        .register::<models::notification_preference::NotificationPreference>()
        .register::<models::scheduler_lease::SchedulerLease>()
        .register::<models::quiz::Quiz>()
        .register::<models::quiz_question::QuizQuestion>()
        .register::<models::quiz_option::QuizOption>()
        .register::<models::quiz_attempt::QuizAttempt>()
        .register::<models::quiz_attempt_session::QuizAttemptSession>()
        .register::<models::quiz_answer::QuizAnswer>()
        .register::<models::activity::Activity>()
        .register::<models::activity_attempt::ActivityAttempt>()
        .register::<models::activity_review_policy::ActivityReviewPolicy>()
        .register::<models::activity_review_state::ActivityReviewState>()
        .register::<models::assignment::Assignment>()
        .register::<models::rubric_criterion::RubricCriterion>()
        .register::<models::assignment_submission::AssignmentSubmission>()
        .register::<models::assignment_grade::AssignmentGrade>()
        .register::<models::assignment_grade_correction::AssignmentGradeCorrection>()
        .register::<models::rubric_score::RubricScore>()
        .register::<models::achievement::Achievement>()
        .register::<models::leaderboard_entry::LeaderboardEntry>()
        .register::<models::automation_rule::AutomationRule>()
        .register::<models::automation_execution::AutomationExecution>()
        .register::<models::user_achievement::UserAchievement>()
        .register::<models::score_event::ScoreEvent>()
        .register::<models::score_correction::ScoreCorrection>()
        .try_build()?;

    let public = routes![
        get("/" => controllers::lms_controller::index),
        // rullst-access: public — course metadata and lesson titles form the public catalog.
        get("/courses/{{id}}" => controllers::lms_controller::show_course),
        // rullst-access: public — an opaque certificate key reveals bounded course evidence, never learner PII.
        get("/certificates/{{certificate_key}}" => controllers::completion_controller::verify),
        get("/login" => controllers::auth_controller::login_view),
        post("/login" => controllers::auth_controller::login_submit),
        get("/register" => controllers::auth_controller::register_view),
        post("/register" => controllers::auth_controller::register_submit),
        post("/logout" => controllers::auth_controller::logout),
    ];
    let learning = routes![
        get("/dashboard" => controllers::auth_controller::dashboard),
        // rullst-access: owner — the authenticated identity, never form data, owns the enrollment.
        post("/courses/{{id}}/enroll" => controllers::learning_controller::enroll),
        // rullst-access: owner — the handler requires an active enrollment for the lesson course.
        get("/lessons/{{id}}/play" => controllers::learning_controller::play_lesson),
        // rullst-access: owner — progress is written only for the authenticated enrollment owner.
        post("/lessons/{{id}}/progress" => controllers::learning_controller::record_progress),
        // rullst-access: owner — completion is derived for the authenticated learner from pinned server state.
        post("/courses/{{id}}/completion" => controllers::completion_controller::complete),
        // rullst-access: owner — the session identity scopes notification listing.
        get("/notifications" => controllers::notification_controller::index),
        // rullst-access: owner — quiz and subject identities come from the path/session.
        post("/quizzes/{{id}}/start" => controllers::assessment_controller::start),
        // rullst-access: owner — answers are graded against the server-side answer key.
        post("/quizzes/{{id}}/submit" => controllers::assessment_controller::submit),
        // rullst-access: owner — only an option and idempotency key cross this boundary; policy and score are server-derived.
        post("/activities/{{id}}/attempts" => controllers::activity_controller::submit),
        // rullst-access: owner — pair IDs are bounded; matching rules and score remain server-derived.
        post("/activities/{{id}}/attempts/matching" => controllers::activity_matching_controller::submit),
        // rullst-access: owner — typed input is bounded/digested; accepted answers and score remain server-derived.
        post("/activities/{{id}}/attempts/typed" => controllers::activity_typed_controller::submit),
        // rullst-access: owner — subject and clock are server-derived; rows remain school/enrollment scoped.
        get("/reviews/due" => controllers::review_controller::index),
        // rullst-access: owner — assignment and learner are derived from path/session before entitlement checks.
        post("/assignments/{{id}}/submissions" => controllers::assignment_controller::submit),
        // rullst-access: role — a persisted evaluator/instructor/admin scores only server rubric criteria.
        post("/submissions/{{id}}/grade" => controllers::assignment_controller::grade),
        // rullst-access: role — only admin can append a reasoned grade correction bounded by the same rubric.
        post("/assignment-grades/{{id}}/correct" => controllers::assignment_controller::correct_grade),
        // rullst-access: owner — the session identity scopes the notification mutation.
        post("/notifications/{{id}}/read" => controllers::notification_controller::read),
        // rullst-access: owner — no subject identity is accepted from the form.
        post("/notifications/preferences" => controllers::notification_controller::update_preference),
        // rullst-access: role — publication service requires instructor/admin from authenticated context.
        post("/courses/{{id}}/versions" => controllers::publication_controller::draft),
        // rullst-access: role — only the authenticated version author or admin can submit review.
        post("/course-versions/{{id}}/submit" => controllers::publication_controller::submit),
        // rullst-access: role — publication service requires a distinct authenticated admin reviewer.
        post("/course-versions/{{id}}/review" => controllers::publication_controller::review),
        // rullst-access: role — rollback creates a new immutable version and requires an authenticated admin.
        post("/courses/{{id}}/rollback" => controllers::publication_rollback_controller::rollback),
        // rullst-access: role — target identity is path-bound and the service enforces the grant hierarchy.
        post("/users/{{id}}/roles" => controllers::role_controller::grant),
        // rullst-access: role — the authenticated grant hierarchy controls durable revocation.
        post("/role-assignments/{{assignment_key}}/revoke" => controllers::role_controller::revoke),
        // rullst-access: role — the service requires admin and records actor, reason and server time.
        post("/certificates/{{certificate_key}}/revoke" => controllers::completion_controller::revoke),
    ].layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware));

    Ok(public
        .merge_axum(learning.into_axum())
        .layer(rullst::server::from_fn(rullst::security::csrf_middleware))
        .layer(rullst::server::from_fn(rullst::security::headers_middleware))
        .nest_axum("/nexus", nexus))
}}

#[unsafe(no_mangle)]
pub extern "C" fn rullst_router_init() -> *mut Router {{
    let router = match router() {{
        Ok(router) => router,
        Err(error) => {{
            eprintln!("LMS startup configuration error: {{error}}");
            Router::new()
        }}
    }};
    Box::into_raw(Box::new(router))
}}
"##,
            repo_decl = repo_decl
        );
        manifest.push(("src/lib.rs", lib_rs));

        let main_rs = format!(
            r##"pub mod migrations;
pub mod models;
{repo_decl}pub mod controllers;
pub mod middlewares;
pub mod pages;
pub mod services;

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    rullst::artisan!(crate::migrations::get_migrations());

    #[cfg(debug_assertions)]
    {{
        rullst::runtime::spawn(async {{
            if let Err(error) = rullst::studio::run_studio(5555).await {{
                eprintln!("Rullst Studio could not start: {{error}}");
            }}
        }});
        println!("📊 Rullst Studio running on http://127.0.0.1:5555");
    }}
    println!("🚀 LMS server starting on port 3000...");
    let is_hot = std::env::var("HOT_RELOAD").is_ok();

    let server = if is_hot {{
        let lib_path = if cfg!(target_os = "windows") {{
            format!("target/debug/{{}}", "{project_name_safe}")
        }} else {{
            format!("target/debug/lib{{}}", "{project_name_safe}")
        }};
        rullst::Server::new_hot(&lib_path)
    }} else {{
        let router = {project_name_safe}::router()?;
        rullst::Server::new(router)
    }};

    server.run(3000).await?;
    Ok(())
}}
"##,
            repo_decl = repo_decl,
            project_name_safe = project_name_safe
        );
        manifest.push(("src/main.rs", main_rs));
    } else {
        let main_rs = format!(
            r##"use rullst::{{routes, Server}};

pub mod migrations;
pub mod models;
{repo_decl}pub mod controllers;
pub mod middlewares;
pub mod pages;
pub mod services;

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    rullst::artisan!(crate::migrations::get_migrations());

    let nexus_auth = rullst::nexus::NexusAuthPolicy::local_development_or_basic_from_env()?;
    let nexus = rullst::nexus::Nexus::new()
        .with_auth_policy(nexus_auth)
        .with_brand("LMS Admin")
        .register::<models::category::Category>()
        .register::<models::course::Course>()
        .register::<models::course_module::CourseModule>()
        .register::<models::course_version::CourseVersion>()
        .register::<models::publication_rollback::PublicationRollback>()
        .register::<models::course_completion::CourseCompletion>()
        .register::<models::certificate::Certificate>()
        .register::<models::role_assignment::RoleAssignment>()
        .register::<models::domain_event::DomainEvent>()
        .register::<models::lesson::Lesson>()
        .register::<models::user::User>()
        .register::<models::enrollment::Enrollment>()
        .register::<models::lesson_progress::LessonProgress>()
        .register::<models::lesson_progress_event::LessonProgressEvent>()
        .register::<models::lesson_release_rule::LessonReleaseRule>()
        .register::<models::notification::Notification>()
        .register::<models::notification_preference::NotificationPreference>()
        .register::<models::scheduler_lease::SchedulerLease>()
        .register::<models::quiz::Quiz>()
        .register::<models::quiz_question::QuizQuestion>()
        .register::<models::quiz_option::QuizOption>()
        .register::<models::quiz_attempt::QuizAttempt>()
        .register::<models::quiz_attempt_session::QuizAttemptSession>()
        .register::<models::quiz_answer::QuizAnswer>()
        .register::<models::activity::Activity>()
        .register::<models::activity_attempt::ActivityAttempt>()
        .register::<models::activity_review_policy::ActivityReviewPolicy>()
        .register::<models::activity_review_state::ActivityReviewState>()
        .register::<models::assignment::Assignment>()
        .register::<models::rubric_criterion::RubricCriterion>()
        .register::<models::assignment_submission::AssignmentSubmission>()
        .register::<models::assignment_grade::AssignmentGrade>()
        .register::<models::assignment_grade_correction::AssignmentGradeCorrection>()
        .register::<models::rubric_score::RubricScore>()
        .register::<models::achievement::Achievement>()
        .register::<models::leaderboard_entry::LeaderboardEntry>()
        .register::<models::automation_rule::AutomationRule>()
        .register::<models::automation_execution::AutomationExecution>()
        .register::<models::user_achievement::UserAchievement>()
        .register::<models::score_event::ScoreEvent>()
        .register::<models::score_correction::ScoreCorrection>()
        .try_build()?;

    let public = routes![
        get("/" => controllers::lms_controller::index),
        // rullst-access: public — course metadata and lesson titles form the public catalog.
        get("/courses/{{id}}" => controllers::lms_controller::show_course),
        // rullst-access: public — an opaque certificate key reveals bounded course evidence, never learner PII.
        get("/certificates/{{certificate_key}}" => controllers::completion_controller::verify),
        get("/login" => controllers::auth_controller::login_view),
        post("/login" => controllers::auth_controller::login_submit),
        get("/register" => controllers::auth_controller::register_view),
        post("/register" => controllers::auth_controller::register_submit),
        post("/logout" => controllers::auth_controller::logout),
    ];
    let learning = routes![
        get("/dashboard" => controllers::auth_controller::dashboard),
        // rullst-access: owner — the authenticated identity, never form data, owns the enrollment.
        post("/courses/{{id}}/enroll" => controllers::learning_controller::enroll),
        // rullst-access: owner — the handler requires an active enrollment for the lesson course.
        get("/lessons/{{id}}/play" => controllers::learning_controller::play_lesson),
        // rullst-access: owner — progress is written only for the authenticated enrollment owner.
        post("/lessons/{{id}}/progress" => controllers::learning_controller::record_progress),
        // rullst-access: owner — completion is derived for the authenticated learner from pinned server state.
        post("/courses/{{id}}/completion" => controllers::completion_controller::complete),
        // rullst-access: owner — the session identity scopes notification listing.
        get("/notifications" => controllers::notification_controller::index),
        // rullst-access: owner — quiz and subject identities come from the path/session.
        post("/quizzes/{{id}}/start" => controllers::assessment_controller::start),
        // rullst-access: owner — answers are graded against the server-side answer key.
        post("/quizzes/{{id}}/submit" => controllers::assessment_controller::submit),
        // rullst-access: owner — only an option and idempotency key cross this boundary; policy and score are server-derived.
        post("/activities/{{id}}/attempts" => controllers::activity_controller::submit),
        // rullst-access: owner — pair IDs are bounded; matching rules and score remain server-derived.
        post("/activities/{{id}}/attempts/matching" => controllers::activity_matching_controller::submit),
        // rullst-access: owner — typed input is bounded/digested; accepted answers and score remain server-derived.
        post("/activities/{{id}}/attempts/typed" => controllers::activity_typed_controller::submit),
        // rullst-access: owner — subject and clock are server-derived; rows remain school/enrollment scoped.
        get("/reviews/due" => controllers::review_controller::index),
        // rullst-access: owner — assignment and learner are derived from path/session before entitlement checks.
        post("/assignments/{{id}}/submissions" => controllers::assignment_controller::submit),
        // rullst-access: role — a persisted evaluator/instructor/admin scores only server rubric criteria.
        post("/submissions/{{id}}/grade" => controllers::assignment_controller::grade),
        // rullst-access: role — only admin can append a reasoned grade correction bounded by the same rubric.
        post("/assignment-grades/{{id}}/correct" => controllers::assignment_controller::correct_grade),
        // rullst-access: owner — the session identity scopes the notification mutation.
        post("/notifications/{{id}}/read" => controllers::notification_controller::read),
        // rullst-access: owner — no subject identity is accepted from the form.
        post("/notifications/preferences" => controllers::notification_controller::update_preference),
        // rullst-access: role — publication service requires instructor/admin from authenticated context.
        post("/courses/{{id}}/versions" => controllers::publication_controller::draft),
        // rullst-access: role — only the authenticated version author or admin can submit review.
        post("/course-versions/{{id}}/submit" => controllers::publication_controller::submit),
        // rullst-access: role — publication service requires a distinct authenticated admin reviewer.
        post("/course-versions/{{id}}/review" => controllers::publication_controller::review),
        // rullst-access: role — rollback creates a new immutable version and requires an authenticated admin.
        post("/courses/{{id}}/rollback" => controllers::publication_rollback_controller::rollback),
        // rullst-access: role — target identity is path-bound and the service enforces the grant hierarchy.
        post("/users/{{id}}/roles" => controllers::role_controller::grant),
        // rullst-access: role — the authenticated grant hierarchy controls durable revocation.
        post("/role-assignments/{{assignment_key}}/revoke" => controllers::role_controller::revoke),
        // rullst-access: role — the service requires admin and records actor, reason and server time.
        post("/certificates/{{certificate_key}}/revoke" => controllers::completion_controller::revoke),
    ].layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware));

    let router = public
        .merge_axum(learning.into_axum())
        .layer(rullst::server::from_fn(rullst::security::csrf_middleware))
        .layer(rullst::server::from_fn(rullst::security::headers_middleware))
        .nest_axum("/nexus", nexus);

    #[cfg(debug_assertions)]
    {{
        rullst::runtime::spawn(async {{
            if let Err(error) = rullst::studio::run_studio(5555).await {{
                eprintln!("Rullst Studio could not start: {{error}}");
            }}
        }});
        println!("📊 Rullst Studio running on http://127.0.0.1:5555");
    }}
    println!("🚀 LMS server starting on port 3000...");
    Server::new(router).run(3000).await?;
    Ok(())
}}
"##,
            repo_decl = repo_decl
        );
        manifest.push(("src/main.rs", main_rs));
    }

    manifest
}
