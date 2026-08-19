//! Unit tests for error console parsing and source context extraction.

#![allow(clippy::unwrap_used)]

use super::*;

#[test]
fn test_find_source_location_linux() {
    let bt = "   0: rullst::error_console::tests::test_panic\n             at /home/user/project/src/error_console.rs:42";
    let res = find_source_location(bt);
    assert_eq!(
        res,
        Some(("/home/user/project/src/error_console.rs".to_string(), 42))
    );
}

#[test]
fn test_find_source_location_windows() {
    let bt = "   0: rullst::error_console::tests::test_panic\n             at C:\\Users\\user\\project\\src\\error_console.rs:55";
    let res = find_source_location(bt);
    assert_eq!(
        res,
        Some((
            "C:\\Users\\user\\project\\src\\error_console.rs".to_string(),
            55
        ))
    );
}

#[test]
fn test_find_source_location_none() {
    let bt = "   0: rust_panic\n             at /home/user/project/main.rs:100";
    let res = find_source_location(bt);
    assert_eq!(res, None);
}

#[test]
fn test_extract_source_context_bounds() {
    use std::io::Write;
    let cwd = std::env::current_dir().unwrap();
    let test_file = cwd.join("test_extract_source_context.rs");
    let mut file = std::fs::File::create(&test_file).unwrap();
    writeln!(file, "line 1").unwrap();
    writeln!(file, "line 2").unwrap();
    writeln!(file, "line 3").unwrap();
    file.sync_all().unwrap();

    let path_str = test_file.to_str().unwrap();

    // Testing line 1 (boundary)
    let ctx = extract_source_context(path_str, 1, 1).unwrap();
    assert_eq!(ctx.len(), 2);
    assert_eq!(ctx[0].1, "line 1");
    assert!(ctx[0].2); // is_target

    // Testing end of file
    let ctx = extract_source_context(path_str, 3, 1).unwrap();
    assert_eq!(ctx.len(), 2);
    assert_eq!(ctx[1].1, "line 3");
    assert!(ctx[1].2);

    let _ = std::fs::remove_file(test_file);
}
