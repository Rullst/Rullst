//! Keeps the `PAGES` list of the generated smoke test (`src/security_tests.rs`,
//! written by `cargo rullst new` since 13.0.0-alpha.5) in step with the pages a
//! generator adds, so `cargo test` and `cargo rullst add browser-tests` check
//! them too.

/// The generated smoke test.
pub(crate) const FILE: &str = "src/security_tests.rs";
const LIST: &str = "const PAGES: &[(&str, u16)] = &[";

/// `source` with every `(path, status)` it does not list yet appended to its
/// `PAGES`; `None` when there is no such list or nothing is missing. A path
/// already listed keeps the status written there.
pub(crate) fn with_pages(source: &str, pages: &[(&str, u16)]) -> Option<String> {
    let start = source.find(LIST)?;
    let end = start + source.get(start..)?.find("\n];")?;
    let list = source.get(start..end)?;
    let missing: Vec<String> = pages
        .iter()
        .filter(|(path, _)| !list.contains(&format!("(\"{path}\",")))
        .map(|(path, status)| format!("\n    (\"{path}\", {status}),"))
        .collect();
    if missing.is_empty() {
        return None;
    }
    let mut updated = source.to_owned();
    updated.insert_str(end, &missing.concat());
    Some(updated)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMOKE: &str = "const PAGES: &[(&str, u16)] = &[\n    (\"/\", 200),\n    (\"/dashboard\", 303),\n];\n\nfn other() {}\n";

    #[test]
    fn new_pages_are_appended_once_and_listed_ones_are_kept() {
        let updated = with_pages(SMOKE, &[("/offline", 200), ("/dashboard", 200)]).unwrap();
        assert_eq!(
            updated,
            "const PAGES: &[(&str, u16)] = &[\n    (\"/\", 200),\n    (\"/dashboard\", 303),\n    (\"/offline\", 200),\n];\n\nfn other() {}\n"
        );
        assert_eq!(with_pages(&updated, &[("/offline", 200)]), None);
        let two = with_pages(SMOKE, &[("/a", 303), ("/b", 200)]).unwrap();
        assert!(two.contains("(\"/dashboard\", 303),\n    (\"/a\", 303),\n    (\"/b\", 200),\n];"));
    }

    #[test]
    fn a_project_without_the_list_is_left_alone() {
        assert_eq!(with_pages("fn main() {}\n", &[("/offline", 200)]), None);
        assert_eq!(
            with_pages("const PAGES: &[(&str, u16)] = &[", &[("/x", 200)]),
            None
        );
        // A path that only starts like a listed one is still added.
        assert!(with_pages(SMOKE, &[("/dash", 303)]).is_some());
    }
}
