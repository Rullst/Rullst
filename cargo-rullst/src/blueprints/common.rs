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
    pub async fn find_all() -> Result<Vec<{model_name}>, rullst::orm::Error> {{
        let pool = Orm::pool()?;
        let rows = sqlx::query_as::<_, {model_name}>("SELECT * FROM {table}")
            .fetch_all(pool)
            .await?;
        Ok(rows)
    }}

    pub async fn find_by_id(id: i64) -> Result<Option<{model_name}>, rullst::orm::Error> {{
        let pool = Orm::pool()?;
        let row = sqlx::query_as::<_, {model_name}>("SELECT * FROM {table} WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;
        Ok(row)
    }}

    pub async fn count() -> Result<i64, rullst::orm::Error> {{
        let pool = Orm::pool()?;
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

/// Returns the cargo dependency line for the chosen frontend engine.
pub fn frontend_cargo_dependency(frontend_engine: &str) -> String {
    if frontend_engine.contains("Tera") || frontend_engine.contains("Template") {
        "tera = \"2.2\"\n".to_string()
    } else {
        String::new()
    }
}

/// Generates appropriate imports for the page module depending on frontend engine.
pub fn frontend_page_imports(frontend_engine: &str) -> String {
    format!(
        "// Frontend Engine: {}\nuse rullst::html;\n",
        frontend_engine
    )
}

/// Generates the page renderer code block using Rullst's compile-time `html!` macro.
pub fn render_page_layout(_frontend_engine: &str) -> String {
    r#"pub fn render_layout(title: &str, body: rullst::html::Markup) -> rullst::html::Markup {
    rullst::html! {
        <html lang="en" class="dark">
            <head>
                <meta charset="UTF-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                <link rel="icon" type="image/png" href="/static/rullst.png" />
                <title>{title}</title>
            </head>
            <body>
                <div>{body}</div>
            </body>
        </html>
    }
}
"#
    .to_string()
}

/// The `src/middlewares/nexus_access.rs` of the SaaS and LMS starters: Nexus
/// under the application's sign-in for users whose `is_admin` is true.
pub const NEXUS_ACCESS: &str = include_str!("nexus_access.rs.template");

#[cfg(test)]
#[path = "nexus_access_tests.rs"]
mod nexus_access_tests;

/// Generates the lib.rs router entry for hot-reload mode,
/// with dynamic repo_mod_decl injection.
pub fn hot_reload_lib_rs(routes_block: &str, nexus_block: &str, repo_mod_decl: &str) -> String {
    format!(
        r##"use rullst::{{routes, Router}};

pub mod migrations;
pub mod models;
{repo_mod_decl}pub mod controllers;
pub mod pages;

pub fn router() -> Router {{
    {nexus_block}

    {routes_block}
}}

#[unsafe(no_mangle)]
pub extern "C" fn rullst_router_init() -> *mut Router {{
    Box::into_raw(Box::new(router()))
}}
"##,
        repo_mod_decl = repo_mod_decl,
        nexus_block = nexus_block,
        routes_block = routes_block,
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_layout_takes_and_returns_markup() {
        let layout = super::render_page_layout("Zero-Bundle HTMX");
        assert!(layout.starts_with(
            "pub fn render_layout(title: &str, body: rullst::html::Markup) -> rullst::html::Markup {"
        ));
        assert!(layout.contains("<div>{body}</div>"));
        syn::parse_file(&layout).expect("the layout parses");
    }

    #[test]
    fn starters_name_only_the_frontend_the_wizard_offers() {
        // `cargo rullst new` has no frontend choice; no starter file may claim
        // a Tailwind, LiveView, Pico.css or Tera profile.
        let frontend = "Zero-Bundle HTMX";
        let mut files = crate::blueprints::blank::file_manifest(
            "demo",
            "demo",
            false,
            false,
            true,
            "Active Record",
            frontend,
        );
        files.extend(crate::blueprints::saas::file_manifest(
            "demo",
            false,
            "Active Record",
            frontend,
        ));
        for (path, content) in files {
            for claim in [
                "profile selected",
                "Frontend Engine: Tailwind",
                "Frontend Engine: Tera",
            ] {
                assert!(!content.contains(claim), "{path} claims `{claim}`");
            }
        }
        assert!(
            super::frontend_page_imports(frontend)
                .starts_with("// Frontend Engine: Zero-Bundle HTMX\n")
        );
        assert_eq!(super::frontend_cargo_dependency(frontend), "");
    }
}
