use std::{collections::BTreeSet, fs, path::PathBuf};

fn aggregated_markdown_paths(aggregator: &str) -> Vec<String> {
    aggregator
        .lines()
        .filter_map(|line| {
            let marker = "../../docs/src/";
            let start = line.find(marker)? + marker.len();
            let remainder = &line[start..];
            let end = remainder.find('"')?;
            Some(remainder[..end].to_string())
        })
        .collect()
}

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

#[test]
fn every_public_reference_with_rust_is_included_in_the_doctest_aggregator() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let docs_dir = manifest_dir.join("../docs/src");
    let crates_dir = docs_dir.join("crates");
    let aggregator = fs::read_to_string(manifest_dir.join("src/book_doctests.rs"))
        .expect("the public-guide doctest aggregator must be readable");

    let mut expected = BTreeSet::new();
    for (directory, prefix) in [(&docs_dir, ""), (&crates_dir, "crates/")] {
        for entry in fs::read_dir(directory)
            .expect("the public-guide directory must be readable")
            .map(|entry| entry.expect("public-guide entries must be readable"))
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "md")
            })
        {
            let contents = fs::read_to_string(entry.path())
                .expect("public Markdown guides must be valid UTF-8");
            if contents.contains("```rust") {
                let filename = entry
                    .file_name()
                    .into_string()
                    .expect("public-guide filenames must be valid UTF-8");
                expected.insert(format!("{prefix}{filename}"));
            }
        }
    }

    let included = aggregated_markdown_paths(&aggregator)
        .into_iter()
        .filter(|path| !path.starts_with("tutorials/"))
        .collect::<Vec<_>>();
    let unique = included.iter().cloned().collect::<BTreeSet<_>>();

    assert_eq!(
        included.len(),
        unique.len(),
        "public reference guides must not be included more than once"
    );
    assert_eq!(
        unique, expected,
        "every public non-tutorial guide containing Rust must be doctested exactly once"
    );
}
