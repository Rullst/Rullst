use std::{collections::BTreeSet, fs, path::PathBuf};

#[test]
fn every_public_tutorial_is_included_in_the_doctest_aggregator() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let tutorial_dir = manifest_dir.join("../docs/src/tutorials");
    let aggregator_path = manifest_dir.join("src/book_doctests.rs");
    let aggregator = fs::read_to_string(&aggregator_path)
        .expect("the tutorial doctest aggregator must be readable");

    let tutorial_files = fs::read_dir(&tutorial_dir)
        .expect("the public tutorial directory must be readable")
        .map(|entry| entry.expect("tutorial directory entries must be readable"))
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "md")
        })
        .map(|entry| {
            entry
                .file_name()
                .into_string()
                .expect("tutorial filenames must be valid UTF-8")
        })
        .collect::<BTreeSet<_>>();

    assert!(
        !tutorial_files.is_empty(),
        "at least one tutorial must exist"
    );

    for filename in &tutorial_files {
        let include_path = format!("../../docs/src/tutorials/{filename}");
        assert_eq!(
            aggregator.matches(&include_path).count(),
            1,
            "{filename} must appear exactly once in the doctest aggregator"
        );
    }

    assert_eq!(
        aggregator.matches("../../docs/src/tutorials/").count(),
        tutorial_files.len(),
        "the doctest aggregator must not contain stale or duplicate tutorial paths"
    );
}
