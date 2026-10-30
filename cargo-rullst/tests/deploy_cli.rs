//! `deploy` writes one Docker/Podman path for a Linux VM and refuses the
//! removed platform targets before writing anything.
#![allow(clippy::expect_used, clippy::panic)]

use std::{
    fs,
    process::{Command, Output},
};

struct Fixture {
    root: tempfile::TempDir,
}

impl Fixture {
    fn new(package: &str) -> Self {
        let root = tempfile::tempdir().expect("fixture directory");
        fs::write(
            root.path().join("Cargo.toml"),
            format!(
                "[package]\nname = \"{package}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst = \"13\"\n"
            ),
        )
        .expect("fixture manifest");
        Self { root }
    }

    fn run(&self, arguments: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_rullst"))
            .current_dir(self.root.path())
            .args(arguments)
            .env("RULLST_DISABLE_UPDATE_CHECK", "1")
            .env("NO_COLOR", "1")
            .output()
            .unwrap_or_else(|error| panic!("run {arguments:?}: {error}"))
    }

    fn read(&self, relative: &str) -> String {
        fs::read_to_string(self.root.path().join(relative))
            .unwrap_or_else(|error| panic!("read {relative}: {error}"))
    }

    fn exists(&self, relative: &str) -> bool {
        self.root.path().join(relative).exists()
    }
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn removed_and_unknown_platforms_fail_before_writing_anything() {
    let fixture = Fixture::new("deploy-fixture");
    for platform in ["fly", "railway", "render"] {
        let rejected = fixture.run(&["deploy", "--platform", platform]);
        assert!(!rejected.status.success(), "{}", text(&rejected));
        assert!(
            text(&rejected).contains("was removed in 13.0.0-alpha.4"),
            "{}",
            text(&rejected)
        );
    }
    let unknown = fixture.run(&["deploy", "--platform", "flyio"]);
    assert!(!unknown.status.success());
    assert!(text(&unknown).contains("Unknown platform 'flyio'"));
    for generated in [
        "Dockerfile",
        ".dockerignore",
        "compose.yaml",
        "Caddyfile",
        "Rullst.toml",
        "fly.toml",
    ] {
        assert!(!fixture.exists(generated), "{generated} was written");
    }
}

#[test]
fn deploy_writes_the_vm_files_and_trusts_its_caddy_proxy() {
    let fixture = Fixture::new("deploy-fixture");
    fs::write(
        fixture.root.path().join("Rullst.toml"),
        "[database]\nurl = \"sqlite://db.sqlite\"\n",
    )
    .expect("fixture configuration");

    // Without a trusted proxy every request reaches the app from Caddy, so
    // all clients share the starters' per-client login budget.
    let deployed = fixture.run(&["deploy"]);
    assert!(deployed.status.success(), "{}", text(&deployed));
    let config = fixture.read("Rullst.toml");
    assert!(config.starts_with("[database]\nurl = \"sqlite://db.sqlite\"\n"));
    assert!(
        config.contains("trusted_proxies = [\"172.31.250.10\"]\n"),
        "{config}"
    );
    let compose = fixture.read("compose.yaml");
    assert!(compose.contains("ipv4_address: 172.31.250.10\n"));
    assert!(compose.contains("read_only: true\n"));
    assert!(fixture.read("Caddyfile").contains("health_uri /health"));
    let dockerfile = fixture.read("Dockerfile");
    assert!(dockerfile.contains("AS planner"));
    assert!(dockerfile.contains("COPY --chown=10001:10001 Rullst.toml /app/Rullst.toml\n"));
    // The relative SQLite URL would point into the read-only /app.
    assert!(dockerfile.contains("ENV DATABASE_URL=sqlite:///app/data/rullst.db?mode=rwc\n"));
    let output = text(&deployed);
    assert!(output.contains("podman compose up -d --build"), "{output}");
    assert!(output.contains("/advanced-deployment.html"), "{output}");

    // A second run (with the former `--platform vps`) keeps every file.
    fs::write(
        fixture.root.path().join("docker-compose.prod.yml"),
        "services: {}\n",
    )
    .expect("old compose");
    let repeated = fixture.run(&["deploy", "--platform", "vps"]);
    assert!(repeated.status.success(), "{}", text(&repeated));
    let repeated = text(&repeated);
    assert!(
        repeated.contains("already trusts the Caddy proxy"),
        "{repeated}"
    );
    assert!(
        repeated.contains("Existing compose.yaml retained"),
        "{repeated}"
    );
    assert!(
        repeated.contains("compose.yaml replaces docker-compose.prod.yml"),
        "{repeated}"
    );
    assert_eq!(fixture.read("Rullst.toml"), config);
    assert_eq!(fixture.read("compose.yaml"), compose);
}

#[test]
fn snake_case_packages_keep_their_binary_and_get_rfc_1123_kubernetes_names() {
    let fixture = Fixture::new("My_App");
    let k8s = fixture.run(&["make:k8s"]);
    assert!(k8s.status.success(), "{}", text(&k8s));
    let deployment = fixture.read("k8s/deployment.yaml");
    assert!(deployment.contains("  name: my-app\n"), "{deployment}");
    assert!(deployment.contains("image: my-app:latest"), "{deployment}");
    assert!(!deployment.contains("My_App"), "{deployment}");
    assert!(
        fixture
            .read("k8s/service.yaml")
            .contains("name: my-app-service")
    );

    let deployed = fixture.run(&["deploy"]);
    assert!(deployed.status.success(), "{}", text(&deployed));
    // The binary keeps the Cargo package name.
    assert!(
        fixture
            .read("Dockerfile")
            .contains("/app/target/release/My_App")
    );
}
