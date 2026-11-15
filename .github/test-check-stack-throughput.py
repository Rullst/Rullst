#!/usr/bin/env python3
"""Offline tests of .github/check-stack-throughput.py and its budget."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import sys
import tomllib
import unittest

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("check_stack_throughput", HERE / "check-stack-throughput.py")
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("cannot import check-stack-throughput.py")
CHECK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECK)


def result(ok=100_000, taken=10.0, p99_us=2500, errors=0) -> dict:
    return {"result": {"req1xx": 0, "req2xx": ok, "req3xx": 0, "req4xx": errors, "req5xx": 0, "others": 0,
                       "timeTakenSeconds": taken, "latency": {"percentiles": {"99": p99_us}}}}


def rounds(minimal, generated, errors=0):
    return [{"minimal": CHECK.figures(result(ok=m), 20.0),
             "generated": CHECK.figures(result(ok=g, errors=errors), 20.0)}
            for m, g in zip(minimal, generated)]


class StackThroughputTest(unittest.TestCase):
    def test_figures_of_a_bombardier_result(self):
        f = CHECK.figures(result(ok=50_000, taken=10.0, p99_us=4200, errors=3), 19.0)
        self.assertEqual(f["rps"], 5000.0)
        self.assertEqual(f["p99_ms"], 4.2)
        self.assertEqual(f["errors"], 3)
        self.assertAlmostEqual(f["cpu_us_per_request"], 380.0)
        self.assertAlmostEqual(f["server_cpus"], 1.9)

    def test_the_ratio_compares_medians(self):
        measured = rounds([1000, 1200, 900], [500, 700, 100])
        self.assertAlmostEqual(CHECK.ratio(measured), 500 / 1000)

    def test_a_ratio_below_the_limit_or_an_error_blocks(self):
        measured = rounds([1000, 1000, 1000], [600, 600, 600])
        self.assertEqual(CHECK.verdict(measured, 0.55), [])
        self.assertEqual(CHECK.verdict(measured, None), [])
        self.assertEqual(CHECK.verdict(measured, 0.65),
                         ["generated/minimal ratio 0.600 is below the limit 0.650"])
        failing = rounds([1000], [600], errors=2)
        self.assertEqual(CHECK.verdict(failing, None),
                         ["round 1: 2 non-2xx responses from the generated stack"])

    def test_a_generated_response_must_carry_the_production_stack(self):
        headers = {name: "x" for name in CHECK.SECURITY_HEADERS}
        headers["Set-Cookie"] = "rullst_csrf=abc; Path=/; SameSite=Lax; Secure"
        self.assertEqual(CHECK.missing_security(headers), [])
        del headers["x-frame-options"]
        headers["Set-Cookie"] = "other=1"
        self.assertEqual(CHECK.missing_security(headers), ["x-frame-options", "set-cookie: rullst_csrf"])

    def test_the_summary_names_the_ratio_limit_and_suggestion(self):
        measured = rounds([1000], [600])
        text = CHECK.summary(measured, 0.5, [])
        self.assertIn("ratio (medians): **0.600**", text)
        self.assertIn("Limit: 0.500. Suggested limit: 0.510.", text)
        self.assertIn("Passed.", text)
        self.assertIn("**Failed:** slow", CHECK.summary(measured, 0.7, ["slow"]))

    def test_the_budget_holds_a_ratio_below_one(self):
        budget = tomllib.loads(CHECK.BUDGET.read_text())["throughput"]
        self.assertGreater(budget["min_ratio"], 0)
        self.assertLess(budget["min_ratio"], 1)


if __name__ == "__main__":
    unittest.main()
