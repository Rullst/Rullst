#!/usr/bin/env python3
"""Offline tests of .github/check-translations.py on a temporary repository."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("check_translations", HERE / "check-translations.py")
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("cannot import check-translations.py")
CHECK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECK)

COMMIT = "0123456789abcdef0123456789abcdef01234567"
ENGLISH = """# Tutorial

> 🇧🇷 [Leia esta página em português](../pt-BR/tutorials/page.md).

Run it:

<!-- first-hour: run -->
```bash
cargo run
```

1. In a list:

   ```rust
   fn main() {}
   ```
"""
TRANSLATED = f"""# Tutorial

> **Tradução** da página em inglês [`page.md`](../../tutorials/page.md), feita a partir do commit [`{COMMIT[:9]}`](https://github.com/Rullst/Rullst/commit/{COMMIT}). Se as duas versões divergirem, vale a versão em inglês.

Execute:

```bash
cargo run
```

1. Em uma lista:

   ```rust
   fn main() {{}}
   ```
"""
README = """# Project

## ⚡ Quickstart

```bash
cargo install x
```

· 🇧🇷 [Em português](https://example.invalid/pt-BR/index.html)

## Other

```bash
not translated
```
"""


class CheckTranslations(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="check-translations-")
        self.root = Path(self.temp.name)
        self.saved = CHECK.TRANSLATIONS
        CHECK.TRANSLATIONS = {
            "docs/src/pt-BR/tutorials/page.md": ("docs/src/tutorials/page.md", None, "../pt-BR/tutorials/page.md"),
            "docs/src/pt-BR/index.md": ("README.md", "## ⚡ Quickstart", "https://example.invalid/pt-BR/index.html"),
        }
        self.write("docs/src/tutorials/page.md", ENGLISH)
        self.write("docs/src/pt-BR/tutorials/page.md", TRANSLATED)
        self.write("README.md", README)
        self.write("docs/src/pt-BR/index.md", TRANSLATED.replace(
            "page.md`](../../tutorials/page.md)", "README.md`](../../../README.md)").split("Execute:")[0]
            + "```bash\ncargo install x\n```\n")
        self.write("docs/src/SUMMARY.md", "- [Primeira hora](pt-BR/index.md)\n  - [Tutorial](pt-BR/tutorials/page.md)\n")

    def tearDown(self):
        CHECK.TRANSLATIONS = self.saved
        self.temp.cleanup()

    def write(self, relative: str, text: str) -> None:
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    def failure(self) -> str:
        with self.assertRaises(CHECK.Failure) as raised:
            CHECK.check(self.root)
        return str(raised.exception)

    def test_identical_blocks_notes_and_links_pass(self):
        self.assertEqual(CHECK.check(self.root), [])

    def test_a_changed_or_missing_block_fails(self):
        self.write("docs/src/pt-BR/tutorials/page.md", TRANSLATED.replace("cargo run", "cargo run --release"))
        self.assertIn("code block 1 differs at its line 2", self.failure())
        self.write("docs/src/pt-BR/tutorials/page.md", TRANSLATED.replace("   ```rust", "```rust", 1))
        self.assertIn("code block 2 differs at its line 1", self.failure())
        self.write("docs/src/pt-BR/tutorials/page.md", TRANSLATED.split("1. Em")[0])
        self.assertIn("1 code blocks, the English source has 2", self.failure())

    def test_only_the_readme_section_is_compared(self):
        self.assertEqual(CHECK.code_blocks(CHECK.section(README, "## ⚡ Quickstart")),
                         [["```bash", "cargo install x", "```"]])
        self.write("README.md", README.replace("## ⚡ Quickstart", "## Quickstart"))
        self.assertIn("has no line `## ⚡ Quickstart`", self.failure())

    def test_the_note_must_link_the_source_and_name_the_full_commit(self):
        self.write("docs/src/pt-BR/tutorials/page.md", TRANSLATED.replace(f"/commit/{COMMIT}", "/commit/0123456"))
        self.assertIn("translation note", self.failure())
        self.write("docs/src/pt-BR/tutorials/page.md", TRANSLATED.replace("../../tutorials/page.md", "page.md"))
        self.assertIn("linking `../../tutorials/page.md`", self.failure())
        self.write("docs/src/pt-BR/tutorials/page.md", TRANSLATED.replace(f"[`{COMMIT[:9]}`]", "[`fedcba987`]"))
        self.assertIn("translation note", self.failure())

    def test_links_listing_and_inventory(self):
        self.write("docs/src/tutorials/page.md", ENGLISH.replace("> 🇧🇷 [Leia", "> [Leia").replace(
            "](../pt-BR/tutorials/page.md)", "](elsewhere.md)"))
        self.assertIn("no longer links its translation", self.failure())
        self.write("docs/src/tutorials/page.md", ENGLISH)
        self.write("docs/src/SUMMARY.md", "- [Primeira hora](pt-BR/index.md)\n")
        self.assertIn("does not list docs/src/pt-BR/tutorials/page.md", self.failure())
        self.write("docs/src/pt-BR/other.md", "# Outra\n")
        self.assertIn("not in TRANSLATIONS: ['docs/src/pt-BR/other.md']", self.failure())

    def test_a_source_changed_since_the_commit_gets_a_notice(self):
        def git(*args: str) -> str:
            return subprocess.run(["git", *args], cwd=self.root, check=True, capture_output=True,
                                  text=True).stdout.strip()

        git("init", "-q")
        git("-c", "user.name=t", "-c", "user.email=t@example.invalid", "commit", "-q", "--allow-empty", "-m", "base")
        # The English page without its translation link, as at translation time.
        self.write("docs/src/tutorials/page.md", ENGLISH.replace(
            "> 🇧🇷 [Leia esta página em português](../pt-BR/tutorials/page.md).\n\n", ""))
        git("add", "-A")
        git("-c", "user.name=t", "-c", "user.email=t@example.invalid", "commit", "-q", "-m", "english")
        commit = git("rev-parse", "HEAD")
        for page in ("docs/src/pt-BR/tutorials/page.md", "docs/src/pt-BR/index.md"):
            text = (self.root / page).read_text(encoding="utf-8")
            self.write(page, text.replace(COMMIT, commit).replace(f"[`{COMMIT[:9]}`]", f"[`{commit[:9]}`]"))
        self.write("docs/src/tutorials/page.md", ENGLISH)
        self.assertEqual(CHECK.check(self.root), [], "adding the translation link is not a change")
        self.write("docs/src/tutorials/page.md", ENGLISH.replace("Run it:", "Run it now:"))
        notices = CHECK.check(self.root)
        self.assertEqual(len(notices), 1)
        self.assertIn("docs/src/pt-BR/tutorials/page.md: docs/src/tutorials/page.md changed since", notices[0])


if __name__ == "__main__":
    unittest.main()
