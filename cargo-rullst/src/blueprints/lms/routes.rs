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
        .register::<models::lesson::Lesson>()
        .register::<models::user::User>()
        .register::<models::enrollment::Enrollment>()
        .register::<models::lesson_progress::LessonProgress>()
        .try_build()?;

    let public = routes![
        get("/" => controllers::lms_controller::index),
        // rullst-access: public — course metadata and lesson titles form the public catalog.
        get("/courses/{{id}}" => controllers::lms_controller::show_course),
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
        .register::<models::lesson::Lesson>()
        .register::<models::user::User>()
        .register::<models::enrollment::Enrollment>()
        .register::<models::lesson_progress::LessonProgress>()
        .try_build()?;

    let public = routes![
        get("/" => controllers::lms_controller::index),
        // rullst-access: public — course metadata and lesson titles form the public catalog.
        get("/courses/{{id}}" => controllers::lms_controller::show_course),
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
