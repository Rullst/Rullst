use colored::*;
use std::fs;
use std::path::Path;

struct ModelDef {
    name: String,
    fields: Vec<(String, String)>,
}

struct RelationDef {
    from: String,
    to: String,
    rel_type: String,
    label: String,
}

pub fn generate_mermaid_diagram(
    base_path: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "📊 Generating Mermaid Diagram...".cyan());
    let base = base_path.unwrap_or_else(|| Path::new("."));

    let src_path = base.join("src");
    if !src_path.exists() {
        return Err(
            "No src/ directory found to scan for models. Are you in a Rullst project root?".into(),
        );
    }

    let mut models = Vec::new();
    let mut relations = Vec::new();

    fn scan_dir(dir: &Path, models: &mut Vec<ModelDef>, relations: &mut Vec<RelationDef>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    scan_dir(&path, models, relations);
                } else if path.extension().unwrap_or_default() == "rs" {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if !content.contains("Orm") {
                            continue;
                        }

                        if let Ok(syntax_tree) = syn::parse_file(&content) {
                            for item in syntax_tree.items {
                                if let syn::Item::Struct(s) = item {
                                    let mut has_orm = false;
                                    for attr in &s.attrs {
                                        let attr_str = quote::quote!(#attr).to_string();
                                        if attr_str.contains("Orm") {
                                            has_orm = true;
                                            break;
                                        }
                                    }
                                    if !has_orm {
                                        continue;
                                    }

                                    let model_name = s.ident.to_string();
                                    let mut model_fields = Vec::new();

                                    if let syn::Fields::Named(named_fields) = s.fields {
                                        for f in named_fields.named {
                                            if let Some(ident) = f.ident {
                                                let field_name = ident.to_string();
                                                let ty = &f.ty;
                                                let ty_str =
                                                    quote::quote!(#ty).to_string().replace(" ", "");

                                                if ty_str.starts_with("HasMany<") {
                                                    let target = ty_str
                                                        .trim_start_matches("HasMany<")
                                                        .trim_end_matches('>')
                                                        .to_string();
                                                    relations.push(RelationDef {
                                                        from: model_name.clone(),
                                                        to: target,
                                                        rel_type: "||--o{".to_string(),
                                                        label: field_name,
                                                    });
                                                } else if ty_str.starts_with("BelongsTo<") {
                                                    let target = ty_str
                                                        .trim_start_matches("BelongsTo<")
                                                        .trim_end_matches('>')
                                                        .to_string();
                                                    relations.push(RelationDef {
                                                        from: model_name.clone(),
                                                        to: target,
                                                        rel_type: "}o--||".to_string(),
                                                        label: field_name,
                                                    });
                                                } else if ty_str.starts_with("HasOne<") {
                                                    let target = ty_str
                                                        .trim_start_matches("HasOne<")
                                                        .trim_end_matches('>')
                                                        .to_string();
                                                    relations.push(RelationDef {
                                                        from: model_name.clone(),
                                                        to: target,
                                                        rel_type: "||--o|".to_string(),
                                                        label: field_name,
                                                    });
                                                } else if ty_str.starts_with("BelongsToMany<") {
                                                    let target = ty_str
                                                        .trim_start_matches("BelongsToMany<")
                                                        .trim_end_matches('>')
                                                        .to_string();
                                                    relations.push(RelationDef {
                                                        from: model_name.clone(),
                                                        to: target,
                                                        rel_type: "}o--o{".to_string(),
                                                        label: field_name,
                                                    });
                                                } else {
                                                    model_fields.push((ty_str, field_name));
                                                }
                                            }
                                        }
                                    }
                                    models.push(ModelDef {
                                        name: model_name,
                                        fields: model_fields,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    scan_dir(&src_path, &mut models, &mut relations);

    let mut diagram = String::from("```mermaid\nerDiagram\n");

    for m in models {
        diagram.push_str(&format!("    {} {{\n", m.name));
        for (ty, name) in m.fields {
            let clean_ty = ty.replace("<", "~").replace(">", "~"); // mermaid requires ~ instead of <>
            diagram.push_str(&format!("        {} {}\n", clean_ty, name));
        }
        diagram.push_str("    }\n");
    }

    for rel in relations {
        diagram.push_str(&format!(
            "    {} {} {} : \"{}\"\n",
            rel.from, rel.rel_type, rel.to, rel.label
        ));
    }
    diagram.push_str("```\n");

    let output_path = base.join("diagram.md");
    fs::write(&output_path, diagram)?;

    println!(
        "{}",
        "  ✅ Mermaid ER Diagram generated successfully at diagram.md".green()
    );

    Ok(())
}
