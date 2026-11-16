#!/usr/bin/env python3
"""Declare the language of the book's translated pages, and check it.

mdBook writes the `language` of docs/book.toml (`en`) into the `<html lang>`
of every page, so screen readers would read the Brazilian Portuguese pages of
docs/src/pt-BR/ in English. Run after `mdbook build docs`, this sets
`lang="pt-BR"` on the `<html>` element of every page under <book>/pt-BR/, then
checks the whole book: each page under a directory of TRANSLATED declares that
language, every other page the book's own, and each translated Markdown page
has its HTML page. With --check it only checks, and changes nothing.

Usage: set-book-language.py [--book docs/book] [--check]
"""

from __future__ import annotations

import argparse
from pathlib import Path
import re
import sys
import tomllib

ROOT = Path(__file__).resolve().parent.parent
# Book directory of a translation -> its BCP 47 language tag.
TRANSLATED = {"pt-BR": "pt-BR"}
HTML_LANG = re.compile(r'(<html\b[^>]*?\blang=")([^"]*)(")', re.IGNORECASE)


class Failure(Exception):
    """A page does not declare the language it is written in."""


def book_language(book_toml: Path) -> str:
    return tomllib.loads(book_toml.read_text(encoding="utf-8"))["book"]["language"]


def expected_language(page: Path, book: Path, default: str) -> str:
    """The language of `page`: its translation's, or the book's."""
    top = page.relative_to(book).parts[0]
    return TRANSLATED.get(top, default)


def declared_language(html: str) -> str | None:
    match = HTML_LANG.search(html)
    return match.group(2) if match else None


def set_language(html: str, language: str) -> str:
    """`html` with `language` in the `lang` of its `<html>` element."""
    updated, count = HTML_LANG.subn(lambda match: match.group(1) + language + match.group(3), html, count=1)
    if count != 1:
        raise Failure("no <html lang> attribute")
    return updated


def pages(book: Path) -> list[Path]:
    return sorted(path for path in book.rglob("*.html") if path.is_file())


def apply(book: Path, default: str) -> int:
    """Sets the translated pages' language; returns how many changed."""
    changed = 0
    for page in pages(book):
        language = expected_language(page, book, default)
        if language == default:
            continue
        html = page.read_text(encoding="utf-8")
        try:
            updated = set_language(html, language)
        except Failure as error:
            raise Failure(f"{page.relative_to(book)}: {error}") from error
        if updated != html:
            page.write_text(updated, encoding="utf-8")
            changed += 1
    return changed


def check(book: Path, source: Path, default: str) -> list[str]:
    """Every page whose declared language is wrong, and translations without a page."""
    problems = []
    for page in pages(book):
        expected = expected_language(page, book, default)
        declared = declared_language(page.read_text(encoding="utf-8"))
        if declared != expected:
            problems.append(f"{page.relative_to(book)} declares lang={declared!r}, not {expected!r}")
    for directory in TRANSLATED:
        chapters = sorted((source / directory).rglob("*.md"))
        if not chapters:
            problems.append(f"{source.name}/{directory} has no translated page")
        for chapter in chapters:
            page = book / chapter.relative_to(source).with_suffix(".html")
            if not page.is_file():
                problems.append(f"{page.relative_to(book)} was not built from {chapter.relative_to(source)}")
    return problems


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("--book", type=Path, default=ROOT / "docs" / "book")
    parser.add_argument("--source", type=Path, default=ROOT / "docs" / "src")
    parser.add_argument("--book-toml", type=Path, default=ROOT / "docs" / "book.toml")
    parser.add_argument("--check", action="store_true", help="only check, change nothing")
    args = parser.parse_args(argv)
    book = args.book.resolve()
    if not book.is_dir():
        print(f"set-book-language: {book} does not exist; run `mdbook build docs` first", file=sys.stderr)
        return 1
    default = book_language(args.book_toml)
    try:
        if not args.check:
            changed = apply(book, default)
            print(f"set-book-language: declared the translated language on {changed} page(s)")
        problems = check(book, args.source.resolve(), default)
    except Failure as error:
        problems = [str(error)]
    for problem in problems:
        print(f"set-book-language: {problem}", file=sys.stderr)
    if not problems:
        print(f"set-book-language: every page declares its language ({', '.join(sorted(TRANSLATED))} and {default})")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
