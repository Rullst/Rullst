#!/usr/bin/env python3
"""Offline tests of .github/check-starter-footprint.py and the starter budgets."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import sys
import tempfile
import tomllib
import unittest

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
SPEC = importlib.util.spec_from_file_location("check_starter_footprint", HERE / "check-starter-footprint.py")
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("cannot import check-starter-footprint.py")
CHECK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECK)


def metric(value):
    return {"status": "measured" if value is not None else "not_measured", "value": value,
            "reason": None if value is not None else "not readable"}


def report(rss=20_000_000, limits=(), status="passed", failure=None, origins=()) -> dict:
    return {
        "schema": "rullst.footprint.v1", "cli_version": "13.0.0-alpha.5",
        "machine": {"os": "linux", "arch": "x86_64", "logical_cpus": 4, "cpu_model": "Test CPU"},
        "measured": {"binary_size_bytes": metric(8_000_000), "image_size_bytes": metric(None),
                     "idle_rss_bytes": metric(rss), "dependency_count": metric(300)},
        "benchmark": {"requests": 5000, "http_errors": 0, "content_encodings": ["zstd"],
                      "cpu_ms_per_request": metric(0.12), "bytes_per_request": metric(1234.5)},
        "page_weight": {"status": "measured", "first_visit": {"bytes": 50_000, "requests": 4},
                        "repeat_visit": {"bytes": 1_300, "requests": 4}, "third_party_origins": list(origins)},
        "budgets": {"status": status, "limits": [dict(limit) for limit in limits]},
        "failure": failure,
    }


def limit(name: str, allowed: float, measured: float | None, status: str, reason: str | None = None) -> dict:
    return {"name": name, "allowed": allowed, "measured": measured, "status": status, "reason": reason}


class PolicyTests(unittest.TestCase):
    def test_passed_limits_do_not_block_and_cpu_is_never_a_limit(self):
        rows, problems = CHECK.evaluate("blank", report(limits=[limit("idle_rss_bytes", 24e6, 20e6, "passed")]),
                                        budgeted=True)
        self.assertEqual(problems, [])
        self.assertEqual([row["name"] for row in rows], list(CHECK.MARGINS))
        self.assertNotIn("cpu_ms_per_request", CHECK.MARGINS)
        rss = next(row for row in rows if row["name"] == "idle_rss_bytes")
        self.assertEqual((rss["allowed"], rss["status"], rss["suggested"]), (24e6, "passed", 24_000_000))

    def test_exceeded_and_unmeasured_limits_block(self):
        limits = [limit("binary_size_bytes", 7e6, 8e6, "exceeded"),
                  limit("idle_rss_bytes", 24e6, None, "skipped", "the app process was not identified")]
        _, problems = CHECK.evaluate("saas", report(rss=None, limits=limits, status="exceeded"), budgeted=True)
        self.assertEqual(problems, ["saas: binary_size_bytes is 8,000,000, over its limit of 7,000,000",
                                    "saas: idle_rss_bytes has a limit but was not measured "
                                    "(the app process was not identified)"])

    def test_a_failed_measurement_or_an_unchecked_budget_blocks(self):
        failure = {"stage": "start", "title": "App did not start", "message": "it exited"}
        _, problems = CHECK.evaluate("api", report(failure=failure, status="not_checked"), budgeted=True)
        self.assertEqual(problems, ["api: footprint could not measure the starter (start): App did not start. "
                                    "it exited", "api: the budget was not checked"])
        _, reported = CHECK.evaluate("api", report(status="not_checked"), budgeted=False)
        self.assertEqual(reported, [])

    def test_suggested_limits_add_the_margin_and_round_up(self):
        self.assertEqual(CHECK.suggest("binary_size_bytes", 8_000_000), 9_200_000)
        self.assertEqual(CHECK.suggest("idle_rss_bytes", 20_000_001), 24_000_002)
        self.assertEqual(CHECK.suggest("dependency_count", 300), 315)
        self.assertEqual(CHECK.suggest("page_first_visit_bytes", 50_000), 55_000)
        self.assertEqual(CHECK.suggest("page_first_visit_bytes", 5_000), 6_024)
        self.assertEqual(CHECK.suggest("bytes_per_request", 1234.5), 2259)
        self.assertEqual(CHECK.suggest("page_repeat_visit_bytes", 0), 1024)
        self.assertEqual(CHECK.suggest("page_requests", 4), 4)
        self.assertEqual(CHECK.suggest("page_third_party_origins", 0), 0)

    def test_summary_shows_limits_reported_cpu_and_a_pasteable_budget(self):
        rows, problems = CHECK.evaluate("blank", report(origins=["https://cdn.example"]), budgeted=False)
        summary = CHECK.render({"blank": report()}, rows, problems)
        self.assertIn("| blank | `binary_size_bytes` | 8,000,000 | — | reported | 9,200,000 |", summary)
        self.assertIn("| blank | `page_third_party_origins` | 1 | — | reported | 1 |", summary)
        self.assertIn("| blank | 0.12 | 5,000 | 0 | zstd |", summary)
        self.assertIn("[footprint.budgets]\nbinary_size_bytes = 9_200_000\nidle_rss_bytes = 24_000_000\n", summary)
        tomllib.loads(summary.split("```toml\n", 1)[1].split("```", 1)[0])


class BudgetFileTests(unittest.TestCase):
    def test_every_starter_has_a_complete_budget_and_nothing_else_does(self):
        files = {path.stem for path in CHECK.BUDGETS.glob("*.toml")}
        self.assertEqual(files, set(CHECK.STARTERS))
        for starter in CHECK.STARTERS:
            with self.subTest(starter=starter):
                limits = CHECK.load_budget(CHECK.budget_path(starter))
                self.assertEqual(set(limits), set(CHECK.MARGINS))
                for name, value in limits.items():
                    self.assertIsInstance(value, int, name)
                    self.assertGreaterEqual(value, 0, name)
                self.assertEqual(limits["page_third_party_origins"], 0)

    def test_an_unknown_or_missing_limit_table_is_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            for text in ("[footprint.budgets]\ncpu_ms_per_request = 1\n", "[footprint]\n"):
                path = Path(directory) / "starter.toml"
                path.write_text(text, encoding="utf-8")
                with self.assertRaises(CHECK.Failure):
                    CHECK.load_budget(path)


if __name__ == "__main__":
    unittest.main()
