import os
import re

BLUEPRINTS_DIR = r"c:\Users\venelouis\Desktop\REPOS\Rullst\cargo-rullst\src\blueprints"
FILES = ["portfolio.rs", "lms.rs", "saas.rs", "blog.rs", "erp.rs", "uptime.rs"]

for filename in FILES:
    filepath = os.path.join(BLUEPRINTS_DIR, filename)
    with open(filepath, "r", encoding="utf-8") as f:
        content = f.read()

    # Change signature
    content = content.replace("pub fn file_manifest(_project_name_safe: &str) -> Vec<(&'static str, String)> {", "pub fn file_manifest(project_name_safe: &str, hot_reload: bool) -> Vec<(&'static str, String)> {")
    content = content.replace("pub fn file_manifest(project_name_safe: &str) -> Vec<(&'static str, String)> {", "pub fn file_manifest(project_name_safe: &str, hot_reload: bool) -> Vec<(&'static str, String)> {")

    # Extract the main_rs template
    main_rs_match = re.search(r'let main_rs = r##"(.*?)"##\s*\.to_string\(\);\s*manifest\.push\(\("src/main\.rs", main_rs\)\);', content, re.DOTALL)
    if not main_rs_match:
        print(f"Skipping {filename}: main_rs not found")
        continue

    original_main_rs = main_rs_match.group(1)

    # Extract modules
    modules = "\n".join(re.findall(r"^pub mod .*?;", original_main_rs, re.MULTILINE))
    
    # Extract routes
    routes_match = re.search(r"let router = routes!\[(.*?)\];", original_main_rs, re.DOTALL)
    routes = routes_match.group(1) if routes_match else ""

    # Extract artisan
    artisan_match = re.search(r"^\s*rullst::artisan!.*?;", original_main_rs, re.MULTILINE)
    artisan_call = artisan_match.group(0) if artisan_match else ""

    # Extract print
    print_match = re.search(r'^\s*println!\(.*?;', original_main_rs, re.MULTILINE)
    print_call = print_match.group(0) if print_match else ""

    # Build the hot reload code
    lib_rs_code = f"""use rullst::{{routes, Router}};

{modules}

#[unsafe(no_mangle)]
pub extern "C" fn rullst_router_init() -> *mut Router {{
    let router = routes![{routes}];
    Box::into_raw(Box::new(router))
}}
"""

    main_rs_hot_code = f"""{modules}

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
{artisan_call}
{print_call}
    let is_hot = std::env::var("HOT_RELOAD").is_ok();

    let server = if is_hot {{
        let lib_path = if cfg!(target_os = "windows") {{
            format!("target/debug/{{}}", "{'{project_name_safe}'}")
        }} else {{
            format!("target/debug/lib{{}}", "{'{project_name_safe}'}")
        }};
        rullst::Server::new_hot(&lib_path)
    }} else {{
        let router_ptr = {'{project_name_safe}'}::rullst_router_init();
        let router = unsafe {{ *Box::from_raw(router_ptr) }};
        rullst::Server::new(router)
    }};

    server.run(3000).await?;

    Ok(())
}}
"""

    replacement = f"""    if hot_reload {{
        let lib_rs = r##"{lib_rs_code}"##.to_string();
        manifest.push(("src/lib.rs", lib_rs));

        let main_rs = format!(r##"{main_rs_hot_code}"##, project_name_safe = project_name_safe);
        manifest.push(("src/main.rs", main_rs));
    }} else {{
        let main_rs = r##"{original_main_rs}"##.to_string();
        manifest.push(("src/main.rs", main_rs));
    }}"""

    new_content = content.replace(main_rs_match.group(0), replacement)

    with open(filepath, "w", encoding="utf-8") as f:
        f.write(new_content)

    print(f"Updated {filename}")
