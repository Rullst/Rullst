// src/blueprints/blank.rs — Blank Starter blueprint templates.

pub fn file_manifest(
    project_name: &str,
    project_name_safe: &str,
    api: bool,
    hot_reload: bool,
    db_needed: bool,
) -> Vec<(&'static str, String)> {
    let mut manifest = Vec::new();

    let db_model_code = if db_needed {
        "use rullst_orm::{Orm, RullstModel, sqlx::{self, FromRow}};\n\n// 1. Define your database model using the built-in rullst-orm ORM!\n#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]\n#[orm(table = \"users\")]\npub struct User {\n    pub id: i32,\n    pub name: String,\n}\n"
    } else {
        ""
    };

    let db_status_code = if db_needed {
        "    // ORM usage example: Fetch active users from database\n    let db_status = match User::all().await {\n        Ok(users) => format!(\"Database connected! Total users: {}\", users.len()),\n        Err(e) => format!(\"Database offline or not configured: {}\", e),\n    };"
    } else {
        "    let db_status = \"Database features are disabled for this project.\".to_string();"
    };

    let migrations_mod_declaration = if db_needed {
        "pub mod migrations;\n"
    } else {
        ""
    };

    let artisan_call = if db_needed {
        "    // 1. Intercept Artisan commands (e.g. cargo rullst db:migrate) before starting server\n    rullst::artisan!(crate::migrations::get_migrations());\n"
    } else {
        ""
    };

    if hot_reload {
        let lib_rs = if api {
            format!(
                r##"use rullst::{{routes, Router, response::IntoResponse}};
use serde::Serialize;

{migrations_mod_declaration}

{db_model_code}

#[derive(Serialize)]
struct HomeResponse {{
    message: String,
    database_status: String,
}}

pub async fn home() -> impl IntoResponse {{
    let name = "Rullst";
{db_status_code}

    axum::Json(HomeResponse {{
        message: format!("Welcome to Rullst REST API: {{}}", name),
        database_status: db_status,
    }})
}}

#[unsafe(no_mangle)]
pub extern "C" fn rullst_router_init() -> *mut Router {{
    let router = routes![
        get("/" => home),
    ];
    Box::into_raw(Box::new(router))
}}
"##,
                migrations_mod_declaration = migrations_mod_declaration,
                db_model_code = db_model_code,
                db_status_code = db_status_code
            )
        } else {
            format!(
                r##"use rullst::{{html, routes, Router, response::{{Html, IntoResponse}}}};
use rullst::htmx::{{HtmxRequest, render_page}};

{migrations_mod_declaration}

{db_model_code}

// Main route: uses hybrid SSR with render_page
pub async fn home(htmx: HtmxRequest) -> impl IntoResponse {{
    let name = "Rullst";
{db_status_code}

    let content = html! {{
        <div class="flex flex-col items-center justify-center min-h-screen bg-slate-950 text-slate-100 p-6 font-sans">
            <div class="max-w-xl text-center space-y-6">
                <h1 class="text-5xl font-extrabold tracking-tight bg-gradient-to-r from-sky-400 via-indigo-400 to-purple-500 bg-clip-text text-transparent">
                    "Welcome to " {{name}}
                </h1>
                
                <p class="text-slate-400 text-lg">
                    "The ultimate full-stack framework for Rust. Focused on Security, Maintainability, and Speed."
                </p>

                <div class="inline-block px-4 py-2 bg-slate-900 border border-slate-800 rounded-lg text-sm text-sky-400 font-mono">
                    {{db_status}}
                </div>

                <div class="bg-slate-900/50 backdrop-blur-md p-6 rounded-xl border border-slate-800 space-y-4">
                    <h2 class="text-xl font-bold text-slate-200">"HTMX Reactive Counter"</h2>
                    <div id="counter-box" class="flex flex-col items-center gap-3">
                        <button hx-post="/clicked" 
                                hx-target="#counter-box" 
                                hx-swap="outerHTML" 
                                class="px-6 py-2.5 bg-gradient-to-r from-sky-500 to-indigo-600 hover:from-sky-400 hover:to-indigo-500 text-white font-medium rounded-lg shadow-lg hover:shadow-indigo-500/20 active:scale-95 transition duration-150 ease-in-out cursor-pointer">
                            "Click here to increment"
                        </button>
                        <p class="text-sm text-slate-400">"Clicks received on server: 0"</p>
                    </div>
                </div>
            </div>
        </div>
    }};

    render_page(&htmx, "Welcome to Rullst", content)
}}

// State for counter
use std::sync::atomic::{{AtomicUsize, Ordering}};
static CLICK_COUNT: AtomicUsize = AtomicUsize::new(0);

// Reactive HTMX endpoint
pub async fn clicked() -> impl IntoResponse {{
    let current_clicks = CLICK_COUNT.fetch_add(1, Ordering::SeqCst) + 1;
    
    Html(html! {{
        <div id="counter-box" class="flex flex-col items-center gap-3">
            <button hx-post="/clicked" 
                    hx-target="#counter-box" 
                    hx-swap="outerHTML" 
                    class="px-6 py-2.5 bg-gradient-to-r from-sky-500 to-indigo-600 hover:from-sky-400 hover:to-indigo-500 text-white font-medium rounded-lg shadow-lg hover:shadow-indigo-500/20 active:scale-95 transition duration-150 ease-in-out cursor-pointer">
                "Click here to increment"
            </button>
            <p class="text-sm text-emerald-400 font-medium">"Clicks received on server: " {{current_clicks.to_string()}}</p>
        </div>
    }})
}}

#[unsafe(no_mangle)]
pub extern "C" fn rullst_router_init() -> *mut Router {{
    let router = routes![
        get("/" => home),
        post("/clicked" => clicked),
    ];
    Box::into_raw(Box::new(router))
}}
"##,
                migrations_mod_declaration = migrations_mod_declaration,
                db_model_code = db_model_code,
                db_status_code = db_status_code
            )
        };

        manifest.push(("src/lib.rs", lib_rs));

        let main_rs = format!(
            r##"{migrations_mod_declaration}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
{artisan_call}
    let is_hot = std::env::var("HOT_RELOAD").is_ok();

    let server = if is_hot {{
        let lib_path = if cfg!(target_os = "windows") {{
            "target/debug/{project_name}"
        }} else {{
            "target/debug/lib{project_name}"
        }};
        rullst::Server::new_hot(lib_path)
    }} else {{
        let router_ptr = {project_name_safe}::rullst_router_init();
        let router = unsafe {{ *Box::from_raw(router_ptr) }};
        rullst::Server::new(router)
    }};

    server.run(3000).await?;

    Ok(())
}}
"##,
            project_name = project_name,
            project_name_safe = project_name_safe,
            migrations_mod_declaration = migrations_mod_declaration,
            artisan_call = artisan_call
        );

        manifest.push(("src/main.rs", main_rs));
    } else {
        let main_rs = if api {
            format!(
                r##"use rullst::{{routes, Server, Router, response::IntoResponse}};
use serde::Serialize;

{migrations_mod_declaration}

{db_model_code}

#[derive(Serialize)]
struct HomeResponse {{
    message: String,
    database_status: String,
}}

async fn home() -> impl IntoResponse {{
    let name = "Rullst";
{db_status_code}

    axum::Json(HomeResponse {{
        message: format!("Welcome to Rullst REST API: {{}}", name),
        database_status: db_status,
    }})
}}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
{artisan_call}
    let router = routes![
        get("/" => home),
    ];

    Server::new(router)
        .run(3000)
        .await?;

    Ok(())
}}
"##,
                migrations_mod_declaration = migrations_mod_declaration,
                db_model_code = db_model_code,
                db_status_code = db_status_code,
                artisan_call = artisan_call
            )
        } else {
            format!(
                r##"use rullst::{{html, routes, Server, response::{{Html, IntoResponse}}}};
use rullst::htmx::{{HtmxRequest, render_page}};

{migrations_mod_declaration}

{db_model_code}

async fn home(htmx: HtmxRequest) -> impl IntoResponse {{
    let name = "Rullst";
{db_status_code}

    let content = html! {{
        <div class="flex flex-col items-center justify-center min-h-screen bg-slate-950 text-slate-100 p-6 font-sans">
            <div class="max-w-xl text-center space-y-6">
                <h1 class="text-5xl font-extrabold tracking-tight bg-gradient-to-r from-sky-400 via-indigo-400 to-purple-500 bg-clip-text text-transparent">
                    "Welcome to " {{name}}
                </h1>
                
                <p class="text-slate-400 text-lg">
                    "The ultimate full-stack framework for Rust. Focused on Security, Maintainability, and Speed."
                </p>

                <div class="inline-block px-4 py-2 bg-slate-900 border border-slate-800 rounded-lg text-sm text-sky-400 font-mono">
                    {{db_status}}
                </div>

                <div class="bg-slate-900/50 backdrop-blur-md p-6 rounded-xl border border-slate-800 space-y-4">
                    <h2 class="text-xl font-bold text-slate-200">"HTMX Reactive Counter"</h2>
                    <div id="counter-box" class="flex flex-col items-center gap-3">
                        <button hx-post="/clicked" 
                                hx-target="#counter-box" 
                                hx-swap="outerHTML" 
                                class="px-6 py-2.5 bg-gradient-to-r from-sky-500 to-indigo-600 hover:from-sky-400 hover:to-indigo-500 text-white font-medium rounded-lg shadow-lg hover:shadow-indigo-500/20 active:scale-95 transition duration-150 ease-in-out cursor-pointer">
                            "Click here to increment"
                        </button>
                        <p class="text-sm text-slate-400">"Clicks received on server: 0"</p>
                    </div>
                </div>
            </div>
        </div>
    }};

    render_page(&htmx, "Welcome to Rullst", content)
}}

// State for counter
use std::sync::atomic::{{AtomicUsize, Ordering}};
static CLICK_COUNT: AtomicUsize = AtomicUsize::new(0);

// Reactive HTMX endpoint
async fn clicked() -> impl IntoResponse {{
    let current_clicks = CLICK_COUNT.fetch_add(1, Ordering::SeqCst) + 1;
    
    Html(html! {{
        <div id="counter-box" class="flex flex-col items-center gap-3">
            <button hx-post="/clicked" 
                    hx-target="#counter-box" 
                    hx-swap="outerHTML" 
                    class="px-6 py-2.5 bg-gradient-to-r from-sky-500 to-indigo-600 hover:from-sky-400 hover:to-indigo-500 text-white font-medium rounded-lg shadow-lg hover:shadow-indigo-500/20 active:scale-95 transition duration-150 ease-in-out cursor-pointer">
                "Click here to increment"
            </button>
            <p class="text-sm text-emerald-400 font-medium">"Clicks received on server: " {{current_clicks.to_string()}}</p>
        </div>
    }})
}}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
{artisan_call}
    let router = routes![
        get("/" => home),
        post("/clicked" => clicked),
    ];

    Server::new(router)
        .run(3000)
        .await?;

    Ok(())
}}
"##,
                migrations_mod_declaration = migrations_mod_declaration,
                db_model_code = db_model_code,
                db_status_code = db_status_code,
                artisan_call = artisan_call
            )
        };

        manifest.push(("src/main.rs", main_rs));
    }

    manifest
}
