//! Literals of non-exhaustive Rullst structs reached through `use super::*;`:
//! the package's module tree decides whether the name is Rullst's.

use super::*;

const LITERAL: &str = "V13-NON-EXHAUSTIVE-LITERAL";
const GLOB: &str = "V13-NON-EXHAUSTIVE-LITERAL-GLOB";

/// A test module that builds Connect's `HttpResponse` with a literal.
const TEST_MODULE: &str = "use super::*;\nfn ok(body: String) -> HttpResponse {\n    HttpResponse { status: 200, body }\n}\n";

/// The findings of these rules in a package made of `files`.
fn scan(files: &[(&str, &str)]) -> Vec<(String, &'static str, usize)> {
    let root = std::env::temp_dir().join(format!("rullst-rules-globs-{}", rand::random::<u64>()));
    for (relative, text) in files {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    let mut scan = SourceScan::default();
    scan_package(&root, &WorkspaceFacts::default(), &mut scan).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
    scan.findings
        .iter()
        .filter(|finding| [LITERAL, GLOB].contains(&finding.rule.code))
        .map(|finding| {
            let relative = finding.path.strip_prefix(&root).unwrap();
            (
                relative.to_string_lossy().replace('\\', "/"),
                finding.rule.code,
                finding.line,
            )
        })
        .collect()
}

fn found(path: &str, code: &'static str, line: usize) -> Vec<(String, &'static str, usize)> {
    vec![(path.to_string(), code, line)]
}

#[test]
fn a_test_module_reaching_rullst_through_its_parents_must_change() {
    // The trial upgrade's layout: `google_tests.rs` is a module of
    // `connected_tests.rs`, itself the `tests` module of `connected.rs`, and
    // only `connected_tests.rs` imports Connect's client types.
    let files = [
        ("src/lib.rs", "mod connected;\n"),
        (
            "src/connected.rs",
            "use rullst::connect::provider::ExchangeParams;\n\n#[cfg(test)]\n#[path = \"connected_tests.rs\"]\nmod tests;\n",
        ),
        (
            "src/connected_tests.rs",
            "use super::*;\nuse rullst::connect::client::{HttpClient, HttpRequest, HttpResponse};\n\n#[path = \"google_tests.rs\"]\nmod google;\n",
        ),
        ("src/google_tests.rs", TEST_MODULE),
    ];
    assert_eq!(scan(&files), found("src/google_tests.rs", LITERAL, 3));
}

#[test]
fn rullst_names_reached_through_globs_and_further_parents_must_change() {
    // A glob of the crate's path, two `use super::*;` links, a `mod.rs`
    // parent and an explicit `use super::HttpResponse;`.
    for parent in [
        "use rullst::connect::client::*;\nmod tests;\n",
        "use rullst_connect::client::HttpResponse as HttpResponse;\nmod tests;\n",
    ] {
        let files = [
            ("src/client.rs", parent),
            ("src/client/tests.rs", TEST_MODULE),
        ];
        assert_eq!(
            scan(&files),
            found("src/client/tests.rs", LITERAL, 3),
            "{parent}"
        );
    }
    let chain = [
        (
            "src/lib.rs",
            "use rullst::connect::client::HttpResponse;\nmod outer;\n",
        ),
        ("src/outer/mod.rs", "use super::*;\nmod inner;\n"),
        ("src/outer/inner.rs", TEST_MODULE),
    ];
    assert_eq!(scan(&chain), found("src/outer/inner.rs", LITERAL, 3));
    let explicit = TEST_MODULE.replace("use super::*;", "use super::*;\nuse super::HttpResponse;");
    let files = [
        (
            "src/main.rs",
            "use rullst::connect::client::HttpResponse;\n#[path = \"shared/../client_tests.rs\"]\nmod tests;\nfn main() {}\n",
        ),
        ("src/client_tests.rs", explicit.as_str()),
    ];
    assert_eq!(scan(&files), found("src/client_tests.rs", LITERAL, 4));
    // A tuple struct of the AI crate, called through the glob.
    let call = [
        (
            "src/rag.rs",
            "use rullst::ai::ai::rag::RagRetrievalError;\nmod tests;\n",
        ),
        (
            "src/rag/tests.rs",
            "use super::*;\nfn e() -> RagRetrievalError { RagRetrievalError(String::new()) }\n",
        ),
    ];
    assert_eq!(scan(&call), found("src/rag/tests.rs", LITERAL, 2));
}

#[test]
fn an_application_type_of_the_same_name_is_not_reported() {
    for parent in [
        // Defined by the parent module, beside an unrelated Connect import.
        "use rullst::connect::provider::ExchangeParams;\npub struct HttpResponse { pub status: u16, pub body: String }\nmod tests;\n",
        // Imported from another crate.
        "use my_http::HttpResponse;\nmod tests;\n",
        // Defined by the grandparent, which the parent reaches by its glob.
        "use super::*;\nmod tests;\n",
    ] {
        let files = [
            (
                "src/lib.rs",
                "pub struct HttpResponse { pub status: u16, pub body: String }\nmod http;\n",
            ),
            ("src/http.rs", parent),
            ("src/http/tests.rs", TEST_MODULE),
        ];
        assert_eq!(scan(&files), Vec::new(), "{parent}");
    }
}

#[test]
fn an_origin_the_tree_cannot_tell_stays_a_review_finding() {
    for (parent_path, parent) in [
        // The parent's own module glob, a `crate::` import, a glob of a Rullst
        // path that does not name Connect, and no `use` of the name at all.
        ("src/http.rs", "use crate::client::*;\nmod tests;\n"),
        (
            "src/http.rs",
            "use crate::client::HttpResponse;\nmod tests;\n",
        ),
        ("src/http.rs", "use rullst::prelude::*;\nmod tests;\n"),
        ("src/http.rs", "mod tests;\n"),
        // No module declares the test file.
        (
            "src/other.rs",
            "use rullst::connect::client::HttpResponse;\n",
        ),
    ] {
        let files = [(parent_path, parent), ("src/http/tests.rs", TEST_MODULE)];
        assert_eq!(
            scan(&files),
            found("src/http/tests.rs", GLOB, 3),
            "{parent}"
        );
    }
    // Two files declare the same module: the parent is ambiguous.
    let files = [
        (
            "src/lib.rs",
            "use rullst::connect::client::HttpResponse;\n#[path = \"http_tests.rs\"]\nmod a;\n#[path = \"http_tests.rs\"]\nmod b;\n",
        ),
        (
            "src/main.rs",
            "pub struct HttpResponse { pub status: u16, pub body: String }\n#[path = \"http_tests.rs\"]\nmod tests;\nfn main() {}\n",
        ),
        ("src/http_tests.rs", TEST_MODULE),
    ];
    assert_eq!(scan(&files), found("src/http_tests.rs", GLOB, 3));
}

#[test]
fn a_reviewed_marker_still_removes_the_finding() {
    let marked = TEST_MODULE.replace(
        "    HttpResponse { status: 200, body }",
        "    HttpResponse { status: 200, body } // rullst-upgrade: reviewed V13-NON-EXHAUSTIVE-LITERAL-GLOB",
    );
    let files = [
        (
            "src/client.rs",
            "use rullst::connect::client::HttpResponse;\nmod tests;\n",
        ),
        ("src/client/tests.rs", marked.as_str()),
    ];
    assert_eq!(scan(&files), Vec::new());
}

/// A chain of `links` modules below `src/lib.rs`: each file imports nothing
/// but `use super::*;`, the deepest is the test module, and `src/lib.rs`
/// imports the Connect type. A module file declares its children in a
/// directory named after it.
fn chain(links: usize) -> Vec<(String, String)> {
    let mut files = vec![(
        "src/lib.rs".to_string(),
        "use rullst::connect::client::HttpResponse;\nmod m1;\n".to_string(),
    )];
    let mut directory = "src".to_string();
    for link in 1..=links {
        let text = if link == links {
            TEST_MODULE.to_string()
        } else {
            format!("use super::*;\nmod m{};\n", link + 1)
        };
        files.push((format!("{directory}/m{link}.rs"), text));
        directory = format!("{directory}/m{link}");
    }
    files
}

fn scan_owned(files: &[(String, String)]) -> Vec<(String, &'static str, usize)> {
    let borrowed: Vec<(&str, &str)> = files
        .iter()
        .map(|(path, text)| (path.as_str(), text.as_str()))
        .collect();
    scan(&borrowed)
}

/// The path of the deepest module of `chain(links)`.
fn last_file(links: usize) -> String {
    let directories: Vec<String> = (1..links).map(|link| format!("m{link}")).collect();
    format!("src/{}/m{links}.rs", directories.join("/"))
}

#[test]
fn a_chain_of_globs_is_followed_for_sixteen_links_and_no_further() {
    // `src/lib.rs` is the 16th parent of `src/m16.rs`, and the 17th of
    // `src/m17.rs`.
    assert_eq!(scan_owned(&chain(16)), found(&last_file(16), LITERAL, 3));
    assert_eq!(scan_owned(&chain(17)), found(&last_file(17), GLOB, 3));
}

#[test]
fn a_module_that_is_its_own_parent_ends_the_search() {
    // `a.rs` declares itself as its module, so `use super::*;` leads back to
    // it (and nothing else declares it); the search stops at the depth limit instead of recursing for ever.
    let files = [(
        "src/a.rs",
        "use super::*;\n#[path = \"a.rs\"]\nmod again;\nfn ok(body: String) -> HttpResponse {\n    HttpResponse { status: 200, body }\n}\n",
    )];
    assert_eq!(scan(&files), found("src/a.rs", GLOB, 5));
}

#[test]
fn only_a_direct_super_import_follows_the_parent() {
    // `use super::other::HttpResponse;` names a sibling, not the parent's own
    // import, and `use super::HttpResponse;` is the parent's.
    let files = [
        (
            "src/lib.rs",
            "use rullst::connect::client::HttpResponse;\nmod http;\n",
        ),
        (
            "src/http.rs",
            "use super::other::HttpResponse;\nmod tests;\n",
        ),
        ("src/http/tests.rs", TEST_MODULE),
    ];
    assert_eq!(scan(&files), found("src/http/tests.rs", GLOB, 3));
}

#[test]
fn an_import_from_a_rullst_path_that_does_not_name_the_crate_is_unknown() {
    // Not Connect's path, so the name may be anything: the review stays.
    for import in [
        "use rullst::prelude::HttpResponse;",
        "use rullst_core::HttpResponse;",
        "use rullst_ai::HttpResponse;",
    ] {
        let files = [
            ("src/http.rs", format!("{import}\nmod tests;\n")),
            ("src/http/tests.rs", TEST_MODULE.to_string()),
        ];
        let borrowed: Vec<(&str, &str)> = files
            .iter()
            .map(|(path, text)| (*path, text.as_str()))
            .collect();
        assert_eq!(
            scan(&borrowed),
            found("src/http/tests.rs", GLOB, 3),
            "{import}"
        );
    }
}

#[test]
fn enums_unions_and_aliases_define_the_application_s_own_name() {
    for definition in [
        "pub enum HttpResponse { Ok }",
        "pub union HttpResponse { status: u16 }",
        "pub type HttpResponse = my_http::Response;",
    ] {
        let files = [
            ("src/http.rs", format!("{definition}\nmod tests;\n")),
            ("src/http/tests.rs", TEST_MODULE.to_string()),
        ];
        let borrowed: Vec<(&str, &str)> = files
            .iter()
            .map(|(path, text)| (*path, text.as_str()))
            .collect();
        assert_eq!(scan(&borrowed), Vec::new(), "{definition}");
    }
}

#[test]
fn an_inline_module_does_not_declare_a_file() {
    // `mod http { .. }` is in the file; `src/http.rs` belongs to no module.
    let files = [
        (
            "src/lib.rs",
            "use rullst::connect::client::HttpResponse;\nmod http {\n    pub fn inner() {}\n}\n",
        ),
        ("src/http.rs", TEST_MODULE),
    ];
    assert_eq!(scan(&files), found("src/http.rs", GLOB, 3));
}

#[test]
fn each_review_finding_is_resolved_on_its_own() {
    // Three test modules with their literal on the same line: one reaches the
    // Rullst type, one the application's own and one neither. Resolving one
    // must leave the other two as they are.
    let files = [
        ("src/lib.rs", "mod a;\nmod b;\nmod c;\n"),
        (
            "src/a.rs",
            "use rullst::connect::client::HttpResponse;\nmod tests;\n",
        ),
        ("src/a/tests.rs", TEST_MODULE),
        ("src/b.rs", "use crate::client::*;\nmod tests;\n"),
        ("src/b/tests.rs", TEST_MODULE),
        (
            "src/c.rs",
            "pub struct HttpResponse { pub status: u16, pub body: String }\nmod tests;\n",
        ),
        ("src/c/tests.rs", TEST_MODULE),
    ];
    assert_eq!(
        scan(&files),
        [
            ("src/a/tests.rs".to_string(), LITERAL, 3),
            ("src/b/tests.rs".to_string(), GLOB, 3),
        ]
    );
}
