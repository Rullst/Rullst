// src/generators/controller.rs — Controller generator.

use crate::generators::{
    is_rullst_project, is_valid_rust_identifier, to_camel_case, to_snake_case,
};
use colored::*;
use std::fs;
use std::path::Path;

pub fn create_new_controller(name: &str, api: bool) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Validate if we are in the root of the Rullst project
    if !is_rullst_project() {
        return Err(crate::ui::error_report::ProjectRequired.into());
    }

    let snake_name = to_snake_case(name);
    let camel_name = to_camel_case(name);
    validate_controller_identifiers(name, &snake_name, &camel_name)?;

    println!(
        "{}",
        format!("🛠️ Generating Rullst controller: {}...", camel_name)
            .cyan()
            .bold()
    );

    // 2. Ensure src/controllers directory exists
    let controllers_dir = Path::new("src/controllers");
    if !controllers_dir.exists() {
        fs::create_dir_all(controllers_dir)?;
    }

    // 3. Garantir que o src/controllers/mod.rs existe
    let mod_path = controllers_dir.join("mod.rs");
    if !mod_path.exists() {
        fs::write(&mod_path, "")?;
    }

    // 4. Register new controller in mod.rs
    let mut mod_content = fs::read_to_string(&mod_path)?;
    let mod_declaration = format!("pub mod {};", snake_name);
    if !mod_content.contains(&mod_declaration) {
        if !mod_content.is_empty() && !mod_content.ends_with('\n') {
            mod_content.push('\n');
        }
        mod_content.push_str(&mod_declaration);
        mod_content.push('\n');
        fs::write(&mod_path, mod_content)?;
    }

    // 5. Create controller file
    let controller_path = controllers_dir.join(format!("{}.rs", snake_name));
    if controller_path.exists() {
        println!(
            "{}",
            format!(
                "⚠️ Warning: Controller '{}.rs' already exists. Skipping file creation.",
                snake_name
            )
            .yellow()
        );
    } else {
        let template = render_controller_source(&camel_name, api);
        fs::write(&controller_path, template)?;
    }

    // 6. Attempt to inject "pub mod controllers;" into src/main.rs if needed
    let main_path = Path::new("src/main.rs");
    if main_path.exists() {
        let mut main_content = fs::read_to_string(main_path)?;
        if !main_content.contains("pub mod controllers;")
            && !main_content.contains("mod controllers;")
        {
            main_content = format!("pub mod controllers;\n{}", main_content);
            fs::write(main_path, main_content)?;
            println!(
                "{}",
                "ℹ️ Automatically added 'pub mod controllers;' to the top of src/main.rs.".cyan()
            );
        }
    }

    println!(
        "{}",
        format!(
            "✨ Controller '{}' successfully created at '{}'!",
            camel_name,
            controller_path.display()
        )
        .green()
        .bold()
    );
    println!("{}", "How to map in your routes:".cyan());
    for line in mapping_instructions(&snake_name, api) {
        println!("{}", line.cyan());
    }

    Ok(())
}

const MACHINE_ENDPOINTS_DOCS: &str = "https://rullst.github.io/Rullst/book/security-layers.html#what-generated-applications-already-have";

/// The routes to add after `make:controller`. JSON write routes also need a
/// machine endpoint: in staging and production the browser CSRF check
/// refuses them otherwise.
fn mapping_instructions(snake_name: &str, api: bool) -> Vec<String> {
    let mut lines = vec![format!("  1. Use: 'use crate::controllers::{snake_name};'")];
    if !api {
        lines.push(format!(
            "  2. Add: 'get(\"/url\" => {snake_name}::index)' inside your routes! macro."
        ));
        return lines;
    }
    let resource = snake_name
        .strip_suffix("_controller")
        .filter(|resource| !resource.is_empty())
        .unwrap_or(snake_name);
    let collection = format!("/api/{resource}");
    let member = format!("{collection}/{{id}}");
    lines.push("  2. Add inside your routes! macro:".to_string());
    for (method, path, action) in [
        ("get", &collection, "index"),
        ("get", &member, "show"),
        ("post", &collection, "store"),
        ("put", &member, "update"),
        ("delete", &member, "delete"),
    ] {
        lines.push(format!(
            "       {method}(\"{path}\" => {snake_name}::{action}),"
        ));
    }
    lines.push("  3. In staging and production, JSON clients need each write route in".to_string());
    lines.push(
        "     machine_endpoints() in src/main.rs, as in the Blank JSON API starter:".to_string(),
    );
    for (method, path) in [("POST", &collection), ("PUT", &member), ("DELETE", &member)] {
        lines.push(format!(
            "       MachineEndpoint::bearer(Method::{method}, \"{path}\", api_token)?,"
        ));
    }
    lines.push(format!("     See {MACHINE_ENDPOINTS_DOCS}"));
    lines
}

/// Renders the controller module emitted by `make:controller`.
///
/// The `--api` variant extracts and returns `Json<T>`. Public so scaffold
/// smoke tests can compile the exact generated source.
#[doc(hidden)]
pub fn render_controller_source(camel_name: &str, api: bool) -> String {
    if api {
        format!(
            r#"//! JSON controller. In staging and production, JSON clients reach its write
//! routes only through `machine_endpoints()` in `src/main.rs` (bearer API
//! token); otherwise the browser CSRF check refuses them.

use rullst::server::{{IntoResponse, Json, Path}};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateDto {{
    // Add fields for creation
}}

#[derive(Deserialize)]
pub struct UpdateDto {{
    // Add fields for update
}}

/// Lists the resources
pub async fn index() -> impl IntoResponse {{
    Json(serde_json::json!({{
        "controller": "{camel_name}",
        "action": "index",
        "message": "This controller was generated by the Rullst CLI and is ready for application logic."
    }}))
}}

/// Shows one resource
pub async fn show(Path(id): Path<i32>) -> impl IntoResponse {{
    Json(serde_json::json!({{
        "controller": "{camel_name}",
        "action": "show",
        "id": id
    }}))
}}

/// Creates a new resource
pub async fn store(Json(_payload): Json<CreateDto>) -> impl IntoResponse {{
    Json(serde_json::json!({{
        "message": "Resource created successfully"
    }}))
}}

/// Updates an existing resource
pub async fn update(Path(id): Path<i32>, Json(_payload): Json<UpdateDto>) -> impl IntoResponse {{
    Json(serde_json::json!({{
        "id": id,
        "message": "Resource updated successfully"
    }}))
}}

/// Deletes a resource
pub async fn delete(Path(id): Path<i32>) -> impl IntoResponse {{
    Json(serde_json::json!({{
        "id": id,
        "message": "Resource deleted successfully"
    }}))
}}
"#
        )
    } else {
        format!(
            r#"use rullst::security::CspNonce;
use rullst::{{html, server::{{Extension, Form, Html, IntoResponse, Path}}}};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateDto {{
    // Add fields for creation
}}

#[derive(Deserialize)]
pub struct UpdateDto {{
    // Add fields for update
}}

/// The page's style sheet. The default Content-Security-Policy
/// (`style-src 'self' 'nonce-…'`) blocks `style` attributes, so the styles
/// live in a `<style>` element carrying the request's nonce.
const STYLES: &str = "
.page {{ font-family: system-ui, sans-serif; display: flex; flex-direction: column; align-items: center; justify-content: center; min-height: 100vh; background: #0f172a; color: #f8fafc; padding: 2rem; box-sizing: border-box; }}
.card {{ max-width: 600px; text-align: center; background: #1e293b; padding: 3rem; border-radius: 1rem; border: 1px solid #334155; box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.3); }}
.title {{ font-size: 2.5rem; margin: 0 0 1rem 0; background: linear-gradient(to right, #38bdf8, #818cf8); -webkit-background-clip: text; background-clip: text; -webkit-text-fill-color: transparent; font-weight: 800; }}
.lede {{ color: #94a3b8; font-size: 1.1rem; line-height: 1.6; margin-bottom: 2rem; }}
.signature {{ display: inline-block; padding: 0.75rem 1.5rem; background: #0f172a; border-radius: 0.5rem; border: 1px solid #334155; color: #38bdf8; font-family: monospace; font-size: 0.95rem; }}
";

/// Lists the resources
pub async fn index(csp_nonce: Option<Extension<CspNonce>>) -> impl IntoResponse {{
    let nonce = csp_nonce
        .as_ref()
        .map(|Extension(nonce)| nonce.as_str())
        .unwrap_or_default();
    Html(html! {{
        <style nonce={{nonce}}>{{STYLES}}</style>
        <div class="page">
            <div class="card">
                <h1 class="title">
                    "{camel_name}"
                </h1>
                <p class="lede">
                    "This controller was generated by the Rullst CLI and is ready for application logic."
                </p>
                <div class="signature">
                    "pub async fn index() -> impl IntoResponse"
                </div>
            </div>
        </div>
    }})
}}

/// Shows one resource
pub async fn show(Path(id): Path<i32>) -> impl IntoResponse {{
    Html(html! {{ 
        <div>"Resource "{{id}}</div> 
    }})
}}

/// Creates a new resource
pub async fn store(Form(_payload): Form<CreateDto>) -> impl IntoResponse {{
    Html(html! {{ <div>"Resource created successfully"</div> }})
}}

/// Updates an existing resource
pub async fn update(Path(id): Path<i32>, Form(_payload): Form<UpdateDto>) -> impl IntoResponse {{
    Html(html! {{ <div>"Resource "{{id}}" updated successfully"</div> }})
}}

/// Deletes a resource
pub async fn delete(Path(id): Path<i32>) -> impl IntoResponse {{
    Html(html! {{ <div>"Resource "{{id}}" deleted successfully"</div> }})
}}
"#
        )
    }
}

/// Rejects a name whose module or type would not be a non-keyword Rust
/// identifier (for example `Bad.Name` → `pub mod bad.name_controller;`)
/// before `src/controllers/mod.rs` is edited.
fn validate_controller_identifiers(
    name: &str,
    snake_name: &str,
    camel_name: &str,
) -> std::io::Result<()> {
    if !name.trim().is_empty()
        && is_valid_rust_identifier(snake_name)
        && is_valid_rust_identifier(camel_name)
    {
        return Ok(());
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        format!(
            "controller name must produce valid non-keyword Rust identifiers (module `{snake_name}`, type `{camel_name}`); use letters, digits, `_` or `-`, for example Account"
        ),
    ))
}

#[cfg(test)]
#[path = "controller_tests.rs"]
mod tests;
