//! The `--json` contract (CLI reference, "Machine-readable output"): stdout is
//! exactly one JSON document with a `schema` field and no escape codes, whatever
//! the exit status. One test per command.
use serde_json::Value;
use std::{fs, path::Path, process::Command};

struct Project {
    dir: tempfile::TempDir,
}

impl Project {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("project directory");
        let root = dir.path();
        fs::create_dir_all(root.join("src/models")).expect("model directory");
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"json-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst = \"13\"\nrullst-auth = \"13\"\nserde = \"1\"\n",
        )
        .expect("manifest");
        fs::write(
            root.join("src/main.rs"),
            "fn main() {}\nroutes![\n    get(\"/users/{id}\" => controllers::users::show),\n];\n",
        )
        .expect("main");
        fs::write(
            root.join("src/models/user.rs"),
            "#[derive(Orm)]\n#[orm(table = \"users\")]\npub struct User {\n    pub id: i64,\n    pub email: Option<String>,\n}\n",
        )
        .expect("model");
        Self { dir }
    }

    fn json(&self, arguments: &[&str], schema: &str) -> Value {
        json_in(self.dir.path(), arguments, schema)
    }
}

fn json_in(directory: &Path, arguments: &[&str], schema: &str) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_cargo-rullst"))
        .args(arguments)
        .current_dir(directory)
        .env("NO_COLOR", "1")
        .env("RULLST_DISABLE_UPDATE_CHECK", "1")
        .env("CARGO_NET_OFFLINE", "true")
        .env_remove("RULLST_ENV")
        .env_remove("APP_ENV")
        .env_remove("APP_KEY")
        .output()
        .expect("command runs");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.stdout.contains(&0x1b),
        "{arguments:?} wrote an escape code to stdout"
    );
    let document: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!("{arguments:?} stdout is not one JSON document: {error}\n{stderr}")
    });
    assert_eq!(document["schema"], schema, "{arguments:?}");
    document
}

#[test]
fn info_prints_one_document() {
    let document = Project::new().json(&["info", "--json"], "rullst.cli-info.v1");
    assert_eq!(document["schema_version"], "rullst.cli-info.v1");
    assert_eq!(document["project"]["name"], "json-fixture");
}

#[test]
fn doctor_prints_one_document() {
    let document = Project::new().json(&["doctor", "--json"], "rullst.cli-doctor.v1");
    assert_eq!(document["schema_version"], "rullst.cli-doctor.v1");
}

#[test]
fn audit_prints_one_document() {
    let document = Project::new().json(&["audit", "--json"], "rullst.cli-audit.v1");
    assert_eq!(document["schema_version"], "rullst.cli-audit.v1");
}

#[test]
fn inspect_routes_prints_one_document() {
    let document = Project::new().json(&["inspect", "routes", "--json"], "rullst.cli-routes.v1");
    assert_eq!(document["schema_version"], "rullst.cli-routes.v1");
    assert_eq!(document["routes"][0]["path"], "/users/{id}");
}

#[test]
fn inspect_models_prints_one_document() {
    let document = Project::new().json(&["inspect", "models", "--json"], "rullst.cli-models.v1");
    assert_eq!(document["complete"], true);
    assert_eq!(document["files"][0]["file"], "user.rs");
    assert_eq!(document["files"][0]["items"][0]["name"], "User");
    assert_eq!(
        document["files"][0]["items"][0]["fields"][1]["type"],
        "Option<String>"
    );
}

#[test]
fn inspect_schema_prints_one_document() {
    let document = Project::new().json(&["inspect", "schema", "--json"], "rullst.cli-schema.v1");
    assert_eq!(document["source"], "src");
    assert_eq!(document["models"][0]["table"], "users");
}

#[test]
fn pkg_list_prints_one_document() {
    let document = Project::new().json(&["pkg", "list", "--json"], "rullst.cli-pkg.v1");
    assert_eq!(document["table"], "dependencies");
    assert_eq!(document["packages"], serde_json::json!(["rullst-auth"]));
}

#[test]
fn deploy_doctor_prints_one_document() {
    let project = Project::new();
    fs::write(
        project.dir.path().join("Rullst.toml"),
        "[app]\nenv = 'development'\n",
    )
    .expect("config");
    let document = project.json(
        &["deploy:doctor", "--json"],
        "rullst.deployment-diagnostic.v1",
    );
    assert_eq!(
        document["schema_version"],
        "rullst.deployment-diagnostic.v1"
    );
}

#[test]
fn academy_doctor_prints_one_document() {
    let document = Project::new().json(
        &["academy:doctor", "--json"],
        "rullst.academy-diagnostic.v1",
    );
    assert_eq!(document["certification"], false);
}

#[test]
fn upgrade_dry_run_prints_one_document() {
    let project = Project::new();
    let root = project.dir.path();
    let framework = root.join("framework");
    fs::create_dir_all(framework.join("src")).expect("framework");
    fs::write(
        framework.join("Cargo.toml"),
        "[package]\nname = \"rullst\"\nversion = \"12.0.0\"\nedition = \"2024\"\n",
    )
    .expect("framework manifest");
    fs::write(framework.join("src/lib.rs"), "pub fn marker() {}\n").expect("framework source");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"json-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst = { version = \"12\", path = \"framework\", default-features = false }\n",
    )
    .expect("manifest");
    let document = project.json(
        &["upgrade", "--dry-run", "--json"],
        "rullst.upgrade-plan.v1",
    );
    assert_eq!(document["schema_version"], "rullst.upgrade-plan.v1");
}

#[test]
fn json_is_refused_where_it_is_not_offered() {
    let project = Project::new();
    for arguments in [["pkg", "add", "--json"], ["inspect", "macros", "--json"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_cargo-rullst"))
            .args(arguments)
            .current_dir(project.dir.path())
            .env("RULLST_DISABLE_UPDATE_CHECK", "1")
            .output()
            .expect("command runs");
        assert!(!output.status.success(), "{arguments:?}");
        assert!(output.stdout.is_empty(), "{arguments:?} wrote to stdout");
    }
}
