use crate::generators::{is_rullst_project, is_valid_rust_identifier};
use colored::*;
use std::fs;
use std::io::{Error as IoError, ErrorKind};
use std::path::Path;

pub fn create_new_live_component(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        return Err(IoError::new(
            ErrorKind::InvalidInput,
            "make:live must run in the root of a Rullst project (a Cargo.toml with a rullst dependency)",
        )
        .into());
    }
    let (struct_name, file_name) = live_component_names(name)?;
    let live_dir = Path::new("src/live");

    if !live_dir.exists() {
        fs::create_dir_all(live_dir)?;
    }

    let file_path = live_dir.join(format!("{}.rs", file_name));

    if file_path.exists() {
        return Err(format!(
            "LiveComponent file '{}' already exists!",
            file_path.display()
        )
        .into());
    }

    let code = render_live_component_source(&struct_name, &file_name);

    fs::write(&file_path, code)?;

    // Register module in src/live/mod.rs or src/live.rs
    let mod_path = live_dir.join("mod.rs");
    let mod_entry = format!("pub mod {};\n", file_name);
    if mod_path.exists() {
        let mut content = fs::read_to_string(&mod_path)?;
        if !content.contains(&mod_entry) {
            content.push_str(&mod_entry);
            fs::write(&mod_path, content)?;
        }
    } else {
        fs::write(&mod_path, format!("pub mod {};\n", file_name))?;
    }

    println!(
        "{}",
        format!(
            "✨ Created LiveComponent '{}' at {}",
            struct_name,
            file_path.display()
        )
        .bold()
        .green()
    );
    println!("  Mount in your controller using:");
    println!(
        "   {}",
        format!(
            "let html = rullst::live::Live::mount::<{struct_name}>(\"/ws/{file_name}\").await;"
        )
        .cyan()
    );

    Ok(())
}

/// Returns the component's type and module names, rejecting a name whose
/// module or type would not be a non-keyword Rust identifier (for example
/// `../notes`, `self` or `Bad.Name`) before anything is written.
fn live_component_names(name: &str) -> Result<(String, String), IoError> {
    let struct_name = to_camel_case(name);
    let file_name = to_snake_case(name);
    if is_valid_rust_identifier(&file_name) && is_valid_rust_identifier(&struct_name) {
        return Ok((struct_name, file_name));
    }
    Err(IoError::new(
        ErrorKind::InvalidInput,
        format!(
            "LiveComponent name must produce valid non-keyword Rust identifiers (module `{file_name}`, type `{struct_name}`); use letters, digits, `_` or `-`, for example LiveCounter"
        ),
    ))
}

/// Renders the LiveComponent module emitted by `make:live`.
///
/// The macro comes from the `rullst` facade: generated projects do not depend
/// on the `async-trait` crate directly. Public so scaffold smoke tests can
/// compile the exact generated source.
#[doc(hidden)]
pub fn render_live_component_source(struct_name: &str, file_name: &str) -> String {
    let mut code = String::new();
    code.push_str("use rullst::async_trait;\n");
    code.push_str("use rullst::live::LiveComponent;\n");
    code.push_str("use serde_json::Value;\n\n");
    code.push_str("#[derive(Default)]\n");
    code.push_str(&format!("pub struct {} {{\n", struct_name));
    code.push_str("    pub count: i32,\n");
    code.push_str("}\n\n");
    code.push_str("#[async_trait]\n");
    code.push_str(&format!("impl LiveComponent for {} {{\n", struct_name));
    code.push_str("    async fn mount(&mut self) {\n");
    code.push_str("        self.count = 0;\n");
    code.push_str("    }\n\n");
    code.push_str("    async fn handle_event(&mut self, payload: Value) {\n");
    code.push_str(
        "        if let Some(action) = payload.get(\"action\").and_then(|v| v.as_str()) {\n",
    );
    code.push_str("            match action {\n");
    code.push_str("                \"increment\" => self.count += 1,\n");
    code.push_str("                \"decrement\" => self.count -= 1,\n");
    code.push_str("                _ => {}\n");
    code.push_str("            }\n");
    code.push_str("        }\n");
    code.push_str("    }\n\n");
    code.push_str("    fn render(&self) -> String {\n");
    code.push_str(&format!(
        "        format!(\n            \"<div id=\\\"{}-component\\\" class=\\\"p-6 bg-slate-800 text-white rounded-xl shadow-lg border border-slate-700\\\">\\n  <h2 class=\\\"text-xl font-bold mb-4\\\">{} (LiveView Component)</h2>\\n  <p class=\\\"text-3xl font-mono mb-6\\\">Count: {{}}</p>\\n  <div class=\\\"flex gap-4\\\">\\n    <button ws-send name=\\\"action\\\" value=\\\"increment\\\" class=\\\"px-4 py-2 bg-emerald-600 hover:bg-emerald-500 rounded font-semibold transition\\\">+ Increment</button>\\n    <button ws-send name=\\\"action\\\" value=\\\"decrement\\\" class=\\\"px-4 py-2 bg-rose-600 hover:bg-rose-500 rounded font-semibold transition\\\">- Decrement</button>\\n  </div>\\n</div>\",\n            self.count\n        )\n",
        file_name, struct_name
    ));
    code.push_str("    }\n");
    code.push_str("}\n");

    code
}

fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for c in s.chars() {
        if c == '_' || c == '-' || c == ' ' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }

    result
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else if c == '-' || c == ' ' {
            result.push('_');
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_component_names_must_be_rust_identifiers() {
        assert_eq!(
            live_component_names("LiveCounter").unwrap(),
            ("LiveCounter".to_string(), "live_counter".to_string())
        );
        assert_eq!(
            live_component_names("chat-room").unwrap(),
            ("ChatRoom".to_string(), "chat_room".to_string())
        );
        for invalid in [
            "../../notes",
            "self",
            "Bad.Name",
            "9lives",
            "",
            "a/b",
            "match",
        ] {
            assert!(
                live_component_names(invalid).is_err(),
                "{invalid} must be rejected"
            );
        }
    }

    #[test]
    fn live_component_imports_async_trait_through_the_rullst_facade() {
        let source = render_live_component_source("LiveCounter", "live_counter");
        assert!(source.contains("use rullst::async_trait;"));
        assert!(!source.contains("use async_trait::"));
        syn::parse_file(&source).expect("generated LiveComponent should parse");
    }
}
