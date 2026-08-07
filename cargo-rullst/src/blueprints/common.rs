// src/blueprints/common.rs — Shared code generation helpers for all blueprints.
// These functions generate ORM-pattern and frontend-engine specific code snippets
// that are injected consistently into every blueprint's file manifest.

/// Returns the extra module declarations for Repository/Hybrid patterns.
/// In Active Record mode, returns empty string (no extra modules).
pub fn repo_mod_decl(orm_pattern: &str) -> &'static str {
    if orm_pattern.contains("Repository") || orm_pattern.contains("Hybrid") {
        "pub mod repositories;\n"
    } else {
        ""
    }
}

/// Returns true if repo-pattern modules should be generated.
pub fn is_repo_mode(orm_pattern: &str) -> bool {
    orm_pattern.contains("Repository") || orm_pattern.contains("Hybrid")
}

/// Generates repository boilerplate for a given model name.
/// Example: model_name = "User", table = "users"
pub fn generate_repository(model_name: &str, table: &str) -> String {
    let snake = model_name
        .chars()
        .enumerate()
        .flat_map(|(i, c)| {
            if c.is_uppercase() && i != 0 {
                vec!['_', c.to_ascii_lowercase()]
            } else {
                vec![c.to_ascii_lowercase()]
            }
        })
        .collect::<String>();

    format!(
        r##"use crate::models::{snake}::{model_name};
use rullst::db::{{Orm, sqlx}};

pub struct {model_name}Repository;

impl {model_name}Repository {{
    pub async fn find_all() -> Result<Vec<{model_name}>, rullst_orm::error::RullstError> {{
        let pool = Orm::pool();
        let rows = sqlx::query_as::<_, {model_name}>("SELECT * FROM {table}")
            .fetch_all(pool)
            .await?;
        Ok(rows)
    }}

    pub async fn find_by_id(id: i64) -> Result<Option<{model_name}>, rullst_orm::error::RullstError> {{
        let pool = Orm::pool();
        let row = sqlx::query_as::<_, {model_name}>("SELECT * FROM {table} WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;
        Ok(row)
    }}

    pub async fn count() -> Result<i64, rullst_orm::error::RullstError> {{
        let pool = Orm::pool();
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM {table}")
            .fetch_one(pool)
            .await?;
        Ok(row.0)
    }}
}}
"##,
        snake = snake,
        model_name = model_name,
        table = table,
    )
}

/// Generates the repositories/mod.rs content for a given list of model names.
pub fn generate_repositories_mod(models: &[&str]) -> String {
    let mods = models
        .iter()
        .map(|m| {
            let snake = m
                .chars()
                .enumerate()
                .flat_map(|(i, c)| {
                    if c.is_uppercase() && i != 0 {
                        vec!['_', c.to_ascii_lowercase()]
                    } else {
                        vec![c.to_ascii_lowercase()]
                    }
                })
                .collect::<String>();
            format!("pub mod {}_repository;", snake)
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("{}\n", mods)
}

/// Returns a comment/badge indicating the active frontend engine.
/// Used in generated page files as a header comment.
pub fn frontend_engine_badge(frontend_engine: &str) -> &'static str {
    if frontend_engine.contains("Leptos") {
        "⚡ Rullst Leptos SSR Adapter Engine Active"
    } else if frontend_engine.contains("Dioxus") {
        "🎨 Rullst Dioxus Virtual DOM SSR Engine Active"
    } else {
        "🔥 Rullst Zero-Bundle HTMX + Tailwind Engine Active"
    }
}

/// Returns the frontend adapter import comment to inject at top of page files.
pub fn frontend_adapter_comment(frontend_engine: &str) -> String {
    format!("// Frontend Adapter: {}", frontend_engine)
}

/// Returns the cargo dependency line for the chosen frontend engine.
pub fn frontend_cargo_dependency(frontend_engine: &str) -> String {
    if frontend_engine.contains("Leptos") {
        "leptos = { version = \"0.7\", features = [\"ssr\"] }\n".to_string()
    } else if frontend_engine.contains("Dioxus") {
        "dioxus = { version = \"0.6\", features = [\"ssr\"] }\n".to_string()
    } else {
        String::new()
    }
}

/// Generates appropriate imports for the page module depending on frontend engine.
pub fn frontend_page_imports(frontend_engine: &str) -> String {
    if frontend_engine.contains("Leptos") {
        format!(
            "// Frontend Adapter: {}\nuse leptos::prelude::*;\nuse rullst::html;\n",
            frontend_engine
        )
    } else if frontend_engine.contains("Dioxus") {
        format!(
            "// Frontend Adapter: {}\nuse dioxus::prelude::*;\nuse rullst::html;\n",
            frontend_engine
        )
    } else {
        format!(
            "// Frontend Adapter: {}\nuse rullst::html;\n",
            frontend_engine
        )
    }
}

/// Generates the page renderer code block.
/// For HTMX: uses `html!` macro.
/// For Leptos SSR: uses `leptos::ssr::render_to_string(...)`.
/// For Dioxus SSR: uses `dioxus_ssr::render_element(...)`.
pub fn render_page_layout(frontend_engine: &str) -> String {
    if frontend_engine.contains("Leptos") {
        r#"pub fn render_layout(title: &str, body_html: &str) -> String {
    view! {
        <!DOCTYPE html>
        <html lang="en" class="dark">
            <head>
                <meta charset="UTF-8" />
                <title>{title.to_string()}</title>
            </head>
            <body>
                <div class="leptos-ssr-container">
                    {body_html.to_string()}
                </div>
            </body>
        </html>
    }.to_html()
}
"#.to_string()
    } else if frontend_engine.contains("Dioxus") {
        r#"pub fn render_layout(title: &str, body_html: &str) -> String {
    dioxus::ssr::render_element(rsx! {
        div { class: "dioxus-ssr-container",
            h1 { "{title}" }
            div { "{body_html}" }
        }
    })
}
"#.to_string()
    } else {
        r#"pub fn render_layout(title: &str, body_html: &str) -> String {
    rullst::html! {
        <html lang="en" class="dark">
            <head>
                <meta charset="UTF-8" />
                <title>{title}</title>
            </head>
            <body>
                <div>{body_html}</div>
            </body>
        </html>
    }
}
"#.to_string()
    }
}

/// Generates the lib.rs router entry for hot-reload mode,
/// with dynamic repo_mod_decl injection.
pub fn hot_reload_lib_rs(routes_block: &str, nexus_block: &str, repo_mod_decl: &str) -> String {
    format!(
        r##"use rullst::{{routes, Router}};

pub mod migrations;
pub mod models;
{repo_mod_decl}pub mod controllers;
pub mod pages;

#[unsafe(no_mangle)]
pub extern "C" fn rullst_router_init() -> *mut Router {{
    {nexus_block}

    let router = {routes_block};

    Box::into_raw(Box::new(router))
}}
"##,
        repo_mod_decl = repo_mod_decl,
        nexus_block = nexus_block,
        routes_block = routes_block,
    )
}


