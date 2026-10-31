#!/usr/bin/env python3
"""Offline tests of .github/check-starter-browsers.py's policy and summary."""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("check_starter_browsers", HERE / "check-starter-browsers.py")
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("cannot import check-starter-browsers.py")
CHECK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECK)

KNOWN = [{"starter": "blank", "page": "/", "kind": "csp", "key": "style-src-elem inline",
          "owner": "rullst-core", "reason": "htmx indicator styles"}]


def finding(kind: str, key: str, browser: str = "chromium", page: str = "/", **extra) -> dict:
    return {"browser": browser, "page": page, "kind": kind, "key": key, "detail": key, **extra}


def report(starter: str, *findings: dict, browser: str = "chromium") -> dict:
    return {"starter": starter, "versions": {"playwright": "1.64.1", "axe-core": "4.14.0"},
            "browsers": [{"browser": browser, "version": "1", "steps": ["/"], "findings": list(findings)}]}


class PolicyTests(unittest.TestCase):
    def test_axe_impact_decides_between_failing_and_recording(self):
        result = CHECK.evaluate([report("saas", finding("axe", "color-contrast", impact="serious"),
                                        finding("axe", "region", impact="moderate"),
                                        finding("axe", "image-alt", impact="critical"))], [])
        self.assertEqual([item["key"] for item in result["blocking"]], ["color-contrast", "image-alt"])
        self.assertEqual([item["key"] for item in result["recorded"]], ["region"])

    def test_known_framework_defect_is_reported_with_its_console_line_covered(self):
        result = CHECK.evaluate([report("blank", finding("csp", "style-src-elem inline"),
                                        finding("console", "Content-Security-Policy blocked", csp=True))], KNOWN)
        self.assertEqual(result["blocking"], [])
        self.assertEqual([(item["kind"], item["owner"]) for item in result["known"]], [("csp", "rullst-core")])
        self.assertEqual(result["stale"], [])

    def test_unknown_csp_violation_console_error_and_keyboard_defect_fail(self):
        result = CHECK.evaluate([report("saas", finding("csp", "script-src-elem inline"),
                                        finding("console", "Refused: Content Security Policy", csp=True),
                                        finding("keyboard", "no visible focus: button"),
                                        finding("console", "TypeError", page="/login", csp=True))], KNOWN)
        self.assertEqual([item["kind"] for item in result["blocking"]], ["csp", "keyboard", "console"])

    def test_known_defect_matches_only_its_starter_page_and_browsers(self):
        scoped = [{**KNOWN[0], "browsers": ["firefox"]}]
        result = CHECK.evaluate([report("blank", finding("csp", "style-src-elem inline")),
                                 report("saas", finding("csp", "style-src-elem inline"))], scoped)
        self.assertEqual(len(result["blocking"]), 2)
        self.assertEqual(result["stale"], scoped)

    def test_stale_entries_fail_only_for_evaluated_starters(self):
        self.assertEqual(CHECK.evaluate([report("saas")], KNOWN)["stale"], [])
        self.assertEqual(CHECK.evaluate([report("blank")], KNOWN)["stale"], KNOWN)

    def test_summary_merges_browsers_and_marks_cells(self):
        reports = [report("blank", finding("csp", "style-src-elem inline")),
                   report("blank", finding("csp", "style-src-elem inline", browser="firefox"), browser="firefox"),
                   report("saas", finding("flow", "sign-in failed"))]
        text = CHECK.render(reports, CHECK.evaluate(reports, KNOWN))
        self.assertIn("| blank | chromium 1 | pass | known defect (1) | pass | pass | pass | 0 | 1 |", text)
        self.assertIn("| saas | chromium 1 | pass | pass | pass | FAIL (1) | pass | 0 | 1 |", text)
        self.assertIn("[rullst-core]: `style-src-elem inline`", text)
        self.assertIn("(chromium, firefox)", text)

    def test_screenshot_csp_reports_are_noted_without_failing(self):
        captured = report("saas", browser="webkit")
        captured["browsers"][0]["capture_reports"] = 5
        result = CHECK.evaluate([captured], [])
        self.assertEqual(result["blocking"], [])
        self.assertIn("screenshot styles (not the starter) were ignored: webkit 5.", CHECK.render([captured], result))

    def test_known_defects_file_is_valid_and_entries_need_owner_and_reason(self):
        self.assertTrue(all(entry["owner"] and entry["reason"] for entry in CHECK.load_known_defects()))
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "known.json"
            path.write_text(json.dumps([{**KNOWN[0], "reason": ""}]), encoding="utf-8")
            with self.assertRaises(CHECK.Failure):
                CHECK.load_known_defects(path)

    def test_driver_never_bypasses_the_starter_csp_and_stays_on_loopback(self):
        driver = CHECK.DRIVER.read_text(encoding="utf-8")
        self.assertNotIn("bypassCSP: true", driver)
        self.assertIn('LOOPBACK = new Set(["127.0.0.1", "localhost", "[::1]"])', driver)
        lock = json.loads((CHECK.DRIVER.parent / "package-lock.json").read_text(encoding="utf-8"))
        for name in ("playwright", "playwright-core", "axe-core"):
            self.assertTrue(lock["packages"][f"node_modules/{name}"]["integrity"].startswith("sha512-"), name)


if __name__ == "__main__":
    unittest.main()
