// src/generators/resource.rs — Full CRUD Resource generator.

use crate::generators::{
    controller::create_new_controller, is_rullst_project, is_valid_rust_identifier,
    model::create_new_model, model_to_pascal_case, model_to_snake_case, pluralize,
};
use colored::*;
use std::fs;
use std::path::Path;

pub fn create_new_resource(name: &str, api: bool) -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        return Err(crate::ui::error_report::ProjectRequired.into());
    }

    let resource_name = model_to_snake_case(name);
    let type_name = model_to_pascal_case(name);
    if !is_valid_rust_identifier(&resource_name) || !is_valid_rust_identifier(&type_name) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "resource name must produce valid non-keyword Rust identifiers",
        )
        .into());
    }
    let route_name = pluralize(&resource_name);

    println!(
        "{}",
        format!("⚡ Scaffolding Full CRUD Resource for '{}'...", type_name)
            .bright_magenta()
            .bold()
    );

    // 1. Create Model & Migration
    create_new_model(&type_name, true)?;

    // 2. Create Controller
    create_new_controller(&type_name, api)?;

    // 3. If HTML fullstack, create Views
    if !api {
        let views_dir = Path::new("views").join(&resource_name);
        if !views_dir.exists() {
            fs::create_dir_all(&views_dir)?;
        }

        let tailwind = crate::generators::frontend::uses_tailwind(Path::new("."));
        let (index_content, form_content) = views(&type_name, &route_name, tailwind);
        let index_view_path = views_dir.join("index.html");
        if !index_view_path.exists() {
            fs::write(&index_view_path, index_content)?;
        }
        let form_view_path = views_dir.join("form.html");
        if !form_view_path.exists() {
            fs::write(&form_view_path, form_content)?;
        }
    }

    println!(
        "{}",
        format!("✨ Resource '{}' scaffolded successfully!", type_name)
            .green()
            .bold()
    );
    println!(
        "{}",
        format!("👉 Model: src/models/{}.rs", resource_name).dimmed()
    );
    println!(
        "{}",
        format!(
            "👉 Controller: src/controllers/{}_controller.rs",
            resource_name
        )
        .dimmed()
    );
    if !api {
        println!(
            "{}",
            format!("👉 Views: views/{}/ (index.html, form.html)", resource_name).dimmed()
        );
    }

    Ok(())
}

/// `index.html` and `form.html` of a resource: Tailwind utility classes
/// when the project uses Tailwind, else plain markup that its own stylesheet
/// can style.
fn views(type_name: &str, route_name: &str, tailwind: bool) -> (String, String) {
    if tailwind {
        return (
            format!(
                r#"<div class="max-w-4xl mx-auto py-8">
  <div class="flex justify-between items-center mb-6">
    <h1 class="text-3xl font-bold text-slate-100">{type_name} List</h1>
    <a href="/{route_name}/create" class="px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-500 transition">Create New</a>
  </div>
  <div class="bg-slate-800 rounded-xl p-6 shadow-lg border border-slate-700">
    <p class="text-slate-400">No {route_name} found yet.</p>
  </div>
</div>
"#
            ),
            format!(
                r#"<div class="max-w-xl mx-auto py-8">
  <h1 class="text-2xl font-bold text-slate-100 mb-6">Create {type_name}</h1>
  <form method="POST" action="/{route_name}" class="space-y-4 bg-slate-800 p-6 rounded-xl border border-slate-700">
    <div>
      <label for="name" class="block text-sm font-medium text-slate-300 mb-1">Title / Name</label>
      <input type="text" id="name" name="name" required class="w-full px-3 py-2 bg-slate-900 border border-slate-700 rounded-lg text-slate-100 focus:outline-none focus:border-indigo-500" />
    </div>
    <div class="flex justify-end space-x-3 pt-4">
      <a href="/{route_name}" class="px-4 py-2 bg-slate-700 text-slate-300 rounded-lg">Cancel</a>
      <button type="submit" class="px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-500">Save</button>
    </div>
  </form>
</div>
"#
            ),
        );
    }
    (
        format!(
            r#"<main class="resource-index">
  <h1>{type_name} List</h1>
  <p><a href="/{route_name}/create">Create New</a></p>
  <p>No {route_name} found yet.</p>
</main>
"#
        ),
        format!(
            r#"<main class="resource-form">
  <h1>Create {type_name}</h1>
  <form method="POST" action="/{route_name}">
    <p>
      <label for="name">Title / Name</label>
      <input type="text" id="name" name="name" required />
    </p>
    <p>
      <a href="/{route_name}">Cancel</a>
      <button type="submit">Save</button>
    </p>
  </form>
</main>
"#
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_names_are_not_controller_names() {
        let resource_name = model_to_snake_case("BlogPostController");
        let type_name = model_to_pascal_case("BlogPostController");
        assert_eq!(resource_name, "blog_post_controller");
        assert_eq!(type_name, "BlogPostController");

        let resource_name = model_to_snake_case("BlogPost");
        assert_eq!(resource_name, "blog_post");
        assert_eq!(pluralize(&resource_name), "blog_posts");
    }

    #[test]
    fn views_use_tailwind_classes_only_in_a_tailwind_project() {
        let (index, form) = views("BlogPost", "blog_posts", false);
        assert!(index.contains("<h1>BlogPost List</h1>"));
        assert!(index.contains(r#"<a href="/blog_posts/create">Create New</a>"#));
        assert!(form.contains(r#"<form method="POST" action="/blog_posts">"#));
        assert!(form.contains(r#"<label for="name">"#) && form.contains(r#"id="name""#));
        for markup in [&index, &form] {
            assert_eq!(markup.matches("class=").count(), 1, "{markup}");
            assert!(!markup.contains("slate") && !markup.contains("rounded"));
        }
        let (index, form) = views("BlogPost", "blog_posts", true);
        assert!(index.contains("bg-indigo-600") && form.contains("bg-slate-800"));
        assert!(form.contains(r#"<label for="name" class="#) && form.contains(r#"id="name""#));
    }
}
