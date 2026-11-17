#!/usr/bin/env python3
"""Offline tests for public-dependency-health.py (no network)."""

from __future__ import annotations

import contextlib
import datetime as dt
import importlib.util
import io
import json
import sys
import tarfile
import tempfile
import unittest
import urllib.error
from pathlib import Path
from unittest import mock

SCRIPT = Path(__file__).with_name("public-dependency-health.py")
SPEC = importlib.util.spec_from_file_location("public_dependency_health", SCRIPT)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
# Dataclasses look their module up while the class is created.
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)

TODAY = dt.date(2026, 11, 17)
UNMAINTAINED = """```toml
[advisory]
id = "RUSTSEC-2026-0543"
package = "chrono"
date = "2026-09-14"
informational = "unmaintained"
url = "https://github.com/chronotope/chrono/issues/1768"

[versions]
patched = []
```

# chrono is unmaintained

The chrono crate is unmaintained; all versions are affected.
"""
PATCHED = """```toml
[advisory]
id = "RUSTSEC-2020-0159"
package = "chrono"
date = "2020-11-10"

[versions]
patched = [">=0.4.20"]
```

# Potential segfault in `localtime_r` invocations
"""
WITHDRAWN = """```toml
[advisory]
id = "RUSTSEC-2021-0001"
package = "chrono"
date = "2021-01-01"
withdrawn = "2021-02-01"

[versions]
patched = []
```

# Withdrawn
"""


def crate(*versions: tuple[str, str] | tuple[str, str, bool], repository: str | None = None) -> dict:
    entries = []
    for entry in versions:
        number, date, yanked = (*entry, False) if len(entry) == 2 else entry
        entries.append({"num": number, "created_at": f"{date}T12:00:00Z", "yanked": yanked})
    return {"crate": {"repository": repository}, "versions": entries}


def repository(archived: bool = False) -> object:
    return MODULE.Repository("owner/name", "https://github.com/owner/name", archived, False, "2026-11-01")


def tarball(files: dict[str, str], commit: str = "233155b4025b854c68a4b199c0e32d7fd53291a2") -> bytes:
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode="w:gz", format=tarfile.PAX_FORMAT,
                      pax_headers={"comment": commit}) as archive:
        for name, text in files.items():
            data = text.encode()
            info = tarfile.TarInfo(f"rustsec-advisory-db-233155b/{name}")
            info.size = len(data)
            archive.addfile(info, io.BytesIO(data))
    return buffer.getvalue()


class Requirements(unittest.TestCase):
    def test_comparators_follow_cargo(self) -> None:
        cases = [
            (">=0.4.20", (0, 4, 20), True), (">=0.4.20", (0, 4, 19), False),
            (">= 1.2.3, < 1.3.0", (1, 2, 9), True), (">= 1.2.3, < 1.3.0", (1, 3, 0), False),
            ("^0.4.20", (0, 4, 45), True), ("^0.4.20", (0, 5, 0), False),
            ("^1.2", (1, 9, 0), True), ("^1.2", (2, 0, 0), False),
            ("^0.0.3", (0, 0, 3), True), ("^0.0.3", (0, 0, 4), False),
            ("^0.0", (0, 0, 9), True), ("^0.0", (0, 1, 0), False),
            ("1.2.3", (1, 5, 0), True), ("0.2", (0, 3, 0), False),
            ("~1.2.3", (1, 2, 9), True), ("~1.2.3", (1, 3, 0), False),
            ("~1", (1, 9, 9), True), ("~1", (2, 0, 0), False),
            ("=1.2", (1, 2, 7), True), ("=1.2", (1, 3, 0), False),
            ("=1.2.3", (1, 2, 3), True), ("=1.2.3", (1, 2, 4), False),
            (">1.2", (1, 2, 9), False), (">1.2", (1, 3, 0), True),
            (">1.2.3", (1, 2, 3), False), (">1.2.3", (1, 2, 4), True),
            ("<=1.2", (1, 2, 9), True), ("<=1.2", (1, 3, 0), False),
            ("<0.2.5", (0, 2, 4), True), ("<0.2.5", (0, 2, 5), False),
            (">= 1.0.0-alpha.1", (1, 0, 0), True), ("1.*", (1, 4, 0), True), ("*", (9, 9, 9), True),
        ]
        for requirement, version, expected in cases:
            with self.subTest(requirement=requirement, version=version):
                self.assertIs(MODULE.requirement_matches(requirement, version), expected)

    def test_an_unreadable_requirement_counts_as_affected(self) -> None:
        with self.assertRaises(ValueError):
            MODULE.requirement_matches("latest", (1, 0, 0))
        advisory = MODULE.Advisory("RUSTSEC-1", "x", "vulnerability", "t", ("latest",), (), False)
        self.assertTrue(MODULE.affects(advisory, (1, 0, 0)))

    def test_lines_and_versions(self) -> None:
        self.assertEqual(MODULE.line_of((0, 4, 45)), (0, 4))
        self.assertEqual(MODULE.line_of((1, 54, 1)), (1,))
        self.assertEqual(MODULE.line_of((0, 0, 3)), (0, 0, 3))
        self.assertEqual(MODULE.parse_line("0.10"), (0, 10))
        self.assertIsNone(MODULE.parse_version("1.0.0-rc.1"))
        self.assertEqual(MODULE.parse_version("1.0.0+build.5"), (1, 0, 0))
        with self.assertRaises(ValueError):
            MODULE.parse_line("0.x")


class Advisories(unittest.TestCase):
    def test_parse_reads_kind_versions_title_and_withdrawal(self) -> None:
        advisory = MODULE.parse_advisory(UNMAINTAINED)
        self.assertEqual((advisory.id, advisory.package, advisory.kind), ("RUSTSEC-2026-0543", "chrono", "unmaintained"))
        self.assertEqual((advisory.title, advisory.patched, advisory.withdrawn), ("chrono is unmaintained", (), False))
        patched = MODULE.parse_advisory(PATCHED)
        self.assertEqual((patched.kind, patched.patched), ("vulnerability", (">=0.4.20",)))
        self.assertTrue(MODULE.parse_advisory(WITHDRAWN).withdrawn)
        with self.assertRaises(ValueError):
            MODULE.parse_advisory("# no front matter\n")

    def test_tarball_keeps_listed_crates_and_records_the_commit(self) -> None:
        data = tarball({
            "crates/chrono/RUSTSEC-2026-0543.md": UNMAINTAINED,
            "crates/chrono/RUSTSEC-2020-0159.md": PATCHED,
            "crates/other/RUSTSEC-2026-0001.md": UNMAINTAINED,
            "crates/chrono/README.md": "not an advisory",
            "crates/broken/RUSTSEC-2026-0002.md": "no front matter",
        })
        revision, found, problems = MODULE.advisories_from_tarball(data, {"chrono", "tokio", "broken"})
        self.assertEqual(revision, "233155b4025b")
        self.assertEqual([advisory.id for advisory in found["chrono"]], ["RUSTSEC-2020-0159", "RUSTSEC-2026-0543"])
        self.assertEqual(found["tokio"], [])
        self.assertNotIn("other", found)
        self.assertEqual(problems, ["RustSec advisory RUSTSEC-2026-0002: no TOML front matter"])


class Evaluation(unittest.TestCase):
    def evaluate(self, name: str, line: str, data: dict, advisories: list | None = None,
                 repo: object | None = None) -> object:
        return MODULE.evaluate(name, line, data, advisories or [], repo or repository(), TODAY)

    def test_a_current_crate_is_not_flagged(self) -> None:
        health = self.evaluate("tokio", "1", crate(("1.54.1", "2026-10-21"), ("1.54.0", "2026-10-01")))
        self.assertEqual(health.flags, [])
        self.assertEqual((health.newest_in_line.version, health.latest.version), ("1.54.1", "1.54.1"))

    def test_staleness_uses_the_most_recent_release_of_any_line(self) -> None:
        stale = self.evaluate("secrecy", "0.10", crate(("0.10.3", "2024-10-09")))
        self.assertEqual([flag.key for flag in stale.flags], ["stale"])
        self.assertIn("no release for 25 months (last on 2024-10-09)", stale.flags[0].text)
        # A 1.x backport released recently keeps a crate whose 2.0 is old fresh.
        fresh = self.evaluate("x", "1", crate(("2.0.0", "2025-01-01"), ("1.9.1", "2026-10-01")))
        self.assertEqual([flag.key for flag in fresh.flags], ["line-2"])

    def test_a_newer_major_or_zero_minor_line_is_flagged(self) -> None:
        major = self.evaluate("axum", "0.8", crate(("0.9.0", "2026-11-01"), ("0.8.9", "2026-04-14")))
        self.assertEqual([flag.key for flag in major.flags], ["line-0.9"])
        self.assertEqual(major.flags[0].text, "newer incompatible line 0.9 (0.9.0, 2026-11-01); Rullst uses 0.8")
        # Pre-releases and yanked releases are ignored.
        ignored = self.evaluate("axum", "0.8", crate(("0.9.0-alpha.1", "2026-11-01"), ("0.9.0", "2026-11-02", True),
                                                     ("0.8.9", "2026-04-14")))
        self.assertEqual(ignored.flags, [])
        self.assertEqual(ignored.latest.version, "0.8.9")

    def test_only_advisories_the_newest_release_of_the_line_does_not_escape_are_flagged(self) -> None:
        advisories = [MODULE.parse_advisory(text) for text in (UNMAINTAINED, PATCHED, WITHDRAWN)]
        health = self.evaluate("chrono", "0.4", crate(("0.4.45", "2026-06-04")), advisories)
        self.assertEqual([advisory.id for advisory in health.advisories], ["RUSTSEC-2026-0543"])
        self.assertEqual([flag.key for flag in health.flags], ["RUSTSEC-2026-0543"])
        self.assertEqual(
            health.flags[0].text,
            "[RUSTSEC-2026-0543](https://rustsec.org/advisories/RUSTSEC-2026-0543.html) (unmaintained) "
            "affects 0.4.45: chrono is unmaintained",
        )

    def test_archived_missing_and_lineless_crates_are_flagged(self) -> None:
        archived = self.evaluate("a", "1", crate(("1.0.0", "2026-11-01")), repo=repository(archived=True))
        self.assertEqual([flag.text for flag in archived.flags], ["repository owner/name is archived"])
        gone = MODULE.Repository("owner/gone", "https://github.com/owner/gone", None, True)
        missing = self.evaluate("b", "1", crate(("1.0.0", "2026-11-01")), repo=gone)
        self.assertEqual([flag.key for flag in missing.flags], ["no-repository"])
        lineless = self.evaluate("c", "0.3", crate(("0.4.0", "2026-11-01")))
        self.assertEqual([flag.key for flag in lineless.flags], ["no-line-release"])
        empty = self.evaluate("d", "1", crate(("1.0.0-rc.1", "2026-11-01")))
        self.assertEqual([flag.key for flag in empty.flags], ["no-release", "no-line-release"])

    def test_github_slugs(self) -> None:
        for url, slug in [
            ("https://github.com/tokio-rs/tokio", "tokio-rs/tokio"),
            ("https://github.com/serde-rs/json.git", "serde-rs/json"),
            ("https://github.com/tower-rs/tower/tree/master/tower-layer", "tower-rs/tower"),
            ("http://www.github.com/a/b.rs/", "a/b.rs"),
            ("https://gitlab.com/a/b", None),
            (None, None),
        ]:
            with self.subTest(url=url):
                self.assertEqual(MODULE.github_slug(url), slug)


class Report(unittest.TestCase):
    def test_the_fingerprint_ignores_ages_and_dates(self) -> None:
        older = MODULE.evaluate("secrecy", "0.10", crate(("0.10.3", "2024-10-09")), [], repository(), TODAY)
        later = MODULE.evaluate("secrecy", "0.10", crate(("0.10.3", "2024-10-09")), [], repository(),
                                TODAY + dt.timedelta(days=62))
        self.assertNotEqual(older.flags[0].text, later.flags[0].text)
        self.assertEqual(MODULE.fingerprint([older]), MODULE.fingerprint([later]))
        self.assertEqual(MODULE.fingerprint([older]), "<!-- flags: secrecy:stale -->")
        fresh = MODULE.evaluate("tokio", "1", crate(("1.54.1", "2026-10-21")), [], repository(), TODAY)
        self.assertEqual(MODULE.fingerprint([fresh]), "<!-- flags: none -->")

    def test_render_lists_flags_errors_and_every_crate(self) -> None:
        stale = MODULE.evaluate("secrecy", "0.10", crate(("0.10.3", "2024-10-09")), [], repository(), TODAY)
        fresh = MODULE.evaluate("tokio", "1", crate(("1.54.1", "2026-10-21")), [], repository(), TODAY)
        report = MODULE.render([stale, fresh], TODAY, "233155b4025b", ["crates.io `x`: timed out"])
        self.assertIn("## Not checked\n\n- crates.io `x`: timed out\n", report)
        self.assertIn("## Flagged (1)\n\n- `secrecy` (Rullst line `0.10`):\n  - no release for 25 months", report)
        self.assertIn("| `tokio` | `1` | 1.54.1 (2026-10-21) | 1.54.1 (2026-10-21) | none | owner/name (pushed 2026-11-01) |", report)
        self.assertTrue(report.endswith("<!-- flags: secrecy:stale -->\n"))
        clean = MODULE.render([fresh], TODAY, "233155b4025b", [])
        self.assertIn("Nothing is flagged.", clean)
        self.assertNotIn("## Not checked", clean)


class Main(unittest.TestCase):
    """main() with the network replaced by fixtures."""

    def run_main(self, responses: dict[str, object]) -> tuple[int, str, list[str]]:
        requested: list[str] = []

        def fake_fetch(url: str, *, token: str | None = None, accept: str = "application/json") -> bytes:
            requested.append(url)
            response = responses[url]
            if isinstance(response, Exception):
                raise response
            return response if isinstance(response, bytes) else json.dumps(response).encode()

        with tempfile.TemporaryDirectory() as directory:
            policy = Path(directory, "policy.json")
            policy.write_text(json.dumps({"dependencies": {"chrono": "0.4", "tokio": "1"}}))
            output = io.StringIO()
            with mock.patch.object(MODULE, "fetch", fake_fetch), mock.patch.object(MODULE.time, "sleep"), \
                    mock.patch.object(MODULE, "POLICY_FILE", policy), \
                    mock.patch.dict(MODULE.os.environ, {"GITHUB_TOKEN": ""}), \
                    contextlib.redirect_stdout(output), contextlib.redirect_stderr(io.StringIO()):
                status = MODULE.main([])
            return status, output.getvalue(), requested

    def responses(self) -> dict[str, object]:
        return {
            "https://api.github.com/repos/rustsec/advisory-db/tarball/main":
                tarball({"crates/chrono/RUSTSEC-2026-0543.md": UNMAINTAINED}),
            "https://crates.io/api/v1/crates/chrono":
                crate(("0.4.45", "2026-06-04"), repository="https://github.com/chronotope/chrono"),
            "https://crates.io/api/v1/crates/tokio":
                crate(("1.54.1", "2026-10-21"), repository="https://github.com/tokio-rs/tokio"),
            "https://api.github.com/repos/chronotope/chrono": {"archived": False, "pushed_at": "2026-09-07T00:00:00Z"},
            "https://api.github.com/repos/tokio-rs/tokio": {"archived": False, "pushed_at": "2026-11-17T00:00:00Z"},
        }

    def test_a_flag_exits_1_with_the_report(self) -> None:
        status, report, requested = self.run_main(self.responses())
        self.assertEqual(status, 1)
        self.assertIn("RUSTSEC-2026-0543", report)
        self.assertTrue(report.endswith("<!-- flags: chrono:RUSTSEC-2026-0543 -->\n"))
        self.assertEqual(len(requested), 5)

    def test_nothing_flagged_exits_0(self) -> None:
        responses = self.responses()
        responses["https://api.github.com/repos/rustsec/advisory-db/tarball/main"] = tarball(
            {"crates/other/RUSTSEC-2026-0001.md": UNMAINTAINED})
        status, report, _ = self.run_main(responses)
        self.assertEqual(status, 0)
        self.assertIn("Nothing is flagged.", report)

    def test_an_unreadable_source_exits_2_and_still_reports(self) -> None:
        responses = self.responses()
        responses["https://crates.io/api/v1/crates/tokio"] = urllib.error.URLError("timed out")
        responses["https://api.github.com/repos/chronotope/chrono"] = urllib.error.HTTPError(
            "https://api.github.com/repos/chronotope/chrono", 404, "Not Found", {}, None)
        status, report, _ = self.run_main(responses)
        self.assertEqual(status, 2)
        self.assertIn("- crates.io `tokio`: <urlopen error timed out>", report)
        self.assertIn("repository chronotope/chrono was not found on GitHub", report)


class Redirects(unittest.TestCase):
    def test_the_token_is_not_sent_to_another_host(self) -> None:
        handler = MODULE._SameHostAuthorization()
        request = MODULE.urllib.request.Request(
            "https://api.github.com/repos/rustsec/advisory-db/tarball/main",
            headers={"Authorization": "Bearer secret", "User-Agent": "rullst-ci"},
        )
        moved = handler.redirect_request(request, None, 302, "Found", {}, "https://codeload.github.com/x.tar.gz")
        self.assertFalse(moved.has_header("Authorization"))
        self.assertEqual(moved.get_header("User-agent"), "rullst-ci")
        renamed = handler.redirect_request(request, None, 301, "Moved", {}, "https://api.github.com/repositories/1")
        self.assertEqual(renamed.get_header("Authorization"), "Bearer secret")


if __name__ == "__main__":
    unittest.main()
