use colored::*;
use std::fs;
use std::path::Path;

pub fn create_new_live_component(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let struct_name = to_camel_case(name);
    let file_name = to_snake_case(name);
    let live_dir = Path::new("src/live");

    if !live_dir.exists() {
        fs::create_dir_all(live_dir)?;
    }

    let file_path = live_dir.join(format!("{}.rs", file_name));

    if file_path.exists() {
        return Err(format!("LiveComponent file '{}' already exists!", file_path.display()).into());
    }

    let mut code = String::new();
    code.push_str("use async_trait::async_trait;\n");
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
    code.push_str("        if let Some(action) = payload.get(\"action\").and_then(|v| v.as_str()) {\n");
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
        format!("✨ Created LiveComponent '{}' at {}", struct_name, file_path.display()).bold().green()
    );
    println!("  Mount in your controller using:");
    println!("   {}", format!("let html = rullst::live::Live::mount::<{struct_name}>(\"/ws/{file_name}\").await;").cyan());

    Ok(())
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
