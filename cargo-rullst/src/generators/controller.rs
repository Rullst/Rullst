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
            r#"use rullst::{{html, server::{{Html, IntoResponse, Path, Form}}}};
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
    Html(html! {{
        <div style="font-family: system-ui, sans-serif; display: flex; flex-direction: column; align-items: center; justify-content: center; min-height: 100vh; background: #0f172a; color: #f8fafc; padding: 2rem; box-sizing: border-box;">
            <div style="max-width: 600px; text-align: center; background: #1e293b; padding: 3rem; border-radius: 1rem; border: 1px solid #334155; box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.3);">
                <h1 style="font-size: 2.5rem; margin: 0 0 1rem 0; background: linear-gradient(to right, #38bdf8, #818cf8); -webkit-background-clip: text; -webkit-text-fill-color: transparent; font-weight: 800;">
                    "{camel_name}"
                </h1>
                <p style="color: #94a3b8; font-size: 1.1rem; line-height: 1.6; margin-bottom: 2rem;">
                    "This controller was generated by the Rullst CLI and is ready for application logic."
                </p>
                <div style="display: inline-block; padding: 0.75rem 1.5rem; background: #0f172a; border-radius: 0.5rem; border: 1px solid #334155; color: #38bdf8; font-family: monospace; font-size: 0.95rem;">
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
mod tests {
    use super::*;

    #[test]
    fn controller_names_must_produce_rust_identifiers() {
        let check = |name: &str| {
            validate_controller_identifiers(name, &to_snake_case(name), &to_camel_case(name))
        };
        for valid in ["Account", "ApiAccount", "user-profile", "OrdersController"] {
            assert!(check(valid).is_ok(), "{valid} must be accepted");
        }
        for invalid in ["Bad.Name", "../../notes", "a/b", "9Lives", ""] {
            assert!(check(invalid).is_err(), "{invalid} must be rejected");
        }
    }

    #[test]
    fn api_controllers_extract_json_bodies() {
        let source = render_controller_source("ProductController", true);
        assert!(source.contains("pub async fn store(Json(_payload): Json<CreateDto>)"));
        assert!(source.contains("Json(_payload): Json<UpdateDto>"));
        assert!(!source.contains("Form"));
        syn::parse_file(&source).expect("generated API controller should parse");

        let html = render_controller_source("ProductController", false);
        assert!(html.contains("Form(_payload): Form<CreateDto>"));
    }

    #[test]
    fn api_instructions_register_every_write_route_as_a_machine_endpoint() {
        let lines = mapping_instructions("product_controller", true);
        let text = lines.join("\n");
        for route in [
            r#"get("/api/product" => product_controller::index),"#,
            r#"get("/api/product/{id}" => product_controller::show),"#,
            r#"post("/api/product" => product_controller::store),"#,
            r#"put("/api/product/{id}" => product_controller::update),"#,
            r#"delete("/api/product/{id}" => product_controller::delete),"#,
        ] {
            assert!(text.contains(route), "{route} missing from:\n{text}");
        }
        let machine: Vec<_> = lines
            .iter()
            .filter_map(|line| line.trim().strip_prefix("MachineEndpoint::bearer(Method::"))
            .collect();
        assert_eq!(
            machine,
            [
                r#"POST, "/api/product", api_token)?,"#,
                r#"PUT, "/api/product/{id}", api_token)?,"#,
                r#"DELETE, "/api/product/{id}", api_token)?,"#,
            ]
        );
        // Core accepts the printed routes.
        for (method, path) in [
            (rullst_core::http::Method::POST, "/api/product"),
            (rullst_core::http::Method::PUT, "/api/product/{id}"),
            (rullst_core::http::Method::DELETE, "/api/product/{id}"),
        ] {
            rullst_core::security::MachineEndpoint::bearer(
                method,
                path,
                "0123456789abcdef".repeat(2),
            )
            .expect("printed route is a valid machine endpoint");
        }
        assert!(text.contains(MACHINE_ENDPOINTS_DOCS));
        assert!(
            mapping_instructions("controller", true)
                .join("\n")
                .contains(r#"post("/api/controller" => controller::store),"#)
        );

        let html = mapping_instructions("product_controller", false).join("\n");
        assert!(html.contains(r#"get("/url" => product_controller::index)"#));
        assert!(!html.contains("MachineEndpoint"));
    }
}
