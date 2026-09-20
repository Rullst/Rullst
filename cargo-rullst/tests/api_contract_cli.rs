//! Compile actual generated Rust and execute the strict TypeScript HTTP consumer.
use std::{
    fs,
    path::Path,
    process::{Child, Command, Output},
    time::{Duration, Instant},
};
fn success(output: Output) -> Output {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}
struct Server(Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn cargo(root: &Path, workspace: &Path, args: &[&str]) -> Output {
    Command::new("cargo")
        .current_dir(root)
        .args(args)
        .env("CARGO_TARGET_DIR", workspace.join("target"))
        .env("CARGO_NET_OFFLINE", "true")
        .env("CARGO_PROFILE_DEV_DEBUG", "0")
        .output()
        .unwrap()
}
#[test]
fn explicit_schema_compiles_codecs_and_executes_a_typed_http_consumer() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let source = root.join("schema.json");
    fs::write(&source, include_str!("fixtures/api_contract/openapi.json")).unwrap();
    let generated = root.join("generated");
    success(
        Command::new(env!("CARGO_BIN_EXE_rullst"))
            .current_dir(&root)
            .args(["generate:api", "--schema"])
            .arg(&source)
            .arg("--output")
            .arg(&generated)
            .output()
            .unwrap(),
    );
    success(
        Command::new(env!("CARGO_BIN_EXE_rullst"))
            .current_dir(&root)
            .args(["generate:api", "--schema"])
            .arg(&source)
            .arg("--output")
            .arg(&generated)
            .arg("--check")
            .output()
            .unwrap(),
    );
    fs::create_dir(root.join("src")).unwrap();
    fs::copy(generated.join("contract.rs"), root.join("src/contract.rs")).unwrap();
    let mut read_only: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/api_contract/openapi.json")).unwrap();
    read_only["paths"]["/lessons/{owner}"]
        .as_object_mut()
        .unwrap()
        .remove("post");
    let read_only_source = root.join("readonly.json");
    fs::write(&read_only_source, serde_json::to_vec(&read_only).unwrap()).unwrap();
    let read_only_output = root.join("readonly");
    success(
        Command::new(env!("CARGO_BIN_EXE_rullst"))
            .current_dir(&root)
            .args(["generate:api", "--schema"])
            .arg(&read_only_source)
            .arg("--output")
            .arg(&read_only_output)
            .output()
            .unwrap(),
    );
    fs::copy(
        read_only_output.join("contract.rs"),
        root.join("src/readonly_contract.rs"),
    )
    .unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented))]\npub mod contract;\npub mod readonly_contract;\n#[cfg(test)] mod consumer_tests;\n",
    )
    .unwrap();
    fs::write(
        root.join("src/consumer_tests.rs"),
        include_str!("fixtures/api_contract/consumer_tests.rs"),
    )
    .unwrap();
    fs::write(
        root.join("src/main.rs"),
        include_str!("fixtures/api_contract/server.rs"),
    )
    .unwrap();
    let security = workspace
        .join("rullst-security")
        .display()
        .to_string()
        .replace('\\', "/");
    let path = toml::Value::String(security).to_string();
    fs::write(
        root.join("Cargo.toml"),
        format!(
            r#"[package]
name = "api-contract-consumer"
version = "0.0.0"
edition = "2024"
publish = false
[workspace]
[dependencies]
rullst-security = {{ path = {path}, default-features = false }}
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"
axum = "0.8"
tokio = {{ version = "1", features = ["full"] }}
[profile.dev]
debug = 0
incremental = false
"#
        ),
    )
    .unwrap();
    fs::copy(workspace.join("Cargo.lock"), root.join("Cargo.lock")).unwrap();
    success(cargo(&root, workspace, &["test", "--lib", "-j", "1"]));
    success(cargo(
        &root,
        workspace,
        &["clippy", "--all-targets", "-j", "1", "--", "-D", "warnings"],
    ));
    if std::env::var("RULLST_API_CONTRACT_TESTS").as_deref() == Ok("1") {
        let tsc = workspace.join(".github/api-contract/node_modules/typescript/bin/tsc");
        assert!(
            tsc.is_file(),
            "Install the pinned compiler with npm ci --prefix .github/api-contract --ignore-scripts"
        );
        fs::write(
            generated.join("consumer.ts"),
            include_str!("fixtures/api_contract/consumer.ts"),
        )
        .unwrap();
        success(
            Command::new("node")
                .arg(tsc)
                .current_dir(&generated)
                .args([
                    "--strict",
                    "--exactOptionalPropertyTypes",
                    "--target",
                    "ES2022",
                    "--module",
                    "commonjs",
                    "--lib",
                    "ES2022,DOM",
                    "--outDir",
                    "js",
                    "client.ts",
                    "consumer.ts",
                ])
                .output()
                .unwrap(),
        );
        success(cargo(
            &root,
            workspace,
            &["build", "--bin", "api-contract-consumer", "-j", "1"],
        ));
        let ready = root.join("ready");
        let mut server = Server(
            Command::new(workspace.join("target/debug").join(format!(
                "api-contract-consumer{}",
                std::env::consts::EXE_SUFFIX
            )))
            .env("RULLST_API_FIXTURE_READY", &ready)
            .spawn()
            .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(20);
        while !ready.is_file() {
            assert!(
                Instant::now() < deadline,
                "HTTP fixture did not become ready"
            );
            assert!(
                server.0.try_wait().unwrap().is_none(),
                "HTTP fixture exited"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        success(
            Command::new("node")
                .current_dir(&generated)
                .arg("js/consumer.js")
                .arg(format!("http://{}", fs::read_to_string(ready).unwrap()))
                .output()
                .unwrap(),
        );
        drop(server);
    } else {
        eprintln!(
            "TypeScript HTTP acceptance not requested; hosted Linux cli-standard requires RULLST_API_CONTRACT_TESTS=1"
        );
    }
    success(cargo(
        &root,
        workspace,
        &["clean", "--package", "api-contract-consumer"],
    ));
}
