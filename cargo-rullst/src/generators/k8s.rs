//! Kubernetes Infrastructure Generator (`cargo rullst make:k8s`)

use colored::Colorize;
use std::fs;
use std::path::Path;

use crate::blueprints::k8s::*;
use crate::generators::output_guard::{reject_existing, reject_symlink, write_new};
use crate::generators::platform_name::{dns_label, package_name};

/// Scaffolds Kubernetes manifest files into `k8s/` directory.
pub fn generate_k8s_manifests() -> Result<(), Box<dyn std::error::Error>> {
    // Object names, the image reference and the ingress host must be RFC 1123
    // labels, which Cargo names such as `my_app` are not.
    let project_name = package_name(Path::new("Cargo.toml"))
        .map_or_else(|| "my-app".to_string(), |name| dns_label(&name));
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
