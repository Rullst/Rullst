#!/usr/bin/env python3
"""Check the generated llms.txt and llms-full.txt against the book.

Run after `mdbook build docs` so every link can be resolved to a built page;
without docs/book the link check is skipped unless RULLST_LLMS_REQUIRE_BOOK=1.
"""

import importlib.util
import os
from pathlib import Path
import re
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT / "docs"
BUILT = DOCS / "book"
SPEC = importlib.util.spec_from_file_location("generate_llms_txt", ROOT / ".github/generate-llms-txt.py")
GENERATOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GENERATOR)

INDEX_LIMIT = 128 * 1024
FULL_LIMIT = 8 * 1024 * 1024
SUMMARY_LINK = re.compile(r"\[[^\]]+\]\(([^)]+\.md)\)")


def summary_pages():
    pages = []
    for path in SUMMARY_LINK.findall((DOCS / "src/SUMMARY.md").read_text(encoding="utf-8")):
        if path not in pages:
            pages.append(path)
    return pages


class GeneratedFilesTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.index, cls.full = GENERATOR.generate(DOCS)

    def test_index_follows_the_llms_txt_shape(self):
        lines = self.index.splitlines()
        self.assertTrue(lines[0].startswith("# "))
        self.assertTrue(lines[2].startswith("> "))
        self.assertGreater(sum(line.startswith("## ") for line in lines), 3)
        for line in lines:
            if line.startswith("- "):
                self.assertRegex(line, r"^- \[[^\]]+\]\(https://rullst\.github\.io/Rullst/book/\S+\.html\)")

    def test_order_follows_summary(self):
        expected = [GENERATOR.page_url(page) for page in summary_pages()]
        linked = re.findall(r"^- \[[^\]]+\]\((\S+)\)", self.index, re.M)
        self.assertEqual(list(dict.fromkeys(linked)), expected)
        sources = re.findall(r"^Source: (\S+)$", self.full, re.M)
        self.assertEqual(sources, expected)

    def test_every_page_starts_with_its_title_and_source(self):
        headers = re.findall(r"^---\n\n# .+\n\nSource: (\S+)\n\n", self.full, re.M)
        self.assertEqual(headers, [GENERATOR.page_url(page) for page in summary_pages()])

    def test_no_directive_or_comment_remains(self):
        for name, text in (("llms.txt", self.index), ("llms-full.txt", self.full)):
            self.assertNotIn("{{#include", text, name)
            self.assertNotIn("<!--", text, name)

    def test_size_is_bounded(self):
        self.assertLess(len(self.index.encode()), INDEX_LIMIT)
        self.assertLess(len(self.full.encode()), FULL_LIMIT)

    def test_output_is_deterministic(self):
        outputs = []
        for _ in range(2):
            with tempfile.TemporaryDirectory(prefix="rullst-llms-") as temp:
                self.assertEqual(GENERATOR.main(["--docs", str(DOCS), "--out", temp]), 0)
                outputs.append(tuple((Path(temp) / name).read_bytes()
                                     for name in ("llms.txt", "llms-full.txt")))
        self.assertEqual(outputs[0], outputs[1])
        self.assertEqual(outputs[0], (self.index.encode(), self.full.encode()))

    def test_every_link_resolves_to_a_built_page(self):
        if not (BUILT / "index.html").is_file():
            if os.environ.get("RULLST_LLMS_REQUIRE_BOOK") == "1":
                self.fail("docs/book is missing; run `mdbook build docs` first")
            self.skipTest("docs/book is not built")
        links = set(re.findall(r"\((https://rullst\.github\.io/Rullst/book/[^)#\s]+)", self.index))
        links |= set(re.findall(r"^Source: (\S+)$", self.full, re.M))
        for link in sorted(links):
            built = BUILT / link.removeprefix(GENERATOR.BOOK_URL)
            self.assertTrue(built.is_file(), f"{link} has no built page")


class FixtureBookTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory(prefix="rullst-llms-fixture-")
        self.addCleanup(temp.cleanup)
        self.docs = Path(temp.name) / "docs"
        source = self.docs / "src"
        (source / "guide").mkdir(parents=True)
        (source / "SUMMARY.md").write_text(
            "# Summary\n\n- [Intro](intro.md)\n\n# 🚀 Part One\n\n"
            "- [**Guide**](guide/page.md)\n  - [Draft]()\n---\n- [Intro again](intro.md)\n"
        )
        (source / "intro.md").write_text(
            "# Intro\n\n**Goal:** say hello. Then more.\n\n<!-- hidden -->\n"
            "See [the guide](guide/page.md#run) and [notes](../../NOTES.txt).\n"
            "```text\n[kept](guide/page.md)\n```\n"
        )
        (source / "guide/code.rs").write_text(
            "fn hidden() {}\n// ANCHOR: main\nfn main() {}\n// ANCHOR_END: main\n"
        )
        (source / "guide/page.md").write_text(
            "# Page\n\nRun it.\n\n```rust\n{{#include code.rs:main}}\n{{#include code.rs:1}}\n```\n"
            "\n{{#include ../../../ROOT.md}}\n"
        )
        (self.docs.parent / "NOTES.txt").write_text("notes\n")
        (self.docs.parent / "ROOT.md").write_text("[back](docs/src/intro.md)\n")

    def test_parts_includes_and_links(self):
        index, full = GENERATOR.generate(self.docs)
        self.assertIn("## Introduction\n\n- [Intro](https://rullst.github.io/Rullst/book/intro.html)"
                      ": Goal: say hello.", index)
        self.assertIn("## Part One\n\n- [Guide](https://rullst.github.io/Rullst/book/guide/page.html)", index)
        self.assertNotIn("Draft", index)
        self.assertEqual(full.count("Source: https://rullst.github.io/Rullst/book/intro.html"), 1)
        self.assertIn("```rust\nfn main() {}\nfn hidden() {}\n```", full)
        self.assertIn("(https://rullst.github.io/Rullst/book/guide/page.html#run)", full)
        self.assertIn("(https://github.com/Rullst/Rullst/blob/main/NOTES.txt)", full)
        self.assertIn("[kept](guide/page.md)", full)
        self.assertIn("[back](https://rullst.github.io/Rullst/book/intro.html)", full)
        self.assertNotIn("hidden -->", full)

    def test_missing_include_fails(self):
        (self.docs / "src/intro.md").write_text("# Intro\n\n{{#include missing.md}}\n")
        with self.assertRaises(ValueError):
            GENERATOR.generate(self.docs)


if __name__ == "__main__":
    unittest.main()
