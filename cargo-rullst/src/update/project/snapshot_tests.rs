//! Path policy, bounded reads, inventories and copies of project inputs.
use super::*;

const UNSAFE: &str = "project preparation rejected: project inventory contains an unsupported or unsafe relative path";
const TARGET: &str = "project preparation rejected: remove generated target artifacts from the Git input inventory before preparation";

fn rejection<T: std::fmt::Debug>(result: Result<T, ProjectError>) -> String {
    result.unwrap_err().to_string()
}

#[test]
fn relative_paths_are_plain_bounded_components() {
    for accepted in ["Cargo.toml", "src/main.rs", "a.b/c", &"a".repeat(4096)] {
        validate_path(accepted).unwrap();
    }
    for refused in [
        String::new(),
        "a".repeat(4097),
        "src\\main.rs".into(),
        "c:main.rs".into(),
        "src/\u{7}.rs".into(),
        "src//main.rs".into(),
        "./main.rs".into(),
        "src/../main.rs".into(),
        ".git/config".into(),
        "src /main.rs".into(),
        "src./main.rs".into(),
        "/etc/passwd".into(),
    ] {
        assert_eq!(rejection(validate_path(&refused)), UNSAFE, "{refused:?}");
    }
    for generated in ["target/debug/app", "crates/web/target"] {
        assert_eq!(rejection(validate_path(generated)), TARGET, "{generated}");
    }
}

#[test]
fn reads_return_regular_files_and_report_absence() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path();
    fs::create_dir(root.join("src")).unwrap();
    fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
    assert_eq!(read(root, "src/main.rs").unwrap().unwrap(), b"fn main() {}");
    assert_eq!(read(root, "src/lib.rs").unwrap(), None);
    assert_eq!(read(root, "missing/lib.rs").unwrap(), None);
    fs::write(root.join("file"), "x").unwrap();
    assert_eq!(
        rejection(read(root, "file/child.rs")),
        "project preparation rejected: project input has a linked or non-directory ancestor"
    );
    let special = "project preparation rejected: project inputs must be regular files bounded to 64 MiB; links/submodules/special files are unsupported";
    assert_eq!(rejection(read(root, "src")), special);
    let largest = fs::File::create(root.join("largest")).unwrap();
    largest.set_len(64 * 1024 * 1024).unwrap();
    assert_eq!(read(root, "largest").unwrap().unwrap().len(), 67_108_864);
    largest.set_len(64 * 1024 * 1024 + 1).unwrap();
    assert_eq!(rejection(read(root, "largest")), special);
}

#[cfg(unix)]
#[test]
fn links_and_unsearchable_ancestors_are_refused() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let root = tempfile::tempdir().unwrap();
    let root = root.path();
    fs::create_dir(root.join("real")).unwrap();
    fs::write(root.join("real/lib.rs"), "x").unwrap();
    symlink(root.join("real"), root.join("alias")).unwrap();
    symlink(root.join("real/lib.rs"), root.join("link.rs")).unwrap();
    assert!(read(root, "alias/lib.rs").is_err());
    assert!(read(root, "link.rs").is_err());
    if !rustix::process::geteuid().is_root() {
        fs::set_permissions(root.join("real"), fs::Permissions::from_mode(0o600)).unwrap();
        assert!(read(root, "real/lib.rs").is_err());
        assert!(read(root, "real/inner/lib.rs").is_err());
        fs::set_permissions(root.join("real"), fs::Permissions::from_mode(0o700)).unwrap();
    }
}

/// sysfs reports a 4096-byte size for every attribute and returns less.
#[cfg(target_os = "linux")]
#[test]
fn an_input_shorter_than_its_reported_size_is_refused() {
    let root = Path::new("/sys/devices/system/cpu");
    if !root.join("online").exists() {
        return;
    }
    assert_eq!(
        rejection(read(root, "online")),
        "project preparation rejected: project input changed size while copying"
    );
}

#[test]
fn trees_list_regular_files_of_an_unlinked_directory() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path();
    fs::create_dir_all(root.join("src/bin")).unwrap();
    fs::write(root.join("Cargo.toml"), "").unwrap();
    fs::write(root.join("src/bin/tool.rs"), "").unwrap();
    assert_eq!(
        tree(root).unwrap(),
        BTreeSet::from(["Cargo.toml".into(), "src/bin/tool.rs".into()])
    );
    assert!(tree(&root.join("Cargo.toml")).is_err());
    fs::create_dir(root.join("target")).unwrap();
    assert_eq!(rejection(tree(root)), TARGET);
}

#[cfg(unix)]
#[test]
fn trees_refuse_links_and_special_files() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let root = root.path();
    fs::write(root.join("Cargo.toml"), "").unwrap();
    symlink(root.join("Cargo.toml"), root.join("alias.toml")).unwrap();
    let linked = "project preparation rejected: prepared source contains linked or special inputs";
    assert_eq!(rejection(tree(root)), linked);
    fs::remove_file(root.join("alias.toml")).unwrap();
    assert!(
        std::process::Command::new("mkfifo")
            .arg(root.join("pipe"))
            .status()
            .unwrap()
            .success()
    );
    assert_eq!(rejection(tree(root)), linked);
    let other = tempfile::tempdir().unwrap();
    symlink(root, other.path().join("alias")).unwrap();
    assert!(tree(&other.path().join("alias")).is_err());
}

fn git(root: &Path, args: &[&str]) {
    assert!(
        std::process::Command::new("git")
            .args(args)
            .current_dir(root)
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap()
            .success()
    );
}

#[test]
fn the_inventory_holds_tracked_and_unignored_files_and_the_lockfile() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path();
    git(root, &["init", "--quiet"]);
    assert_eq!(
        rejection(inventory(root)),
        "project preparation rejected: select a Git project directory containing Cargo.toml"
    );
    fs::create_dir(root.join("src")).unwrap();
    fs::write(root.join("Cargo.toml"), "[package]").unwrap();
    fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
    fs::write(root.join(".gitignore"), "secret.env\n").unwrap();
    fs::write(root.join("secret.env"), "TOKEN=x").unwrap();
    assert_eq!(
        inventory(root).unwrap(),
        BTreeSet::from([
            ".gitignore".into(),
            "Cargo.lock".into(),
            "Cargo.toml".into(),
            "src/main.rs".into(),
        ])
    );
    // A listing larger than 17 KiB still fits the 16 MiB output bound.
    fs::create_dir(root.join("assets")).unwrap();
    for index in 0..900 {
        fs::write(root.join(format!("assets/image-{index:05}.svg")), "").unwrap();
    }
    assert_eq!(inventory(root).unwrap().len(), 904);
}

#[test]
fn copies_record_every_input_and_unchanged_detects_edits() {
    let source = tempfile::tempdir().unwrap();
    let root = source.path();
    git(root, &["init", "--quiet"]);
    fs::create_dir(root.join("src")).unwrap();
    fs::write(root.join("Cargo.toml"), "[package]").unwrap();
    // More than the 512 KiB and 1 MiB that shortened bounds would allow.
    fs::write(root.join("src/data.bin"), vec![7u8; 2 * 1024 * 1024]).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::write(root.join("run.sh"), "#!/bin/sh\n").unwrap();
        fs::set_permissions(root.join("run.sh"), fs::Permissions::from_mode(0o755)).unwrap();
    }
    let paths = inventory(root).unwrap();
    let stage = tempfile::tempdir().unwrap();
    let (before, candidate) = (stage.path().join("before"), stage.path().join("candidate"));
    let records = copy(root, &before, &candidate, &paths).unwrap();
    assert_eq!(records.len(), paths.len());
    let lock = records
        .iter()
        .find(|record| record.path == "Cargo.lock")
        .unwrap();
    assert_eq!((lock.bytes, lock.sha256.as_deref()), (0, None));
    assert_eq!(fs::read(before.join("Cargo.toml")).unwrap(), b"[package]");
    assert_eq!(
        fs::read(candidate.join("src/data.bin")).unwrap().len(),
        2_097_152
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for copied in [&before, &candidate] {
            let mode = |name: &str| {
                fs::metadata(copied.join(name))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777
            };
            assert_eq!(mode("run.sh"), 0o700);
            assert_eq!(mode("Cargo.toml"), 0o600);
        }
    }
    unchanged(root, &paths, &records).unwrap();
    fs::write(root.join("Cargo.toml"), "[package] # edited").unwrap();
    assert!(unchanged(root, &paths, &records).is_err());
    fs::write(root.join("Cargo.toml"), "[package]").unwrap();
    fs::write(root.join("new.rs"), "").unwrap();
    assert!(unchanged(root, &paths, &records).is_err());
    unchanged_ignoring(
        root,
        &paths,
        &records,
        &BTreeSet::from(["new.rs".to_string()]),
    )
    .unwrap();
}
