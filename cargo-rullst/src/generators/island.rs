use crate::generators::{is_rullst_project, to_camel_case, to_snake_case};
use colored::*;
use std::fs;
use std::path::Path;

pub fn create_new_island(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        println!(
            "{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold()
        );
        std::process::exit(1);
    }

    let snake_name = to_snake_case(name);
    let camel_name = to_camel_case(name);

    println!(
        "{}",
        format!("🏝️ Generating Wasm Island: {}...", camel_name)
            .cyan()
            .bold()
    );

    let islands_dir = Path::new("src/islands");
    if !islands_dir.exists() {
        fs::create_dir_all(islands_dir)?;
    }

    let mod_path = islands_dir.join("mod.rs");
    if !mod_path.exists() {
        fs::write(&mod_path, "")?;
    }

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

    let island_path = islands_dir.join(format!("{}.rs", snake_name));
    if island_path.exists() {
        println!(
            "{}",
            format!(
                "⚠️ Warning: Island '{}.rs' already exists. Skipping file creation.",
                snake_name
            )
            .yellow()
        );
    } else {
        let template = format!(
            r#"use rullst::{{island, view}};
use wasm_bindgen::prelude::*;
use serde::{{Serialize, Deserialize}};
use web_sys::HtmlElement;

#[derive(Serialize, Deserialize, Clone)]
pub struct {camel_name}Props {{
    pub initial_value: i32,
}}

#[island]
pub fn {snake_name}(props: {camel_name}Props, container: HtmlElement) {{
    // Wasm logic goes here!
    // You can attach event listeners to `container` or render with leptos/yew/sycamore
    
    let doc = web_sys::window().unwrap().document().unwrap();
    let btn = doc.create_element("button").unwrap();
    btn.set_inner_html(&format!("Click me! Initial: {{}}", props.initial_value));
    
    let closure = Closure::wrap(Box::new(move || {{
        web_sys::console::log_1(&"Island clicked!".into());
    }}) as Box<dyn FnMut()>);
    
    btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref()).unwrap();
    closure.forget();
    
    container.append_child(&btn).unwrap();
}}
"#
        );

        fs::write(&island_path, template)?;
        println!(
            "{}",
            format!("✅ Island '{}' created successfully at: {:?}", camel_name, island_path).green()
        );
    }

    Ok(())
}
