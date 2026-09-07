#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[cfg(target_os = "windows")]
#[test]
fn loaded_library_lock_errors_defer_cleanup_without_a_warning() {
    for code in [5, 32] {
        let error = std::io::Error::from_raw_os_error(code);
        assert!(is_expected_loaded_library_removal_error(&error));
    }

    let unexpected = std::io::Error::from_raw_os_error(3);
    assert!(!is_expected_loaded_library_removal_error(&unexpected));
}

#[test]
fn missing_library_errors_include_the_resolved_platform_path() {
    let base =
        std::env::temp_dir().join(format!("rullst-missing-library-{}", uuid::Uuid::new_v4()));
    let error =
        load_dylib_router(base.to_str().unwrap(), false).expect_err("missing base path must fail");
    let expected_extension = if cfg!(target_os = "windows") {
        ".dll"
    } else if cfg!(target_os = "macos") {
        ".dylib"
    } else {
        ".so"
    };
    assert!(error.to_string().contains(expected_extension));

    let explicit = format!("{}{}", base.display(), expected_extension);
    let error = load_dylib_router(&explicit, true).expect_err("missing explicit path must fail");
    assert!(error.to_string().contains(&explicit));
}

#[test]
fn invalid_library_isolated_copy_fails_without_touching_the_source() {
    let directory =
        std::env::temp_dir().join(format!("rullst-invalid-library-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&directory).unwrap();
    let extension = if cfg!(target_os = "windows") {
        "dll"
    } else if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    };
    let source = directory.join(format!("application.{extension}"));
    let stale = directory.join(format!("application_active_stale.{extension}"));
    std::fs::write(&source, b"not a dynamic library").unwrap();
    std::fs::write(&stale, b"stale isolated copy").unwrap();

    assert!(load_dylib_router(source.to_str().unwrap(), false).is_err());
    assert!(source.exists());
    assert!(!stale.exists());

    std::fs::remove_dir_all(&directory).unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn valid_system_library_loads_but_missing_router_symbol_fails_safely() {
    let system_library = [
        "/lib/x86_64-linux-gnu/libm.so.6",
        "/usr/lib/x86_64-linux-gnu/libm.so.6",
        "/lib/aarch64-linux-gnu/libm.so.6",
        "/usr/lib/aarch64-linux-gnu/libm.so.6",
    ]
    .into_iter()
    .find(|path| std::path::Path::new(path).is_file());
    let Some(system_library) = system_library else {
        return;
    };

    let directory =
        std::env::temp_dir().join(format!("rullst-system-library-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&directory).unwrap();
    let source = directory.join("fixture.so");
    std::fs::copy(system_library, &source).unwrap();

    let error = load_dylib_router(source.to_str().unwrap(), true)
        .expect_err("system library does not expose the Rullst router ABI");
    assert!(error.to_string().contains("dlsym"));
    assert!(source.exists());
    assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 1);

    std::fs::remove_file(&source).unwrap();
    std::fs::remove_dir(&directory).unwrap();
}
