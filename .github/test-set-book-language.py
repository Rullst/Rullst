#!/usr/bin/env python3
"""Offline tests of .github/set-book-language.py on a small built book."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("set_book_language", HERE / "set-book-language.py")
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("cannot import set-book-language.py")
LANGUAGE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(LANGUAGE)

# The opening of a page as mdBook 0.5 writes it.
PAGE = ('<!DOCTYPE HTML>\n<html lang="en" class="light sidebar-visible" dir="ltr">\n'
        '<head><title>{title}</title></head>\n<body><p lang="en">{title}</p></body>\n</html>\n')


class BookLanguageTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        root = Path(self.directory.name)
        self.book, self.source = root / "book", root / "src"
        self.book_toml = root / "book.toml"
        self.book_toml.write_text('[book]\nlanguage = "en"\ntitle = "Test"\n', encoding="utf-8")
        for chapter in ("index", "1-getting-started", "pt-BR/index", "pt-BR/tutorials/01-hello-world"):
            (self.source / chapter).parent.mkdir(parents=True, exist_ok=True)
            (self.source / f"{chapter}.md").write_text(f"# {chapter}\n", encoding="utf-8")
            (self.book / chapter).parent.mkdir(parents=True, exist_ok=True)
            (self.book / f"{chapter}.html").write_text(PAGE.format(title=chapter), encoding="utf-8")
        (self.book / "print.html").write_text(PAGE.format(title="print"), encoding="utf-8")

    def tearDown(self):
        self.directory.cleanup()

    def run_script(self, *extra: str) -> int:
        return LANGUAGE.run(self.book, self.source, self.book_toml, check_only="--check" in extra)

    def declared(self, page: str) -> str | None:
        return LANGUAGE.declared_language((self.book / page).read_text(encoding="utf-8"))

    def test_the_translated_pages_declare_pt_br_and_the_others_keep_english(self):
        self.assertEqual(self.run_script("--check"), 1)
        self.assertEqual(self.run_script(), 0)
        self.assertEqual(self.declared("pt-BR/index.html"), "pt-BR")
        self.assertEqual(self.declared("pt-BR/tutorials/01-hello-world.html"), "pt-BR")
        for page in ("index.html", "1-getting-started.html", "print.html"):
            self.assertEqual(self.declared(page), "en", page)
        html = (self.book / "pt-BR/index.html").read_text(encoding="utf-8")
        # Only the <html> element changes, with its other attributes.
        self.assertIn('<html lang="pt-BR" class="light sidebar-visible" dir="ltr">', html)
        self.assertIn('<p lang="en">', html)
        self.assertEqual(self.run_script("--check"), 0)
        # A second run changes nothing.
        self.assertEqual(LANGUAGE.apply(self.book, "en"), 0)

    def test_a_missing_page_or_attribute_or_a_wrong_language_fails(self):
        self.assertEqual(self.run_script(), 0)
        (self.book / "1-getting-started.html").write_text(
            PAGE.format(title="x").replace('lang="en"', 'lang="pt-BR"', 1), encoding="utf-8")
        self.assertEqual(LANGUAGE.check(self.book, self.source, "en"),
                         ["1-getting-started.html declares lang='pt-BR', not 'en'"])
        (self.book / "1-getting-started.html").write_text(PAGE.format(title="x"), encoding="utf-8")
        (self.book / "pt-BR/tutorials/01-hello-world.html").unlink()
        self.assertEqual(LANGUAGE.check(self.book, self.source, "en"),
                         ["pt-BR/tutorials/01-hello-world.html was not built from "
                          "pt-BR/tutorials/01-hello-world.md"])
        (self.book / "pt-BR/index.html").write_text("<html><body></body></html>", encoding="utf-8")
        with self.assertRaises(LANGUAGE.Failure):
            LANGUAGE.apply(self.book, "en")
        self.assertEqual(self.run_script(), 1)

    def test_a_book_without_translations_fails_the_check(self):
        for path in sorted((self.source / "pt-BR").rglob("*.md")):
            path.unlink()
        self.assertEqual(LANGUAGE.check(self.book, self.source, "en")[-1], "src/pt-BR has no translated page")

    def test_the_repository_book_language_is_english(self):
        self.assertEqual(LANGUAGE.book_language(HERE.parent / "docs" / "book.toml"), "en")
        self.assertEqual(LANGUAGE.TRANSLATED, {"pt-BR": "pt-BR"})
        self.assertEqual(LANGUAGE.DOCS, HERE.parent / "docs")

    def test_a_missing_book_is_reported(self):
        self.assertEqual(LANGUAGE.run(self.book / "missing", self.source, self.book_toml, check_only=True), 1)


if __name__ == "__main__":
    unittest.main()
