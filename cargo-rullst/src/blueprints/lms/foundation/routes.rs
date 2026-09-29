//! Entrypoint templates for the auth + learning LMS starter.

const MODULES: &str = "pub mod controllers;
pub mod middlewares;
pub mod migrations;
pub mod models;
pub mod pages;
pub mod services;
";

const ROUTER: &str = r##"pub fn router() -> Result<Router, Box<dyn std::error::Error>> {
    let nexus_auth = rullst::nexus::NexusAuthPolicy::local_development_or_basic_from_env()?;
    let nexus = rullst::nexus::Nexus::new()
        .with_auth_policy(nexus_auth)
        .with_brand("LMS Foundation Admin")
        .register::<models::category::Category>()
        .register::<models::course::Course>()
        .register::<models::course_module::CourseModule>()
        .register::<models::lesson::Lesson>()
        .register::<models::user::User>()
        .register::<models::enrollment::Enrollment>()
        .register::<models::lesson_progress::LessonProgress>()
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
    ].layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware));

    Ok(public
        .merge_axum(learning.into_axum())
        .layer(rullst::server::from_fn(rullst::security::csrf_middleware))
        .layer(rullst::server::from_fn(rullst::security::headers_middleware))
        .nest_axum("/nexus", nexus))
}
"##;

const STARTUP: &str = r##"    rullst::artisan!(crate::migrations::get_migrations());
    #[cfg(debug_assertions)]
    rullst::runtime::spawn(async {
        if let Err(error) = rullst::studio::run_studio(5555).await {
            eprintln!("Rullst Studio could not start: {error}");
        }
    });
"##;

/// `src/main.rs` without hot reload: the binary owns the router.
pub(super) fn main_source() -> String {
    format!(
        "use rullst::{{routes, Router, Server}};\n\n{MODULES}\n{ROUTER}\n#[rullst::runtime::main]\nasync fn main() -> Result<(), Box<dyn std::error::Error>> {{\n{STARTUP}    Server::new(router()?).run(3000).await?;\n    Ok(())\n}}\n"
    )
}

/// `src/lib.rs` with hot reload: the reloadable library exports the router.
pub(super) fn hot_lib_source() -> String {
    format!(
        r##"use rullst::{{routes, Router}};

{MODULES}
{ROUTER}
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
"##
    )
}

/// `src/main.rs` with hot reload: `HOT_RELOAD` loads the rebuilt library.
pub(super) fn hot_main_source(project_name_safe: &str) -> String {
    format!(
        r##"{MODULES}
#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
{STARTUP}    let server = if std::env::var("HOT_RELOAD").is_ok() {{
        let lib_path = if cfg!(target_os = "windows") {{
            "target/debug/{project_name_safe}"
        }} else {{
            "target/debug/lib{project_name_safe}"
        }};
        rullst::Server::new_hot(lib_path)
    }} else {{
        rullst::Server::new({project_name_safe}::router()?)
    }};
    server.run(3000).await?;
    Ok(())
}}
"##
    )
}
