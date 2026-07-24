use std::fs;
use std::path::Path;

pub fn generate_mermaid_diagram() -> Result<(), Box<dyn std::error::Error>> {
    let mut diagram = String::from("```mermaid\nerDiagram\n");

    let src_path = Path::new("src");
    if !src_path.exists() {
        return Err("No src/ directory found to scan for models. Are you in a Rullst project root?".into());
    }

    let mut structs = Vec::new();
    let mut relations = Vec::new();

    fn scan_dir(dir: &Path, structs: &mut Vec<String>, relations: &mut Vec<(String, String, String)>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    scan_dir(&path, structs, relations);
                } else if path.extension().unwrap_or_default() == "rs" {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if !content.contains("#[derive(") || !content.contains("Orm") {
                            continue;
                        }
                        
                        let mut current_struct = String::new();
                        for line in content.lines() {
                            let line = line.trim();
                            if line.starts_with("pub struct ") || line.starts_with("struct ") {
                                let parts: Vec<&str> = line.split_whitespace().collect();
                                if parts.len() >= 2 {
                                    let name = parts[parts.len() - 1].trim_matches(|c| c == '{' || c == '(' || c == ';');
                                    current_struct = name.to_string();
                                    structs.push(current_struct.clone());
                                }
                            } else if !current_struct.is_empty() && line.contains("HasMany<") {
                                if let Some(start) = line.find("HasMany<") {
                                    let rest = &line[start + 8..];
                                    if let Some(end) = rest.find('>') {
                                        let target = &rest[..end];
                                        relations.push((current_struct.clone(), target.to_string(), "||--o{".to_string()));
                                    }
                                }
                            } else if !current_struct.is_empty() && line.contains("BelongsTo<") {
                                if let Some(start) = line.find("BelongsTo<") {
                                    let rest = &line[start + 10..];
                                    if let Some(end) = rest.find('>') {
                                        let target = &rest[..end];
                                        relations.push((current_struct.clone(), target.to_string(), "}o--||".to_string()));
                                    }
                                }
                            } else if !current_struct.is_empty() && line.contains("HasOne<") {
                                if let Some(start) = line.find("HasOne<") {
                                    let rest = &line[start + 7..];
                                    if let Some(end) = rest.find('>') {
                                        let target = &rest[..end];
                                        relations.push((current_struct.clone(), target.to_string(), "||--o|".to_string()));
                                    }
                                }
                            } else if !current_struct.is_empty() && line.contains("BelongsToMany<") {
                                if let Some(start) = line.find("BelongsToMany<") {
                                    let rest = &line[start + 14..];
                                    if let Some(end) = rest.find('>') {
                                        let target = &rest[..end];
                                        relations.push((current_struct.clone(), target.to_string(), "}o--o{".to_string()));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    scan_dir(src_path, &mut structs, &mut relations);

    for s in structs {
        diagram.push_str(&format!("    {} {{\n    }}\n", s));
    }
    for (from, to, rel) in relations {
        diagram.push_str(&format!("    {} {} {} : \"\"\n", from, rel, to));
    }
    diagram.push_str("```\n");

    fs::write("diagram.md", diagram)?;
    
    Ok(())
}
