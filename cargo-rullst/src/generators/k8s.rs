//! Kubernetes Infrastructure Generator (`cargo rullst make:k8s`)

use colored::Colorize;
use std::fs;
use std::path::Path;

use crate::blueprints::k8s::*;
use crate::generators::output_guard::{reject_existing, reject_symlink, write_new};

/// Scaffolds Kubernetes manifest files into `k8s/` directory.
pub fn generate_k8s_manifests() -> Result<(), Box<dyn std::error::Error>> {
    let project_name = get_project_name().unwrap_or_else(|| "my-app".to_string());
    let target_dir = Path::new("k8s");
    let port = 3000;
    let manifests = [
        ("deployment.yaml", deployment_yaml(&project_name, port)),
        ("service.yaml", service_yaml(&project_name, port)),
        ("configmap.yaml", configmap_yaml(&project_name, port)),
        ("hpa.yaml", hpa_yaml(&project_name)),
        ("ingress.yaml", ingress_yaml(&project_name)),
        ("all-in-one.yaml", all_in_one_yaml(&project_name, port)),
    ];
    let paths = manifests
        .iter()
        .map(|(name, _)| target_dir.join(name))
        .collect::<Vec<_>>();
    // Customized manifests are application files: refuse before writing any.
    reject_symlink(target_dir)?;
    reject_existing(
        "Kubernetes manifests",
        &paths,
        "; move them aside to regenerate the templates",
    )?;
    fs::create_dir_all(target_dir)?;
    for (path, (_, contents)) in paths.iter().zip(&manifests) {
        write_new(path, contents.as_bytes())?;
    }

    println!(
        "{}",
        "☸️  Kubernetes Manifests Scaffolded Successfully!"
            .green()
            .bold()
    );
    println!("   📁 Location: {}", "k8s/".cyan());
    println!("   📄 Files created:");
    println!("      • k8s/deployment.yaml");
    println!("      • k8s/service.yaml");
    println!("      • k8s/configmap.yaml");
    println!("      • k8s/hpa.yaml");
    println!("      • k8s/ingress.yaml");
    println!("      • k8s/all-in-one.yaml");
    println!(
        "\n   💡 Deployment Command: {}",
        "kubectl apply -f k8s/".bold().yellow()
    );

    Ok(())
}

fn get_project_name() -> Option<String> {
    let content = fs::read_to_string("Cargo.toml").ok()?;
    for line in content.lines() {
        if line.starts_with("name = ") {
            return Some(
                line.replace("name = ", "")
                    .replace('"', "")
                    .trim()
                    .to_string(),
            );
        }
    }
    None
}
