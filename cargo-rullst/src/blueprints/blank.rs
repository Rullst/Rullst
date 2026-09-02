// src/blueprints/blank.rs — Blank Starter blueprint templates.

use super::common;

mod client;
mod turso;

pub fn file_manifest(
    project_name: &str,
    project_name_safe: &str,
    api: bool,
    hot_reload: bool,
    db_needed: bool,
    orm_pattern: &str,
    frontend_engine: &str,
) -> Vec<(&'static str, String)> {
    let mut manifest = Vec::new();
    let is_repo = common::is_repo_mode(orm_pattern);
    let _ = (
        project_name,
        project_name_safe,
        api,
        hot_reload,
        frontend_engine,
    );
    let turso_primary = orm_pattern == "Turso Active Record";

    let db_model_code = if turso_primary {
        "// Primary Turso/libSQL model. The familiar Orm derive targets libSQL explicitly.\n#[derive(Debug, Clone, rullst_orm::Orm)]\n#[orm(table = \"users\", backend = \"turso\")]\npub struct User {\n    pub id: i64,\n    pub name: String,\n}\n"
    } else if db_needed {
        "use rullst::db::{Orm, FromRow};\n\n// 1. Define your database model using the built-in rullst-orm ORM!\n#[derive(Debug, Clone, FromRow, Orm)]\n#[orm(table = \"users\")]\npub struct User {\n    pub id: i32,\n    pub name: String,\n}\n"
    } else {
        ""
    };

    let db_status_code = if turso_primary {
        "    // Typed Turso Active Record query through the primary Hrana transport.\n    let db_status = match User::all().await {\n        Ok(_) => \"Database connected.\".to_string(),\n        Err(error) => {\n            tracing::warn!(error = %error, \"database status check failed\");\n            \"Database unavailable.\".to_string()\n        }\n    };"
    } else if db_needed {
        "    // ORM usage example: verify availability without exposing database details.\n    let db_status = match User::all().await {\n        Ok(_) => \"Database connected.\".to_string(),\n        Err(error) => {\n            tracing::warn!(error = %error, \"database status check failed\");\n            \"Database unavailable.\".to_string()\n        }\n    };"
    } else {
        "    let db_status = \"Database features are disabled for this project.\".to_string();"
    };

    let migrations_mod_declaration = if db_needed {
        "pub mod migrations;\n"
    } else {
        ""
    };

    let artisan_call = if turso_primary {
        r#"    // Turso is initialized before routes use the global typed model facade.
    let _ = dotenvy::dotenv();
    rullst_orm::polyglot::TursoOrm::init_from_env().await?;
    if let Some(command) = std::env::args().nth(1) {
        match command.as_str() {
            "db:migrate" => {
                let report = rullst_orm::polyglot::TursoOrm::migrate(
                    crate::migrations::get_migrations()?,
                ).await?;
                println!("Applied {} Turso migration(s); {} already current.", report.applied.len(), report.skipped.len());
                return Ok(());
            }
            "db:rollback" => {
                let report = rullst_orm::polyglot::TursoOrm::rollback_last(
                    crate::migrations::get_migrations()?,
                ).await?;
                match report.rolled_back {
                    Some(name) => println!("Rolled back Turso migration {name}."),
                    None => println!("No Turso migrations to roll back."),
                }
                return Ok(());
            }
            "db:status" => {
                let applied = rullst_orm::polyglot::TursoOrm::migration_status().await?;
                if applied.is_empty() {
                    println!("No Turso migrations applied.");
                } else {
                    for name in applied { println!("[applied] {name}"); }
                }
                return Ok(());
            }
            "db:seed" => {
                println!("No Turso seeders are configured in this blank starter.");
                return Ok(());
            }
            _ => {}
        }
    }
"#
    } else if db_needed {
        "    // 1. Intercept Artisan commands (e.g. cargo rullst db:migrate) before starting server\n    rullst::artisan!(crate::migrations::get_migrations());\n"
    } else {
        ""
    };
    let client_modules = if !api {
        "pub mod islands;\npub mod rpc;\n"
    } else {
        ""
    };
    let rpc_module = if !api { "mod rpc;\n" } else { "" };

    if hot_reload {
        let lib_rs = if api {
            format!(
                r##"use rullst::{{routes, Router, response::IntoResponse}};
use serde::Serialize;

{migrations_mod_declaration}{client_modules}

{db_model_code}

#[derive(Serialize)]
struct HomeResponse {{
    message: String,
    database_status: String,
}}

pub async fn home() -> impl IntoResponse {{
    let name = "Rullst";
{db_status_code}

    rullst::server::Json(HomeResponse {{
        message: format!("Welcome to Rullst REST API: {{}}", name),
        database_status: db_status,
    }})
}}

pub fn router() -> Router {{
    routes![
        get("/" => home),
    ].layer(rullst::server::from_fn(rullst::security::headers_middleware))
}}

#[unsafe(no_mangle)]
pub extern "C" fn rullst_router_init() -> *mut Router {{
    Box::into_raw(Box::new(router()))
}}
"##,
                migrations_mod_declaration = migrations_mod_declaration,
                client_modules = client_modules,
                db_model_code = db_model_code,
                db_status_code = db_status_code
            )
        } else {
            let fe_imports = common::frontend_page_imports(frontend_engine);
            format!(
                r##"{fe_imports}use rullst::{{routes, Router, response::{{Html, IntoResponse}}, server::Extension}};
use rullst::htmx::{{HtmxRequest, render_page}};

{migrations_mod_declaration}{client_modules}

{db_model_code}

// Main route: uses hybrid SSR with render_page
pub async fn home(
    htmx: HtmxRequest,
    Extension(csrf_token): Extension<rullst::security::CsrfToken>,
) -> impl IntoResponse {{
    let name = "Rullst";
{db_status_code}

    let content = html! {{
        <div class="flex flex-col items-center justify-center min-h-screen bg-slate-950 text-slate-100 p-6 font-sans">
            <div class="max-w-xl text-center space-y-6">
                <h1 class="text-5xl font-extrabold tracking-tight bg-gradient-to-r from-sky-400 via-indigo-400 to-purple-500 bg-clip-text text-transparent">
                    "Welcome to " {{name}}
                </h1>
                
                <p class="text-slate-400 text-lg">
                    "A full-stack Rust framework focused on explicit security boundaries, maintainability, and measured speed."
                </p>

                <div class="inline-block px-4 py-2 bg-slate-900 border border-slate-800 rounded-lg text-sm text-sky-400 font-mono">
                    {{db_status}}
                </div>

                <div class="bg-slate-900/50 backdrop-blur-md p-6 rounded-xl border border-slate-800 space-y-4">
                    <h2 class="text-xl font-bold text-slate-200">"HTMX Reactive Counter"</h2>
                    <div id="counter-box" class="flex flex-col items-center gap-3">
                        <form method="post" action="/clicked" hx-post="/clicked" hx-target="#counter-box" hx-swap="outerHTML">
                            <input type="hidden" name="_token" value={{csrf_token.as_str()}} />
                            <button type="submit" class="px-6 py-2.5 bg-gradient-to-r from-sky-500 to-indigo-600 hover:from-sky-400 hover:to-indigo-500 text-white font-medium rounded-lg shadow-lg hover:shadow-indigo-500/20 active:scale-95 transition duration-150 ease-in-out cursor-pointer">
                            "Click here to increment"
                            </button>
                        </form>
                        <p class="text-sm text-slate-400">"Clicks received on server: 0"</p>
                    </div>
                </div>

                <div class="bg-slate-900/50 backdrop-blur-md p-6 rounded-xl border border-slate-800 space-y-4">
                    <h2 class="text-xl font-bold text-slate-200">"Wasm Island (Client Side)"</h2>
                    <div data-island="counter" data-props="{{\"props\": {{\"initial_value\": 0}}}}"></div>
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
pub async fn clicked(
    Extension(csrf_token): Extension<rullst::security::CsrfToken>,
) -> impl IntoResponse {{
    let current_clicks = CLICK_COUNT.fetch_add(1, Ordering::SeqCst) + 1;
    
    Html(html! {{
        <div id="counter-box" class="flex flex-col items-center gap-3">
            <form method="post" action="/clicked" hx-post="/clicked" hx-target="#counter-box" hx-swap="outerHTML">
                <input type="hidden" name="_token" value={{csrf_token.as_str()}} />
                <button type="submit" class="px-6 py-2.5 bg-gradient-to-r from-sky-500 to-indigo-600 hover:from-sky-400 hover:to-indigo-500 text-white font-medium rounded-lg shadow-lg hover:shadow-indigo-500/20 active:scale-95 transition duration-150 ease-in-out cursor-pointer">
                    "Click here to increment"
                </button>
            </form>
            <p class="text-sm text-emerald-400 font-medium">"Clicks received on server: " {{current_clicks.to_string()}}</p>
        </div>
    }})
}}

pub fn router() -> Router {{
    routes![
        get("/" => home),
        post("/clicked" => clicked),
    ]
    .merge_axum(rpc::increment_counter_rpc_router().into_axum())
    .layer(rullst::server::from_fn(rullst::security::csrf_middleware))
    .layer(rullst::server::from_fn(rullst::security::headers_middleware))
}}

#[unsafe(no_mangle)]
pub extern "C" fn rullst_router_init() -> *mut Router {{
    Box::into_raw(Box::new(router()))
}}
"##,
                migrations_mod_declaration = migrations_mod_declaration,
                client_modules = client_modules,
                db_model_code = db_model_code,
                db_status_code = db_status_code
            )
        };

        manifest.push(("src/lib.rs", lib_rs));

        if !api {
            let island_counter = include_str!("../generators/island.rs.template")
                .replace("__MODULE_NAME__", "counter")
                .replace("__TYPE_NAME__", "Counter");
            manifest.push(("src/islands/mod.rs", "pub mod counter;\n".to_string()));
            manifest.push(("src/islands/counter.rs", island_counter));
        }

        let main_rs = format!(
            r##"{migrations_mod_declaration}

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
{artisan_call}
    let is_hot = std::env::var("HOT_RELOAD").is_ok();

    let server = if is_hot {{
        let lib_path = if cfg!(target_os = "windows") {{
            "target/debug/{project_name_safe}"
        }} else {{
            "target/debug/lib{project_name_safe}"
        }};
        rullst::Server::new_hot(lib_path)
    }} else {{
        let router = {project_name_safe}::router();
        rullst::Server::new(router)
    }};

    server.run(3000).await?;

    Ok(())
}}
"##,
            project_name_safe = project_name_safe,
            migrations_mod_declaration = migrations_mod_declaration,
            artisan_call = artisan_call
        );

        manifest.push(("src/main.rs", main_rs));
    } else {
        let main_rs = if api {
            format!(
                r##"use rullst::{{routes, Server, response::IntoResponse}};
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

    rullst::server::Json(HomeResponse {{
        message: format!("Welcome to Rullst REST API: {{}}", name),
        database_status: db_status,
    }})
}}

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
{artisan_call}
    let router = routes![
        get("/" => home),
    ].layer(rullst::server::from_fn(rullst::security::headers_middleware));

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
                r##"use rullst::{{html, routes, Server, response::{{Html, IntoResponse}}, server::Extension}};
use rullst::htmx::{{HtmxRequest, render_page}};

{migrations_mod_declaration}{rpc_module}

{db_model_code}

async fn home(
    htmx: HtmxRequest,
    Extension(csrf_token): Extension<rullst::security::CsrfToken>,
) -> impl IntoResponse {{
    let name = "Rullst";
{db_status_code}

    let content = html! {{
        <div class="flex flex-col items-center justify-center min-h-screen bg-slate-950 text-slate-100 p-6 font-sans">
            <div class="max-w-xl text-center space-y-6">
                <h1 class="text-5xl font-extrabold tracking-tight bg-gradient-to-r from-sky-400 via-indigo-400 to-purple-500 bg-clip-text text-transparent">
                    "Welcome to " {{name}}
                </h1>
                
                <p class="text-slate-400 text-lg">
                    "A full-stack Rust framework focused on explicit security boundaries, maintainability, and measured speed."
                </p>

                <div class="inline-block px-4 py-2 bg-slate-900 border border-slate-800 rounded-lg text-sm text-sky-400 font-mono">
                    {{db_status}}
                </div>

                <div class="bg-slate-900/50 backdrop-blur-md p-6 rounded-xl border border-slate-800 space-y-4">
                    <h2 class="text-xl font-bold text-slate-200">"HTMX Reactive Counter"</h2>
                    <div id="counter-box" class="flex flex-col items-center gap-3">
                        <form method="post" action="/clicked" hx-post="/clicked" hx-target="#counter-box" hx-swap="outerHTML">
                            <input type="hidden" name="_token" value={{csrf_token.as_str()}} />
                            <button type="submit" class="px-6 py-2.5 bg-gradient-to-r from-sky-500 to-indigo-600 hover:from-sky-400 hover:to-indigo-500 text-white font-medium rounded-lg shadow-lg hover:shadow-indigo-500/20 active:scale-95 transition duration-150 ease-in-out cursor-pointer">
                            "Click here to increment"
                            </button>
                        </form>
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
async fn clicked(
    Extension(csrf_token): Extension<rullst::security::CsrfToken>,
) -> impl IntoResponse {{
    let current_clicks = CLICK_COUNT.fetch_add(1, Ordering::SeqCst) + 1;
    
    Html(html! {{
        <div id="counter-box" class="flex flex-col items-center gap-3">
            <form method="post" action="/clicked" hx-post="/clicked" hx-target="#counter-box" hx-swap="outerHTML">
                <input type="hidden" name="_token" value={{csrf_token.as_str()}} />
                <button type="submit" class="px-6 py-2.5 bg-gradient-to-r from-sky-500 to-indigo-600 hover:from-sky-400 hover:to-indigo-500 text-white font-medium rounded-lg shadow-lg hover:shadow-indigo-500/20 active:scale-95 transition duration-150 ease-in-out cursor-pointer">
                    "Click here to increment"
                </button>
            </form>
            <p class="text-sm text-emerald-400 font-medium">"Clicks received on server: " {{current_clicks.to_string()}}</p>
        </div>
    }})
}}

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
{artisan_call}
    let router = routes![
        get("/" => home),
        post("/clicked" => clicked),
    ]
    .merge_axum(rpc::increment_counter_rpc_router().into_axum())
    .layer(rullst::server::from_fn(rullst::security::csrf_middleware))
    .layer(rullst::server::from_fn(rullst::security::headers_middleware));

    Server::new(router)
        .run(3000)
        .await?;

    Ok(())
}}
"##,
                migrations_mod_declaration = migrations_mod_declaration,
                rpc_module = rpc_module,
                db_model_code = db_model_code,
                db_status_code = db_status_code,
                artisan_call = artisan_call
            )
        };

        manifest.push(("src/main.rs", main_rs));
    }

    if !api {
        manifest.push(("src/rpc.rs", client::rpc_source()));
    }

    if turso_primary {
        manifest.extend(turso::migration_files());
    } else if db_needed {
        let migration = r##"use rullst::db::schema::{Schema, Migration};
use rullst::db::async_trait;

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str {
        "m20260601000000_create_users_table"
    }

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::create("users", |table| {
            table.id();
            table.string("name").not_null();
            table.timestamps();
        }).await
    }

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::drop_if_exists("users").await
    }
}
"##;
        manifest.push((
            "src/migrations/m20260601000000_create_users_table.rs",
            migration.to_string(),
        ));

        let migrations_mod = r##"// Generated by Rullst.
pub mod m20260601000000_create_users_table;

pub fn get_migrations() -> Vec<Box<dyn rullst::db::schema::Migration>> {
    vec![
        Box::new(m20260601000000_create_users_table::MigrationImpl),
    ]
}
"##;
        manifest.push(("src/migrations/mod.rs", migrations_mod.to_string()));

        if is_repo {
            manifest.push((
                "src/repositories/user_repository.rs",
                common::generate_repository("User", "users"),
            ));
            manifest.push((
                "src/repositories/mod.rs",
                common::generate_repositories_mod(&["User"]),
            ));
        }
    }

    manifest
}
