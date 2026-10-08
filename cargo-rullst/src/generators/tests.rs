use super::*;
use syn::visit::Visit;

const PANICKING_RUNTIME_CALLS: [&str; 5] = [
    ".unwrap(",
    ".expect(",
    "panic!(",
    "todo!(",
    "unimplemented!(",
];

#[derive(Default)]
struct RuntimeLiteralAudit {
    violations: Vec<String>,
}

impl<'ast> Visit<'ast> for RuntimeLiteralAudit {
    fn visit_lit_str(&mut self, literal: &'ast syn::LitStr) {
        let value = literal.value();
        for needle in PANICKING_RUNTIME_CALLS {
            // Unit tests use the exact needles to assert the generated
            // output. Longer literals containing them are emitted source
            // candidates and must remain panic-free.
            if value != needle && value.contains(needle) {
                self.violations.push(value.clone());
            }
        }
        syn::visit::visit_lit_str(self, literal);
    }
}

#[test]
fn test_is_rullst_project() {
    // Since we are running in the Rullst repo with a Cargo.toml containing "rullst", this should return true
    assert!(is_rullst_project());
}

#[test]
fn test_register_mod_ast_flow() {
    let temp_dir = std::env::temp_dir().join(format!("rullst_test_mod_{}", rand::random::<u64>()));
    let _ = fs::create_dir_all(&temp_dir);
    let mod_file = temp_dir.join("mod.rs");

    // 1. Register in new non-existent file
    register_mod_ast(&mod_file, "users").unwrap();
    let content1 = fs::read_to_string(&mod_file).unwrap();
    assert!(content1.contains("pub mod users;"));

    // 2. Register same module again (idempotent, shouldn't duplicate)
    register_mod_ast(&mod_file, "users").unwrap();
    let content2 = fs::read_to_string(&mod_file).unwrap();
    assert_eq!(content1, content2);

    // 3. Register a second module
    register_mod_ast(&mod_file, "posts").unwrap();
    let content3 = fs::read_to_string(&mod_file).unwrap();
    assert!(content3.contains("pub mod users;"));
    assert!(content3.contains("pub mod posts;"));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn generated_identifiers_reject_keywords_and_syntax() {
    assert!(is_valid_rust_identifier("billing_customer"));
    assert!(is_valid_rust_identifier("BillingCustomer"));
    assert!(!is_valid_rust_identifier("type"));
    assert!(!is_valid_rust_identifier("bad-name"));
    assert!(!is_valid_rust_identifier(""));
}

#[test]
fn primary_turso_projects_are_distinguished_from_hybrid_projects() {
    let turso_primary = r#"
[dependencies]
rullst = { version = "12", features = ["orm", "orm-turso"] }
rullst-orm = { version = "12", features = ["turso"] }
"#;
    assert_eq!(
        project_orm_backend_from_manifest(turso_primary),
        ProjectOrmBackend::Turso
    );

    let hybrid = format!("{turso_primary}\nsqlx = \"0.9\"\n");
    assert_eq!(
        project_orm_backend_from_manifest(&hybrid),
        ProjectOrmBackend::Sqlx
    );
    assert_eq!(
        project_orm_backend_from_manifest("[dependencies]\nrullst = \"12\""),
        ProjectOrmBackend::Sqlx
    );
}

#[test]
fn embedded_runtime_templates_have_no_panicking_calls() {
    let generators_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generators");
    let mut violations = Vec::new();

    for entry in walkdir::WalkDir::new(generators_dir) {
        let entry = entry.expect("generator source entry");
        let path = entry.path();
        if !entry.file_type().is_file()
            || path.extension().and_then(std::ffi::OsStr::to_str) != Some("rs")
        {
            continue;
        }

        let source = fs::read_to_string(path).expect("generator source");
        let syntax = syn::parse_file(&source).expect("generator source must parse");
        let mut audit = RuntimeLiteralAudit::default();
        audit.visit_file(&syntax);

        for literal in audit.violations {
            violations.push(format!("{}: {literal:?}", path.display()));
        }
    }

    assert!(
        violations.is_empty(),
        "embedded generated-runtime literals contain panic paths:\n{}",
        violations.join("\n")
    );
}

#[test]
fn slash_path_names_windows_paths_with_forward_slashes() {
    assert_eq!(
        slash_path(Path::new("crates\\http\\src\\lib.rs")),
        "crates/http/src/lib.rs"
    );
    assert_eq!(
        slash_path(&Path::new("k8s").join("deployment.yaml")),
        "k8s/deployment.yaml"
    );
}
