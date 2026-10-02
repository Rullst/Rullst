use super::*;

fn project() -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(directory.path()).unwrap();
    fs::create_dir_all(root.join("src/models")).unwrap();
    fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
    (directory, root)
}

#[test]
fn ordinary_project_files_resolve_below_the_root() {
    let (_guard, root) = project();
    let existing = resolve(&root, "src/main.rs").unwrap();
    assert!(existing.exists);
    assert_eq!(existing.display, "src/main.rs");
    assert_eq!(existing.absolute, root.join("src/main.rs"));
    assert!(!existing.sensitive);

    let new = resolve(&root, "./src/controllers/posts.rs").unwrap();
    assert!(!new.exists);
    assert_eq!(new.display, "src/controllers/posts.rs");
    assert!(resolve(&root, ".env.example").is_ok());
    assert!(resolve(&root, "src/models/credentials_form.rs").is_ok());
    assert!(resolve(&root, "Cargo.toml").unwrap().sensitive);
    assert!(resolve(&root, "build.rs").unwrap().sensitive);
    assert!(
        resolve(&root, ".github/workflows/ci.yml")
            .unwrap()
            .sensitive
    );
    // Cargo builds and runs these targets on `cargo test`.
    for raw in [
        "tests/smoke.rs",
        "benches/load.rs",
        "examples/demo.rs",
        "macros/tests/ui.rs",
    ] {
        assert!(resolve(&root, raw).unwrap().sensitive, "{raw}");
    }
    assert!(!resolve(&root, "src/tests.rs").unwrap().sensitive);
    assert!(runs_during_cargo("#[cfg(test)]\nmod tests {}"));
    assert!(runs_during_cargo("#[tokio::test]\nasync fn t() {}"));
    assert!(runs_during_cargo("use proc_macro::TokenStream;"));
    assert!(!runs_during_cargo("fn main() {}"));
}

#[test]
fn traversal_absolute_and_malformed_paths_are_rejected() {
    let (_guard, root) = project();
    for raw in [
        "../outside.rs",
        "src/../../etc/passwd",
        "src/..",
        "a/b/../../..",
    ] {
        assert_eq!(resolve(&root, raw), Err(PathError::Escapes), "{raw}");
    }
    // Absolute and drive-qualified paths fail on every platform.
    let absolute = root.join("src/main.rs");
    assert!(resolve(&root, absolute.to_str().unwrap()).is_err());
    assert!(resolve(&root, "C:/x.rs").is_err());
    assert!(resolve(&root, "/etc/passwd").is_err());
    for raw in [
        "",
        ".",
        "~/x",
        "src\\main.rs",
        "src/ma\0in.rs",
        "src/line\nbreak.rs",
        "src/file.rs:stream",
        "NUL",
        "src/con.txt",
        "src/com1",
        "trailing.",
        // Windows 8.3 aliases of `.git`, `.cargo` and `.env`.
        "GIT~1/hooks/pre-commit",
        "CARGO~1/config.toml",
        "ENV~1",
        "src/PROJEC~2.RS",
        // HFS+ ignores these characters, so this name would be `.git`.
        ".g\u{200c}it/hooks/pre-commit",
        "src/\u{feff}main.rs",
    ] {
        assert_eq!(resolve(&root, raw), Err(PathError::Malformed), "{raw:?}");
    }
    assert_eq!(resolve(&root, &"a/".repeat(300)), Err(PathError::Malformed));
}

#[test]
fn version_control_build_output_and_secrets_are_denied() {
    let (_guard, root) = project();
    for raw in [
        ".git/config",
        ".git",
        "sub/.GIT/hooks/pre-commit",
        "target/debug/app",
        ".cargo/config.toml",
        ".env",
        "config/.ENV",
        ".env.production",
        "credentials.toml",
        "certs/server.key",
        "certs/server.PEM",
        "Cargo.lock",
        "rust-toolchain.toml",
        "data/app.db",
        ".ssh/id_ed25519",
    ] {
        assert!(
            matches!(resolve(&root, raw), Err(PathError::Denied(_))),
            "{raw}"
        );
    }
}

#[test]
fn directories_and_file_prefixes_are_not_files() {
    let (_guard, root) = project();
    assert_eq!(resolve(&root, "src/models"), Err(PathError::NotAFile));
    assert_eq!(
        resolve(&root, "src/main.rs/child.rs"),
        Err(PathError::NotAFile)
    );
}

#[cfg(unix)]
#[test]
fn symlinks_anywhere_on_the_path_are_rejected() {
    let (_guard, root) = project();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("secret.txt"), "outside").unwrap();
    std::os::unix::fs::symlink(outside.path(), root.join("linked")).unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("secret.txt"),
        root.join("src/escape.rs"),
    )
    .unwrap();
    std::os::unix::fs::symlink(root.join("src/main.rs"), root.join("src/inner.rs")).unwrap();
    assert_eq!(resolve(&root, "linked/secret.txt"), Err(PathError::Symlink));
    assert_eq!(resolve(&root, "linked/new.rs"), Err(PathError::Symlink));
    assert_eq!(resolve(&root, "src/escape.rs"), Err(PathError::Symlink));
    // Even a link that stays inside the project is refused.
    assert_eq!(resolve(&root, "src/inner.rs"), Err(PathError::Symlink));
}
