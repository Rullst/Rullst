//! New projects and subsequent scaffolds keep one bounded, checkable inventory.
use std::{
    fs,
    process::{Command, Output},
};

fn cli(root: &std::path::Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rullst"))
        .current_dir(root)
        .args(args)
        .env("RULLST_UPDATE_CHECK", "0")
        .output()
        .unwrap()
}
fn success(output: Output) {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
#[test]
fn generated_projects_preserve_instructions_and_refresh_context_after_scaffolding() {
    let temporary = tempfile::tempdir().unwrap();
    for blueprint in ["blank", "saas", "lms", "blog", "portfolio", "erp"] {
        let root = temporary.path().join(blueprint);
        success(cli(
            temporary.path(),
            &[
                "new",
                blueprint,
                "--default",
                "--blueprint",
                blueprint,
                "--database",
                "sqlite",
                "--skip-initial-migration",
            ],
        ));
        assert!(root.join("AGENTS.md").is_file());
        assert!(root.join(".rullst/context-map.json").is_file());
        success(cli(&root, &["generate:ai-context", "--check"]));
        fs::write(
            root.join("AGENTS.md"),
            "maintainer instructions must survive",
        )
        .unwrap();
        let initial = fs::read(root.join(".llms.txt")).unwrap();
        success(cli(&root, &["make:controller", "InventoryProbe"]));
        assert_ne!(initial, fs::read(root.join(".llms.txt")).unwrap());
        assert_eq!(
            fs::read_to_string(root.join("AGENTS.md")).unwrap(),
            "maintainer instructions must survive"
        );
        success(cli(&root, &["generate:ai-context", "--check"]));
        let map: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join(".rullst/context-map.json")).unwrap())
                .unwrap();
        assert!(
            map["files"]
                .as_array()
                .unwrap()
                .iter()
                .any(|file| file["path"] == "src/controllers/inventory_probe_controller.rs")
        );
        let manifest = fs::read_to_string(root.join("Cargo.toml")).unwrap();
        assert!(!manifest.contains("context-map"));
        fs::write(
            root.join("src/local_change.rs"),
            "// changed after generation",
        )
        .unwrap();
        let before = fs::read(root.join(".llms.txt")).unwrap();
        assert!(
            !cli(&root, &["generate:ai-context", "--check"])
                .status
                .success()
        );
        assert_eq!(before, fs::read(root.join(".llms.txt")).unwrap());
        success(cli(&root, &["generate:ai-context"]));
        success(cli(&root, &["generate:ai-context", "--check"]));
        let secret = "this_value_must_never_appear_in_the_context";
        fs::write(root.join(".env"), format!("PRIVATE_TOKEN={secret}\n")).unwrap();
        fs::write(
            root.join(".env.example"),
            format!("PRIVATE_TOKEN={secret}\n"),
        )
        .unwrap();
        success(cli(&root, &["generate:ai-context"]));
        let context = fs::read_to_string(root.join(".llms.txt")).unwrap();
        assert!(context.contains("PRIVATE_TOKEN"));
        assert!(!context.contains(secret));
        assert!(!context.contains(temporary.path().to_str().unwrap()));
    }
}
