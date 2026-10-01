//! `deploy` reports the provider CLI's real outcome through its exit status.
#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic)]

use std::{
    fs,
    os::unix::fs::PermissionsExt as _,
    path::{Path, PathBuf},
    process::{Command, Output},
};

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("rullst-cli-deploy-{}", rand::random::<u64>()));
        fs::create_dir_all(root.join("tools")).expect("tool directory");
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"deploy-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst = \"13\"\n",
        )
        .expect("fixture manifest");
        fs::write(root.join("Dockerfile"), "FROM scratch\n").expect("existing Dockerfile");
        Self { root }
    }

    fn provider(&self, name: &str, status: u8) {
        let path = self.root.join("tools").join(name);
        fs::write(&path, format!("#!/bin/sh\nexit {status}\n")).expect("provider fixture");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
            .expect("provider permissions");
    }

    fn deploy(&self, platform: &str) -> Output {
        self.run(&["deploy", "--platform", platform])
    }

    fn run(&self, arguments: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_rullst"))
            .current_dir(&self.root)
            .args(arguments)
            .env("RULLST_DISABLE_UPDATE_CHECK", "1")
            .env("NO_COLOR", "1")
            .env("PATH", self.root.join("tools"))
            .output()
            .unwrap_or_else(|error| panic!("run {arguments:?}: {error}"))
    }

    fn read(&self, relative: &str) -> String {
        fs::read_to_string(self.root.join(relative))
            .unwrap_or_else(|error| panic!("read {relative}: {error}"))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
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
fn failed_provider_deployments_exit_nonzero_and_missing_tools_stay_advisory() {
    let fixture = Fixture::new();

    let missing = fixture.deploy("fly");
    assert!(missing.status.success(), "{}", text(&missing));
    assert!(text(&missing).contains("not found"));
    assert!(Path::new(&fixture.root.join("fly.toml")).is_file());

    for (platform, program, command) in [
        ("fly", "flyctl", "flyctl deploy"),
        ("railway", "railway", "railway up"),
    ] {
        fixture.provider(program, 3);
        let failed = fixture.deploy(platform);
        assert!(!failed.status.success(), "{}", text(&failed));
        assert!(text(&failed).contains(command), "{}", text(&failed));
        assert!(!text(&failed).contains("successfully deployed"));

        fixture.provider(program, 0);
        let deployed = fixture.deploy(platform);
        assert!(deployed.status.success(), "{}", text(&deployed));
        assert!(text(&deployed).contains("successfully deployed"));
    }
}

#[test]
fn unknown_platforms_fail_before_scaffolding_a_dockerfile() {
    let fixture = Fixture::new();
    fs::remove_file(fixture.root.join("Dockerfile")).expect("start without a Dockerfile");

    let rejected = fixture.deploy("flyio");
    assert!(!rejected.status.success(), "{}", text(&rejected));
    assert!(text(&rejected).contains("Unknown platform 'flyio'"));
    for generated in ["Dockerfile", ".dockerignore", "fly.toml"] {
        assert!(
            !fixture.root.join(generated).exists(),
            "{generated} was written for a rejected platform"
        );
    }
}

#[test]
fn snake_case_packages_get_rfc_1123_kubernetes_and_fly_names() {
    let fixture = Fixture::new();
    fs::write(
        fixture.root.join("Cargo.toml"),
        "[package]\nname = \"My_App\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst = \"13\"\n",
    )
    .expect("snake_case manifest");
    fs::remove_file(fixture.root.join("Dockerfile")).expect("start without a Dockerfile");

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
    assert!(
        fixture
            .read("k8s/ingress.yaml")
            .contains("host: my-app.local")
    );

    let fly = fixture.deploy("fly");
    assert!(fly.status.success(), "{}", text(&fly));
    assert!(fixture.read("fly.toml").contains("app = \"my-app\""));
    // The binary keeps the Cargo package name.
    assert!(
        fixture
            .read("Dockerfile")
            .contains("/app/target/release/My_App")
    );
}

#[test]
fn vps_deploy_trusts_its_caddy_proxy_in_the_image_configuration() {
    let fixture = Fixture::new();
    fs::remove_file(fixture.root.join("Dockerfile")).expect("start without a Dockerfile");
    fs::write(
        fixture.root.join("Rullst.toml"),
        "[database]\nurl = \"sqlite://db.sqlite\"\n",
    )
    .expect("fixture configuration");

    // Without a trusted proxy every request reaches the app from Caddy, so
    // all clients share the starters' per-client login budget.
    let vps = fixture.deploy("vps");
    assert!(vps.status.success(), "{}", text(&vps));
    let config = fixture.read("Rullst.toml");
    assert!(config.starts_with("[database]\nurl = \"sqlite://db.sqlite\"\n"));
    assert!(config.contains("[security]\n"), "{config}");
    assert!(
        config.contains("trusted_proxies = [\"172.31.250.10\"]\n"),
        "{config}"
    );
    assert!(
        fixture
            .read("docker-compose.prod.yml")
            .contains("ipv4_address: 172.31.250.10\n")
    );
    assert!(
        fixture
            .read("Dockerfile")
            .contains("COPY --chown=10001:10001 Rullst.toml /app/Rullst.toml\n")
    );

    let repeated = fixture.deploy("vps");
    assert!(repeated.status.success(), "{}", text(&repeated));
    assert!(text(&repeated).contains("already trusts the Caddy proxy"));
    assert_eq!(fixture.read("Rullst.toml"), config);

    fs::write(
        fixture.root.join("docker-compose.prod.yml"),
        "services: {}\n",
    )
    .expect("old compose");
    let outdated = fixture.deploy("vps");
    assert!(outdated.status.success(), "{}", text(&outdated));
    assert!(text(&outdated).contains("does not pin Caddy to 172.31.250.10"));

    // Managed platforms explain the requirement instead of guessing networks.
    fs::remove_file(fixture.root.join("Rullst.toml")).expect("remove configuration");
    let render = fixture.deploy("render");
    assert!(render.status.success(), "{}", text(&render));
    assert!(text(&render).contains("Render forwards every request through its proxy"));
}
