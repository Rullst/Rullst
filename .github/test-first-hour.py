#!/usr/bin/env python3
"""Offline checks of the first-hour markers, commands and edits (no builds)."""

import importlib.util
from pathlib import Path
import re
import sys
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import first_hour_checks as checks  # noqa: E402
import first_hour_docs as docs  # noqa: E402
from first_hour_docs import MarkerError  # noqa: E402

SPEC = importlib.util.spec_from_file_location("check_first_hour", HERE / "check-first-hour.py")
RUNNER = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = RUNNER  # dataclasses resolve annotations through it
SPEC.loader.exec_module(RUNNER)
BLANK = HERE.parent / "cargo-rullst/src/blueprints/blank"
VERSION = RUNNER.candidate_version()
CRATES = docs.release_crates()


def page(*lines: str) -> str:
    return "\n".join(lines) + "\n"


def block(page_name: str, step: str):
    return next(b for b in docs.load()[page_name] if b.step == step)


class Session:
    """The hook API of check-first-hour.py's Session, on a scratch directory."""

    def __init__(self, cwd: Path):
        self.cwd = cwd


class RepositoryTests(unittest.TestCase):
    def test_every_journey_page_is_marked_and_every_hook_has_its_block(self):
        pages = RUNNER.validate(docs.load(), VERSION)
        self.assertEqual(set(pages), {p for journey in docs.JOURNEYS.values() for p in journey})
        for key in checks.APPLY:
            self.assertIn(key, {(b.page, b.step) for b in pages[key[0]] if b.action == "apply"})

    def test_documented_installs_name_the_candidate(self):
        installs = [tokens for blocks in docs.load().values() for b in blocks
                    if b.language in docs.SHELL for tokens in docs.commands(b.code)
                    if tokens[:3] == ["cargo", "install", "cargo-rullst"]]
        self.assertGreaterEqual(len(installs), 3)
        for tokens in installs:
            self.assertIn(f"={VERSION}", tokens)

    def test_shop_edits_apply_to_the_generated_starter(self):
        main = (BLANK / "src/main.rs.template").read_text()
        main = main.replace("__ARTISAN_CALL__", (BLANK / "fragments/artisan.rs.template").read_text())
        main = docs.edit(block(checks.CLI_GENERATORS, "mount"), main)
        self.assertIn('        get("/" => home),\n'
                      '        get("/products" => controllers::products_controller::index),\n', main)
        main = docs.edit(block(checks.MIGRATIONS, "artisan"), main)
        self.assertIn("    rullst::artisan!(\n        crate::migrations::get_migrations(),\n"
                      "        crate::seeds::get_seeders()\n    );\n", main)
        with tempfile.TemporaryDirectory() as directory:
            session = Session(Path(directory))
            (session.cwd / "src").mkdir()
            (session.cwd / "src/main.rs").write_text(main)
            for step in ("auth-routes", "password-routes"):
                checks.wire_routes(session, block(checks.AUTHENTICATION, step))
            wired = (session.cwd / "src/main.rs").read_text()
        router = wired.split("fn router() -> Router {", 1)[1]
        self.assertLess(router.index("let public_auth"), router.index("let password_change"))
        self.assertLess(router.index("let password_change"), router.index("\n    routes![\n"))
        merges = re.findall(r"\.merge_axum\((\w+)\.into_axum\(\)\)", router.split("csrf_middleware")[0])
        self.assertEqual(merges[-4:], ["public_auth", "signed_in", "password_reset", "password_change"])

    def test_migration_edits_keep_the_generated_names(self):
        generated = (HERE.parent / "cargo-rullst/src/generators/migration.rs").read_text()
        template = re.search(r'r#"(use rullst::db::schema.*?)"#', generated, re.S)[1]
        products = template.replace("{{", "{").replace("}}", "}").replace(
            "{file_stem}", "m20261030101010_create_products").replace("{table_name}", "products")
        edited = docs.edit(block(checks.ACTIVE_RECORD, "columns"), products)
        self.assertIn('        Schema::create("products", |table| {\n            table.id();\n'
                      '            table.string("name").not_null();', edited)
        self.assertIn("        }).await\n    }\n\n    async fn down", edited)
        catalog = products.replace("m20261030101010_create_products", "m20261030111111_create_catalog_items_table")
        edited = docs.edit(block(checks.MIGRATIONS, "catalog-migration"), catalog)
        self.assertIn('"m20261030111111_create_catalog_items_table"', edited)
        self.assertNotIn("m20260901000000", edited)


class ParserTests(unittest.TestCase):
    def test_markers_bind_to_the_next_fence_in_reading_order(self):
        blocks = docs.parse_page("p.md", page(
            "```bash", "<!-- first-hour: ignored -->", "```",
            "<!-- first-hour: one new-terminal -->", "```bash", "cargo new a \\", "  --bin  # x", "```",
            "<!-- first-hour: two output -->", "````json", "{}", "````"))
        self.assertEqual([(b.step, b.line, b.language) for b in blocks],
                         [("one", 5, "bash"), ("two", 10, "json")])
        self.assertEqual(docs.commands(blocks[0].code), [["cargo", "new", "a", "--bin"]])
        self.assertEqual(blocks[0].options, {"new-terminal": True})

    def test_broken_markers_fail(self):
        cases = {
            "no fence": page("<!-- first-hour: a -->", "", "```bash", "x", "```"),
            "typo": page("<!-- first-hour a -->", "```bash", "x", "```"),
            "unknown option": page("<!-- first-hour: a fast -->", "```bash", "x", "```"),
            "flag with value": page("<!-- first-hour: a new-terminal=yes -->", "```bash", "x", "```"),
            "two actions": page("<!-- first-hour: a output apply -->", "```text", "x", "```"),
            "missing anchor": page("<!-- first-hour: a insert=x.rs -->", "```rust", "x", "```"),
            "shell action": page("<!-- first-hour: a write=x -->", "```bash", "x", "```"),
            "hidden lines": page("<!-- first-hour: a write=x.rs -->", "```rust", "# fn x() {}", "```"),
            "duplicate": page("<!-- first-hour: a -->", "```bash", "x", "```",
                              "<!-- first-hour: a -->", "```bash", "y", "```"),
            "unclosed": page("<!-- first-hour: a -->", "```bash", "x"),
        }
        for name, text in cases.items():
            with self.subTest(name), self.assertRaises(MarkerError):
                docs.parse_page("p.md", text)

    def test_release_crates_need_the_exact_candidate(self):
        good = [["cargo", "install", "cargo-rullst", "--version", f"={VERSION}", "--locked"],
                ["cargo", "add", f"rullst@={VERSION}"], ["cargo", "add", "tokio", "--features", "full"]]
        for tokens in good:
            docs.check_command(tokens, VERSION, CRATES)
        bad = [["cargo", "install", "cargo-rullst"], ["cargo", "install", "cargo-rullst", "--version", "^13"],
               ["cargo", "add", "rullst"], ["cargo", "add", f"rullst@{VERSION}"],
               ["cargo", "install", "rullst-studio", "--version", f"={VERSION}"]]
        for tokens in bad:
            with self.subTest(tokens), self.assertRaises(MarkerError):
                docs.check_command(tokens, VERSION, CRATES)

    def test_edits_fail_when_their_anchor_is_gone(self):
        marked = docs.parse_page("p.md", page(
            "<!-- first-hour: a replace=x.rs from=start through=end -->", "```rust", "new", "```"))[0]
        self.assertEqual(docs.edit(marked, page("a", "  start", "  b", "  end", "c")), page("a", "  new", "c"))
        with self.assertRaises(MarkerError):
            docs.edit(marked, page("a", "end"))
        with self.assertRaises(MarkerError):
            docs.edit(marked, None)


if __name__ == "__main__":
    unittest.main()
