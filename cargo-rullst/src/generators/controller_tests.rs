use super::*;

#[test]
fn controller_names_must_produce_rust_identifiers() {
    let check = |name: &str| {
        validate_controller_identifiers(name, &to_snake_case(name), &to_camel_case(name))
    };
    for valid in ["Account", "ApiAccount", "user-profile", "OrdersController"] {
        assert!(check(valid).is_ok(), "{valid} must be accepted");
    }
    for invalid in ["Bad.Name", "../../notes", "a/b", "9Lives", ""] {
        assert!(check(invalid).is_err(), "{invalid} must be rejected");
    }
}

#[test]
fn api_controllers_extract_json_bodies() {
    let source = render_controller_source("ProductController", true);
    assert!(source.contains("pub async fn store(Json(_payload): Json<CreateDto>)"));
    assert!(source.contains("Json(_payload): Json<UpdateDto>"));
    assert!(!source.contains("Form"));
    syn::parse_file(&source).expect("generated API controller should parse");

    let html = render_controller_source("ProductController", false);
    assert!(html.contains("Form(_payload): Form<CreateDto>"));
}

#[test]
fn html_controllers_style_through_the_csp_nonce_not_style_attributes() {
    let source = render_controller_source("ProductController", false);
    syn::parse_file(&source).expect("generated HTML controller should parse");
    // The default `style-src 'self' 'nonce-…'` blocks `style` attributes.
    assert!(!source.contains("style="), "{source}");
    assert!(source.contains("pub async fn index(csp_nonce: Option<Extension<CspNonce>>)"));
    assert!(source.contains("<style nonce={nonce}>{STYLES}</style>"));
    let styles = source
        .split_once("const STYLES: &str = \"")
        .and_then(|(_, rest)| rest.split_once("\";"))
        .map(|(styles, _)| styles)
        .expect("STYLES constant");
    for class in ["page", "card", "title", "lede", "signature"] {
        assert!(styles.contains(&format!(".{class} {{")), "{class}");
        assert!(source.contains(&format!("class=\"{class}\"")), "{class}");
    }
    // `html!` escapes interpolated text; CSS without these characters
    // reaches the browser unchanged.
    assert!(!styles.contains(['<', '>', '&', '"', '\'']), "{styles}");
    assert!(!render_controller_source("ProductController", true).contains("style"));
}

#[test]
fn api_instructions_register_every_write_route_as_a_machine_endpoint() {
    let lines = mapping_instructions("product_controller", true);
    let text = lines.join("\n");
    for route in [
        r#"get("/api/product" => product_controller::index),"#,
        r#"get("/api/product/{id}" => product_controller::show),"#,
        r#"post("/api/product" => product_controller::store),"#,
        r#"put("/api/product/{id}" => product_controller::update),"#,
        r#"delete("/api/product/{id}" => product_controller::delete),"#,
    ] {
        assert!(text.contains(route), "{route} missing from:\n{text}");
    }
    let machine: Vec<_> = lines
        .iter()
        .filter_map(|line| line.trim().strip_prefix("MachineEndpoint::bearer(Method::"))
        .collect();
    assert_eq!(
        machine,
        [
            r#"POST, "/api/product", api_token)?,"#,
            r#"PUT, "/api/product/{id}", api_token)?,"#,
            r#"DELETE, "/api/product/{id}", api_token)?,"#,
        ]
    );
    // Core accepts the printed routes.
    for (method, path) in [
        (rullst_core::http::Method::POST, "/api/product"),
        (rullst_core::http::Method::PUT, "/api/product/{id}"),
        (rullst_core::http::Method::DELETE, "/api/product/{id}"),
    ] {
        rullst_core::security::MachineEndpoint::bearer(method, path, "0123456789abcdef".repeat(2))
            .expect("printed route is a valid machine endpoint");
    }
    assert!(text.contains(MACHINE_ENDPOINTS_DOCS));
    assert!(
        mapping_instructions("controller", true)
            .join("\n")
            .contains(r#"post("/api/controller" => controller::store),"#)
    );

    let html = mapping_instructions("product_controller", false).join("\n");
    assert!(html.contains(r#"get("/url" => product_controller::index)"#));
    assert!(!html.contains("MachineEndpoint"));
}

/// Runs `create_new_controller` in the working directory when a parent test
/// names a report file; otherwise does nothing.
#[test]
fn controller_child_fixture() {
    let Some(report) = std::env::var_os("RULLST_CONTROLLER_REPORT") else {
        return;
    };
    let name = std::env::var("RULLST_CONTROLLER_NAME").unwrap();
    let api = std::env::var_os("RULLST_CONTROLLER_API").is_some();
    let outcome = match create_new_controller(&name, api) {
        Ok(()) => "ok".to_string(),
        Err(error) => format!("err {error}"),
    };
    fs::write(report, outcome).unwrap();
}

/// The outcome and standard output of `make:controller name` run inside
/// `project`.
fn make_controller(project: &Path, name: &str, api: bool) -> (String, String) {
    let report = project.join(".controller-result");
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "generators::controller::tests::controller_child_fixture",
            "--nocapture",
        ])
        .current_dir(project)
        .env("NO_COLOR", "1")
        .env("RULLST_CONTROLLER_REPORT", &report)
        .env("RULLST_CONTROLLER_NAME", name)
        .env_remove("RULLST_CONTROLLER_API")
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    if api {
        command.env("RULLST_CONTROLLER_API", "1");
    }
    let output = command.output().unwrap();
    assert!(output.status.success());
    let outcome = fs::read_to_string(&report).unwrap();
    fs::remove_file(report).unwrap();
    (outcome, String::from_utf8(output.stdout).unwrap())
}

fn rullst_project() -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join("Cargo.toml"),
        "[package]\nname = \"app\"\n\n[dependencies]\nrullst = \"13\"\n",
    )
    .unwrap();
    project
}

#[test]
fn a_controller_needs_a_rullst_project() {
    let outside = tempfile::tempdir().unwrap();
    let (outcome, _) = make_controller(outside.path(), "Account", false);
    assert!(outcome.starts_with("err "), "{outcome}");
    assert!(!outside.path().join("src").exists());

    let project = rullst_project();
    let (outcome, _) = make_controller(project.path(), "Account", false);
    assert_eq!(outcome, "ok");
    let source =
        fs::read_to_string(project.path().join("src/controllers/account_controller.rs")).unwrap();
    assert_eq!(source, render_controller_source("AccountController", false));
}

#[test]
fn an_invalid_controller_name_writes_nothing() {
    let project = rullst_project();
    let (outcome, _) = make_controller(project.path(), "Bad.Name", false);
    assert!(
        outcome.starts_with("err controller name must produce"),
        "{outcome}"
    );
    assert!(!project.path().join("src").exists());
}

#[test]
fn the_api_flag_selects_the_json_template_and_the_printed_routes() {
    let project = rullst_project();
    let (outcome, printed) = make_controller(project.path(), "Product", true);
    assert_eq!(outcome, "ok");
    let source =
        fs::read_to_string(project.path().join("src/controllers/product_controller.rs")).unwrap();
    assert_eq!(source, render_controller_source("ProductController", true));
    assert!(printed.contains("successfully created at"), "{printed}");
    assert!(printed.contains("How to map in your routes:"), "{printed}");
    for line in mapping_instructions("product_controller", true) {
        assert!(printed.contains(&line), "{line} missing from:\n{printed}");
    }
}

#[test]
fn the_controllers_module_is_created_and_extended_without_duplicates() {
    // Neither the directory nor `mod.rs` exists yet.
    let project = rullst_project();
    let modules = project.path().join("src/controllers/mod.rs");
    make_controller(project.path(), "Account", false);
    assert_eq!(
        fs::read_to_string(&modules).unwrap(),
        "pub mod account_controller;\n"
    );
    // A second run registers nothing twice and keeps the file it wrote.
    let account = project.path().join("src/controllers/account_controller.rs");
    fs::write(&account, "// edited\n").unwrap();
    let (_, printed) = make_controller(project.path(), "Account", false);
    assert_eq!(
        fs::read_to_string(&modules).unwrap(),
        "pub mod account_controller;\n"
    );
    assert_eq!(fs::read_to_string(&account).unwrap(), "// edited\n");
    assert!(printed.contains("already exists"), "{printed}");

    // The directory exists without `mod.rs`.
    let bare = rullst_project();
    fs::create_dir_all(bare.path().join("src/controllers")).unwrap();
    make_controller(bare.path(), "Order", false);
    assert_eq!(
        fs::read_to_string(bare.path().join("src/controllers/mod.rs")).unwrap(),
        "pub mod order_controller;\n"
    );
}

#[test]
fn a_new_module_line_starts_on_its_own_line() {
    for (existing, expected) in [
        ("", "pub mod order_controller;\n"),
        (
            "pub mod account_controller;\n",
            "pub mod account_controller;\npub mod order_controller;\n",
        ),
        (
            "pub mod account_controller;",
            "pub mod account_controller;\npub mod order_controller;\n",
        ),
    ] {
        let project = rullst_project();
        let controllers = project.path().join("src/controllers");
        fs::create_dir_all(&controllers).unwrap();
        fs::write(controllers.join("mod.rs"), existing).unwrap();
        make_controller(project.path(), "Order", false);
        assert_eq!(
            fs::read_to_string(controllers.join("mod.rs")).unwrap(),
            expected,
            "{existing:?}"
        );
    }
}

#[test]
fn main_rs_declares_the_controllers_module_once() {
    for (main, expected, announced) in [
        (
            "fn main() {}\n",
            "pub mod controllers;\nfn main() {}\n",
            true,
        ),
        (
            "pub mod controllers;\nfn main() {}\n",
            "pub mod controllers;\nfn main() {}\n",
            false,
        ),
        (
            "mod controllers;\nfn main() {}\n",
            "mod controllers;\nfn main() {}\n",
            false,
        ),
    ] {
        let project = rullst_project();
        fs::create_dir_all(project.path().join("src")).unwrap();
        fs::write(project.path().join("src/main.rs"), main).unwrap();
        let (outcome, printed) = make_controller(project.path(), "Account", false);
        assert_eq!(outcome, "ok");
        assert_eq!(
            fs::read_to_string(project.path().join("src/main.rs")).unwrap(),
            expected,
            "{main:?}"
        );
        assert_eq!(
            printed.contains("Automatically added 'pub mod controllers;'"),
            announced,
            "{main:?}"
        );
    }
    // Without a `src/main.rs` none is created.
    let library = rullst_project();
    make_controller(library.path(), "Account", false);
    assert!(!library.path().join("src/main.rs").exists());
}
