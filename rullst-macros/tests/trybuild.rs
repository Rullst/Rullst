use std::fs;
use std::path::Path;

#[test]
#[cfg_attr(miri, ignore)]
fn ui() {
    copy_i18n_fixtures();
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}

/// `i18n_catalog!` reads `dir` relative to `CARGO_MANIFEST_DIR`, which for
/// these tests is trybuild's generated project in
/// `<target>/tests/trybuild/rullst-macros`, so the fixture catalogs are
/// copied there first.
fn copy_i18n_fixtures() {
    let target = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .parent()
        .expect("CARGO_TARGET_TMPDIR is inside the target directory");
    let destination = target.join("tests/trybuild/rullst-macros/tests/i18n");
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/i18n");
    for case in fs::read_dir(&source).expect("fixture directory") {
        let case = case.expect("fixture entry").path();
        let Some(name) = case.file_name() else {
            continue;
        };
        let copy = destination.join(name);
        fs::create_dir_all(&copy).expect("create fixture copy");
        for file in fs::read_dir(&case).expect("fixture case") {
            let file = file.expect("fixture file").path();
            if let Some(file_name) = file.file_name() {
                fs::copy(&file, copy.join(file_name)).expect("copy fixture");
            }
        }
    }
}
