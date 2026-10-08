use super::super::files::{manifest_findings, project_findings};
use super::*;

fn plan(path: PathBuf, original: &str) -> ManifestUpgradePlan {
    ManifestUpgradePlan {
        path,
        original: original.to_string(),
        updated: original.to_string(),
        is_package: true,
        matched: 1,
        source_majors: BTreeSet::from([12]),
        changes: Vec::new(),
        warnings: Vec::new(),
    }
}

struct Project {
    root: PathBuf,
}

impl Project {
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("rullst-rules-files-{}", rand::random::<u64>()));
        std::fs::create_dir_all(&root).unwrap();
        Self {
            root: root.canonicalize().unwrap(),
        }
    }

    fn write(&self, relative: &str, text: &str) -> &Self {
        let path = self.root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
        self
    }

    fn findings(&self, manifest: &str) -> Vec<(String, &'static str, usize)> {
        let plans = vec![plan(self.root.join("Cargo.toml"), manifest)];
        project_findings(&self.root, &plans)
            .into_iter()
            .map(|finding| {
                let relative = finding.path.strip_prefix(&self.root).unwrap();
                (
                    relative.to_string_lossy().replace('\\', "/"),
                    finding.rule.code,
                    finding.line,
                )
            })
            .collect()
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn manifests_report_removed_and_default_feature_dependencies() {
    let manifest = r#"[package]
name = "app"

[dependencies]
rullst = { version = "12.1.2", default-features = false }
rullst-orm = { version = "12.1.2", features = ["strict-sqlite"] }
utoipa = "5.3"

[dev-dependencies.runner]
package = "rullst-labs-runner"
version = "12"
"#;
    let (findings, facts) = manifest_findings(&[plan(PathBuf::from("Cargo.toml"), manifest)]);
    let found: Vec<_> = findings
        .iter()
        .map(|finding| (finding.rule.code, finding.line))
        .collect();
    assert_eq!(
        found,
        vec![
            ("V13-ORM-DEFAULT-FEATURES", 6),
            ("V13-LABS-RUNNER-REMOVED", 9)
        ]
    );
    assert!(facts.utoipa_below_6);

    let current = r#"[dependencies]
rullst-orm = { version = "13.0.0-alpha.1", default-features = false, features = ["strict-sqlite"] }
utoipa = { version = "6", features = ["axum_extras"] }
utoipa-axum = "0.3"
[workspace.dependencies]
inherited = { workspace = true, package = "rullst-orm" }
"#;
    let (findings, facts) = manifest_findings(&[plan(PathBuf::from("Cargo.toml"), current)]);
    assert!(findings.is_empty());
    assert!(!facts.utoipa_below_6);
    let (_, facts) = manifest_findings(&[plan(
        PathBuf::from("Cargo.toml"),
        "[dependencies]\nutoipa-axum = \"0.2\"\n",
    )]);
    assert!(facts.utoipa_below_6);
}

#[test]
fn generated_project_files_are_reviewed() {
    let project = Project::new();
    project
        .write(".gitignore", "/target\n*.sqlite\n")
        .write(".cargo/config.toml", "[target.x]\nrustflags = [\"-C\", \"link-arg=-fuse-ld=mold\"]\n")
        .write(".dockerignore", "target\n.env\n")
        .write("Foundry.toml", "[app]\nport = 3000\n")
        .write("docker-compose.prod.yml", "services:\n  app: {}\n")
        .write(
            "omni-app/src/lib.rs",
            "fn run() {\n    if Path::new(\"../Cargo.toml\").exists() {}\n    tauri::Builder::default();\n}\n",
        )
        .write("rullst-client.ts", "export class RullstClient {}\n")
        .write(".env.example", "APP_KEY=\nOTEL_EXPORTER_OTLP_ENDPOINT=http://collector:4318\n")
        .write(
            "k8s/ingress.yaml",
            "apiVersion: networking.k8s.io/v1\nkind: Ingress\nmetadata:\n  annotations:\n    kubernetes.io/ingress.class: nginx\n",
        )
        .write(".github/workflows/release.yml", "steps:\n  - run: cargo rullst omni android --release\n")
        .write("build.sh", "#!/bin/sh\necho build\n");
    let found = project.findings("[package]\nname = \"my_app\"\n");
    let expected = [
        (".cargo/config.toml", "V13-LINKER-CONFIG", 2),
        (".dockerignore", "V13-DOCKER-CONTEXT", 1),
        (".env.example", "V13-OTLP-ENVIRONMENT", 2),
        (
            ".github/workflows/release.yml",
            "V13-ANDROID-RELEASE-SIGNING",
            2,
        ),
        (".gitignore", "V13-GITIGNORE-DATABASES", 1),
        ("Cargo.toml", "V13-K8S-NAMES", 2),
        ("Foundry.toml", "V13-FOUNDRY-SERVICE", 1),
        ("docker-compose.prod.yml", "V13-VPS-DEPLOY-PROXY", 1),
        ("k8s/ingress.yaml", "V13-K8S-INGRESS-TLS", 5),
        ("omni-app/src/lib.rs", "V13-OMNI-BACKEND", 2),
        ("omni-app/src/lib.rs", "V13-OMNI-RUNNER", 3),
        ("rullst-client.ts", "V13-TS-CLIENT-REGENERATE", 1),
    ];
    let mut found = found;
    found.sort();
    let expected: Vec<_> = expected
        .iter()
        .map(|(path, code, line)| (path.to_string(), *code, *line))
        .collect();
    assert_eq!(found, expected);
}

#[test]
fn current_project_files_produce_no_findings() {
    let project = Project::new();
    project
        .write(
            ".gitignore",
            "/target\n.cargo/config.toml\n*.sqlite\n*.sqlite-shm\n*.sqlite-wal\n",
        )
        .write(".cargo/config.toml", "rustflags = [\"-C\", \"link-arg=-fuse-ld=mold\"]\n")
        .write(".dockerignore", ".env\n.env.*\n!.env.example\n*.sqlite\n")
        .write("docker-compose.prod.yml", "networks:\n  edge:\n    ipv4_address: 172.31.250.10\n")
        .write(
            "omni-app/src/lib.rs",
            "fn run() {\n    println!(\"Launching Omni interface...\");\n    tauri::Builder::default();\n}\n",
        )
        .write("rullst-client.ts", "headers['X-CSRF-Token'] = token;\n")
        .write("k8s/deployment.yaml", "apiVersion: apps/v1\nkind: Deployment\n")
        .write(".github/workflows/ci.yml", "steps:\n  - run: cargo rullst omni android\n");
    assert_eq!(project.findings("[package]\nname = \"my-app\"\n"), vec![]);
}

/// The build files only the 12.0 CLI generated, and their 12.1 replacements.
#[test]
fn build_files_from_the_12_0_cli_are_reviewed() {
    let project = Project::new();
    project
        .write(
            ".cargo/config.toml",
            "# Rullst linker configuration\n\n[target.x86_64-pc-windows-msvc]\nrustflags = [\"-C\", \"link-arg=/DEBUG:FASTLINK\"]\n",
        )
        .write(".gitignore", "# Rust build artifacts\n/target\n/Cargo.lock\n")
        .write(
            "Dockerfile",
            "FROM rust:1.96 AS builder\n# cargo build --release\nRUN cargo build --release\n",
        );
    assert_eq!(
        project.findings("[package]\nname = \"app\"\n"),
        vec![
            (".cargo/config.toml".to_string(), "V13-MSVC-FASTLINK", 4),
            (".gitignore".to_string(), "V13-CARGO-LOCK-IGNORED", 3),
            ("Dockerfile".to_string(), "V13-DOCKER-UNLOCKED-BUILD", 3),
        ]
    );

    let current = Project::new();
    current
        .write(
            ".cargo/config.toml",
            "# /DEBUG:FASTLINK is not supported\n[target.x86_64-unknown-linux-gnu]\nrustflags = [\"-C\", \"split-debuginfo=unpacked\"]\n",
        )
        .write(
            ".gitignore",
            "/target\n# Commit Cargo.lock for reproducible application/deployment builds.\n!Cargo.lock\n",
        )
        .write(
            "Dockerfile",
            "# Generate and commit Cargo.lock before building this application image.\nRUN cargo build --release --locked\nRUN cargo build --frozen\n",
        );
    assert_eq!(current.findings("[package]\nname = \"app\"\n"), vec![]);
}

#[cfg(unix)]
#[test]
fn symlinked_project_files_are_not_read() {
    let project = Project::new();
    let outside = project.root.with_extension("outside");
    std::fs::write(&outside, "target\n").unwrap();
    std::os::unix::fs::symlink(&outside, project.root.join(".gitignore")).unwrap();
    std::os::unix::fs::symlink(&outside, project.root.join("Foundry.toml")).unwrap();
    assert!(project.findings("[package]\nname = \"app\"\n").is_empty());
    std::fs::remove_file(outside).unwrap();
}

/// A removed `MAIL_DRIVER`, or a setting only removed transports read, in
/// `.env`, `.env.example` or `Rullst.toml` points to its row.
#[test]
fn removed_mail_settings_are_reviewed() {
    let project = Project::new();
    project
        .write(".env", "APP_KEY=x\n# MAIL_DRIVER=sendgrid\nMAIL_DRIVER=\"SendGrid\"\n")
        .write(".env.example", "MAIL_FROM=\nPOSTMARK_SERVER_TOKEN=\n")
        .write(
            "Rullst.toml",
            "[app]\ndriver = \"postmark\"\n[mail]\nfrom = \"ops@example.com\"\ndriver = \"azure-acs\" # ACS\n",
        );
    let mut found = project.findings("[package]\nname = \"app\"\n");
    found.sort();
    let expected: Vec<_> = [(".env", 3), (".env.example", 2), ("Rullst.toml", 5)]
        .iter()
        .map(|(path, line)| (path.to_string(), "V13-MAIL-REMOVED", *line))
        .collect();
    assert_eq!(found, expected);

    let current = Project::new();
    current
        .write(".env", "MAIL_DRIVER=resend\nRESEND_API_KEY=re_x\n")
        .write(".env.example", "# MAIL_DRIVER=resend\nMAIL_FROM=\n")
        .write(
            "Rullst.toml",
            "[mail]\ndriver = \"ses\"\n[queue]\ndriver = \"mailtrap\"\n",
        );
    assert_eq!(current.findings("[package]\nname = \"app\"\n"), vec![]);
}
