#!/usr/bin/env python3
"""Tests for check-fail-closed.py and fail_closed_source.py."""

from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("check-fail-closed.py")
sys.path.insert(0, str(SCRIPT.parent))
SPEC = importlib.util.spec_from_file_location("check_fail_closed", SCRIPT)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)
SOURCE = sys.modules["fail_closed_source"]

UNLISTED_FALLBACK = """\
pub struct Client { mode: Mode }

impl Client {
    pub fn from_env() -> Self {
        let key = std::env::var("ACME_API_KEY").unwrap_or_default();
        if key.is_empty() || key.starts_with("mock_") {
            Self { mode: Mode::Mock }
        } else {
            Self { mode: Mode::Live(key) }
        }
    }
}
"""

REFUSING_SETTINGS = """\
pub fn configured() -> Result<Client, Error> {
    let key = rullst::config::project_setting("ACME_API_KEY")?.unwrap_or_default();
    if key.trim().is_empty() && environment()?.requires_secure_defaults() {
        return Err(Error::Missing("ACME_API_KEY"));
    }
    Ok(Client::new(key))
}
"""

TEST_ONLY = """\
pub fn live() {}

#[cfg(test)]
mod tests {
    #[test]
    fn mock() {
        assert!("mock_key".starts_with("mock_"));
    }
}

#[cfg(all(test, feature = "x"))]
fn helper() -> bool { std::env::var("ACME_TOKEN").unwrap_or_default().starts_with("mock_") }

#[cfg(test)]
mod support;

#[cfg(test)]
#[path = "elsewhere.rs"]
mod moved;

// A comment: key.starts_with("mock_") selects the mock.
/// Doc: `std::env::var("ACME_API_KEY").unwrap_or_default()`.
"""

TEST_ONLY_CODE = 'pub fn f(key: &str) -> bool { key.is_empty() || key.starts_with("mock_") }\n'


def write(root: Path, files: dict[str, str]) -> None:
    for name, content in files.items():
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")


def workspace(root: Path, files: dict[str, str], sites: list[dict]) -> None:
    write(root, {"Cargo.toml": '[workspace]\nmembers = ["acme", "cargo-rullst", "examples/demo"]\n', **files})
    write(root, {".github/fail-closed-sites.json": json.dumps({"sites": sites})})


def run(root: Path) -> tuple[int, str]:
    output = io.StringIO()
    with contextlib.redirect_stdout(output):
        status = MODULE.main([], root)
    return status, output.getvalue()


def entry(path: str, anchor: str, classification: str = "caller-provided-values", **extra: object) -> dict:
    return {"path": path, "anchor": anchor, "classification": classification, "reason": "test", **extra}


class SignalTests(unittest.TestCase):
    def signals(self, source: str) -> set[str]:
        code, _ = SOURCE.lex(source)
        return {name for name, pattern in MODULE.SIGNALS.items() if pattern.search(code)}

    def test_each_signal_finds_its_shape(self) -> None:
        for source, signal in [
            ('if key.starts_with("mock_") {}', "mock-prefix"),
            ('value.get(..5).is_some_and(|p| p.eq_ignore_ascii_case("mock_"))', "mock-prefix"),
            ("if url.starts_with(MOCK_PREFIX) {}", "mock-prefix"),
            ('if key == "mock" {}', "mock-prefix"),
            ("Self { inner: Inner::Offline(pool) }", "mock-variant"),
            ("if empty { Mode::Mock } else { Mode::Live }", "mock-variant"),
            ("let mode = ProviderMode::from_credential(&key);", "mode-helper"),
            ("require_credential(settings, &value, name)?;", "mode-helper"),
            ("if provider.credential_mode().is_mock() {}", "mode-helper"),
            ('let key = std::env::var("ACME_API_KEY").unwrap_or_default();', "empty-setting-default"),
            ('let url = setting("ACME_URL")?.unwrap_or_else(String::new);', "empty-setting-default"),
            ('let token = read("ACME_TOKEN")?.unwrap_or("");', "empty-setting-default"),
            ('providers.push(OpenAiProvider::new("mock_auto"));', "mock-credential"),
            ('let env = "ACME_URL=mock_local\\n";', "generated-mock-setting"),
            ('documented("ACME_API_KEY", "mock_acme")', "generated-mock-setting"),
            ('out.replace("__ACME_KEY__", "mock_acme")', "generated-mock-setting"),
            ("if key.is_empty() { return Ok(Self::offline()); }", "empty-selects-mock"),
            ('let path = env::var("P").unwrap_or_else(|_| "local.db".to_owned());', "local-store-default"),
            ('return "sqlite://db.sqlite".to_string();', "local-store-default"),
            ('"let key = std::env::var(\\"ACME_KEY\\").unwrap_or_default();"', "empty-setting-default"),
        ]:
            with self.subTest(source=source):
                self.assertIn(signal, self.signals(source))

    def test_ordinary_code_is_not_a_signal(self) -> None:
        for source in [
            'let path = std::env::var_os("PATH").unwrap_or_default();',
            'let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();',
            "let live = stripe_contract::credential_mode(api_key);",
            "if self.is_mock() { return fixture(); }",
            "match mode { Mode::Mock => 1, Mode::Live => 2 }",
            'json!({ "access_token": "mock_access_token" })',
            'fn credential_mode(value: &str) -> Mode { todo!() }',
            'if name.is_empty() { return Err(Error::Empty); }',
            "// key.starts_with(\"mock_\") in a comment",
            "/* nested /* */ std::env::var(\"ACME_KEY\").unwrap_or_default() */",
        ]:
            with self.subTest(source=source):
                self.assertEqual(self.signals(source), set())


class SourceTests(unittest.TestCase):
    def test_lexing_keeps_strings_and_offsets(self) -> None:
        source = 'let a = r#"// not a comment"#; let c = \'"\'; // gone\nfn f<\'a>(x: &\'a str) {}'
        code, skeleton = SOURCE.lex(source)
        self.assertEqual(len(code), len(source))
        self.assertIn("// not a comment", code)
        self.assertNotIn("gone", code)
        self.assertNotIn("not a comment", skeleton)
        self.assertEqual(SOURCE.scopes(skeleton)[0][2], "f")

    def test_items_are_named_with_their_type(self) -> None:
        source = (
            "impl<T: Send> Store for Wrapper<T> where T: Clone {\n"
            "    fn open(lookup: impl Fn(&str) -> Option<String>) -> Self { todo!() }\n"
            "}\n"
            "pub(crate) const DEFAULTS: &[&str] = &[\"a\"];\n"
            "fn outer() { fn inner() {} }\n"
        )
        _, skeleton = SOURCE.lex(source)
        spans = SOURCE.scopes(skeleton)
        self.assertEqual(SOURCE.enclosing(spans, source.index("todo"))[0], "Wrapper::open")
        self.assertEqual(SOURCE.enclosing(spans, source.index('"a"'))[0], "DEFAULTS")
        self.assertEqual(SOURCE.enclosing(spans, source.index("inner() {}") + 9)[0], "outer::inner")
        self.assertEqual(SOURCE.enclosing(spans, 0), ("", None))

    def test_cfg_test_predicates(self) -> None:
        self.assertTrue(SOURCE.test_only("test"))
        self.assertTrue(SOURCE.test_only('all(feature = "x", test)'))
        self.assertFalse(SOURCE.test_only('any(test, feature = "mock")'))
        self.assertFalse(SOURCE.test_only("not(test)"))


class WorkspaceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.root = Path(self.directory.name)

    def tearDown(self) -> None:
        self.directory.cleanup()

    def test_an_unlisted_fallback_fails(self) -> None:
        workspace(self.root, {"acme/src/lib.rs": UNLISTED_FALLBACK}, [])
        status, output = run(self.root)
        self.assertEqual(status, 1)
        self.assertIn("unlisted site: acme/src/lib.rs Client::from_env (", output)
        self.assertIn("empty-setting-default line 5", output)
        self.assertIn("AGENTS.md section 3.5", output)

    def test_a_listed_fallback_passes(self) -> None:
        workspace(
            self.root,
            {"acme/src/lib.rs": UNLISTED_FALLBACK},
            [
                entry(
                    "acme/src/lib.rs",
                    "Client::from_env",
                    "documented-exception",
                    docs=["acme/README.md"],
                )
            ],
        )
        write(self.root, {"acme/README.md": "`Client::from_env` selects the mock everywhere.\n"})
        self.assertEqual(
            run(self.root),
            (0, "All 1 offline-fallback sites are classified in .github/fail-closed-sites.json.\n"),
        )

    def test_a_stale_entry_fails(self) -> None:
        sites = [entry("acme/src/lib.rs", "Client::from_env")]
        workspace(self.root, {"acme/src/lib.rs": "pub fn live() {}\n"}, sites)
        status, output = run(self.root)
        self.assertEqual(status, 1)
        self.assertIn("stale entry (no signal matches it): acme/src/lib.rs Client::from_env", output)

    def test_test_code_comments_and_examples_are_ignored(self) -> None:
        workspace(
            self.root,
            {
                "acme/src/lib.rs": TEST_ONLY,
                "acme/src/support.rs": TEST_ONLY_CODE,
                "acme/src/elsewhere.rs": TEST_ONLY_CODE,
                "acme/src/only_tests.rs": "#![cfg(test)]\n" + TEST_ONLY_CODE,
                "acme/src/thing_tests.rs": TEST_ONLY_CODE,
                "acme/src/thing_test.rs": TEST_ONLY_CODE,
                "acme/src/tests.rs": TEST_ONLY_CODE,
                "acme/src/id_tests/verify.rs": TEST_ONLY_CODE,
                "acme/tests/contract.rs": TEST_ONLY_CODE,
                "acme/benches/bench.rs": TEST_ONLY_CODE,
                "acme/examples/demo.rs": TEST_ONLY_CODE,
                "acme/fuzz/fuzz_targets/a.rs": TEST_ONLY_CODE,
                "acme/target/debug/build/out.rs": TEST_ONLY_CODE,
                "examples/demo/src/main.rs": TEST_ONLY_CODE,
                "outside/src/lib.rs": TEST_ONLY_CODE,
            },
            [],
        )
        self.assertEqual(run(self.root)[0], 0)
        self.assertEqual(MODULE.find_sites(self.root), {})

    def test_a_feature_gated_mock_is_not_test_code(self) -> None:
        source = '#[cfg(any(test, feature = "mock"))]\n' + TEST_ONLY_CODE
        workspace(self.root, {"acme/src/lib.rs": source}, [])
        self.assertEqual(list(MODULE.find_sites(self.root)), [("acme/src/lib.rs", "f")])

    def test_templates_use_line_snippets_and_refusals_are_checked(self) -> None:
        template = "cargo-rullst/src/fragments/start.rs.template"
        sites = [
            entry(
                template,
                'project_setting("ACME_API_KEY")',
                "refuses-in-secure-environments",
                refusal="requires_secure_defaults",
            )
        ]
        workspace(self.root, {template: REFUSING_SETTINGS.split("\n", 1)[1]}, sites)
        self.assertEqual(run(self.root)[0], 0)
        weakened = REFUSING_SETTINGS.split("\n", 1)[1].replace("requires_secure_defaults", "is_development")
        write(self.root, {template: weakened})
        status, output = run(self.root)
        self.assertEqual(status, 1)
        self.assertIn("refusal missing:", output)

    def test_invalid_entries_fail(self) -> None:
        workspace(self.root, {"acme/src/lib.rs": UNLISTED_FALLBACK}, [])
        site = MODULE.find_sites(self.root)
        for bad, message in [
            (entry("acme/src/lib.rs", "Client::from_env", "fine"), "unknown classification 'fine'"),
            (
                entry("acme/src/lib.rs", "Client::from_env", "refuses-in-secure-environments"),
                "missing field 'refusal'",
            ),
            (entry("acme/src/lib.rs", "Client::from_env", docs=[]), "unexpected field 'docs'"),
            (
                entry("acme/src/lib.rs", "Client::from_env", "refuses-in-secure-environments", refusal=""),
                "refusal is not an identifier",
            ),
            (
                entry("acme/src/lib.rs", "Client::from_env", "documented-exception", docs=["missing.md"]),
                "doc missing.md does not exist",
            ),
        ]:
            with self.subTest(message=message):
                self.assertIn(
                    f"invalid entry: acme/src/lib.rs Client::from_env: {message}",
                    MODULE.compare(self.root, site, [bad]),
                )
        twice = [entry("acme/src/lib.rs", "Client::from_env")] * 2
        self.assertIn(
            "duplicate entry: acme/src/lib.rs Client::from_env", MODULE.compare(self.root, site, twice)
        )


class RepositoryTests(unittest.TestCase):
    def test_the_repository_matches_its_inventory(self) -> None:
        status, output = run(MODULE.ROOT)
        self.assertEqual(status, 0, output)


if __name__ == "__main__":
    unittest.main()
