//! Route composition for the detached gamification profile.

pub(super) fn main_source() -> String {
    r##"use rullst::{routes, Router, Server};

pub mod controllers;
pub mod middlewares;
pub mod migrations;
pub mod models;
pub mod pages;
pub mod services;

pub fn router() -> Result<Router, Box<dyn std::error::Error>> {
    let nexus_auth = rullst::nexus::NexusAuthPolicy::local_development_or_basic_from_env()?;
    let nexus = rullst::nexus::Nexus::new()
        .with_auth_policy(nexus_auth)
        .with_brand("LMS Gamification Admin")
        .register::<models::category::Category>()
        .register::<models::course::Course>()
        .register::<models::course_module::CourseModule>()
        .register::<models::lesson::Lesson>()
        .register::<models::user::User>()
        .register::<models::enrollment::Enrollment>()
        .register::<models::lesson_progress::LessonProgress>()
        .register::<models::activity::Activity>()
        .register::<models::score_event::ScoreEvent>()
        .register::<models::leaderboard_entry::LeaderboardEntry>()
        .try_build()?;

    let public = routes![
        get("/" => controllers::lms_controller::index),
        // rullst-access: public — bounded published course metadata forms the public catalog.
        get("/courses/{id}" => controllers::lms_controller::show_course),
        get("/login" => controllers::auth_controller::login_view),
        post("/login" => controllers::auth_controller::login_submit),
        get("/register" => controllers::auth_controller::register_view),
        post("/register" => controllers::auth_controller::register_submit),
        post("/logout" => controllers::auth_controller::logout),
    ];
    let learning = routes![
        get("/dashboard" => controllers::auth_controller::dashboard),
        // rullst-access: owner — the authenticated session owns the enrollment created by the service.
        post("/courses/{id}/enroll" => controllers::learning_controller::enroll),
        // rullst-access: owner — the handler requires the authenticated learner's active enrollment.
        get("/lessons/{id}/play" => controllers::learning_controller::play_lesson),
        // rullst-access: owner — progress is persisted only for the authenticated learner and lesson.
        post("/lessons/{id}/progress" => controllers::learning_controller::record_progress),
        // rullst-access: owner — leaderboard reads require the authenticated learner's course enrollment.
        get("/courses/{id}/leaderboards/{season}" => controllers::gamification_controller::leaderboard),
    ].layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware));

    Ok(public
        .merge_axum(learning.into_axum())
        .layer(rullst::server::from_fn(rullst::security::csrf_middleware))
        .layer(rullst::server::from_fn(rullst::security::headers_middleware))
        .nest_axum("/nexus", nexus))
}

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    rullst::artisan!(crate::migrations::get_migrations());
    #[cfg(debug_assertions)]
    rullst::runtime::spawn(async {
        if let Err(error) = rullst::studio::run_studio(5555).await {
            eprintln!("Rullst Studio could not start: {error}");
        }
    });
    Server::new(router()?).run(3000).await?;
    Ok(())
}
"##
    .to_string()
}
