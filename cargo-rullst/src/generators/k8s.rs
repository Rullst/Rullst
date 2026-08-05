//! Kubernetes Infrastructure Generator (`cargo rullst make:k8s`)

use colored::Colorize;
use std::fs;
use std::path::Path;

use crate::blueprints::k8s::*;

/// Scaffolds Kubernetes manifest files into `k8s/` directory.
pub fn generate_k8s_manifests() -> Result<(), Box<dyn std::error::Error>> {
    let project_name = get_project_name().unwrap_or_else(|| "my-app".to_string());
    let target_dir = Path::new("k8s");

    if !target_dir.exists() {
        fs::create_dir_all(target_dir)?;
    }

    let port = 3000;

    fs::write(target_dir.join("deployment.yaml"), deployment_yaml(&project_name, port))?;
    fs::write(target_dir.join("service.yaml"), service_yaml(&project_name, port))?;
    fs::write(target_dir.join("configmap.yaml"), configmap_yaml(&project_name, port))?;
    fs::write(target_dir.join("hpa.yaml"), hpa_yaml(&project_name))?;
    fs::write(target_dir.join("ingress.yaml"), ingress_yaml(&project_name))?;
    fs::write(target_dir.join("all-in-one.yaml"), all_in_one_yaml(&project_name, port))?;

    println!("{}", "☸️  Kubernetes Manifests Scaffolded Successfully!".green().bold());
    println!("   📁 Location: {}", "k8s/".cyan());
    println!("   📄 Files created:");
    println!("      • k8s/deployment.yaml");
    println!("      • k8s/service.yaml");
    println!("      • k8s/configmap.yaml");
    println!("      • k8s/hpa.yaml");
    println!("      • k8s/ingress.yaml");
    println!("      • k8s/all-in-one.yaml");
    println!("\n   💡 Deployment Command: {}", "kubectl apply -f k8s/".bold().yellow());

    Ok(())
}

fn get_project_name() -> Option<String> {
    let content = fs::read_to_string("Cargo.toml").ok()?;
    for line in content.lines() {
        if line.starts_with("name = ") {
            return Some(line.replace("name = ", "").replace('"', "").trim().to_string());
        }
    }
    None
}
