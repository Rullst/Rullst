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


def rounds(minimal, generated, errors=0, saas=None):
    measured = []
    for index, (m, g) in enumerate(zip(minimal, generated)):
        one = {"minimal": CHECK.figures(result(ok=m), 20.0),
               "generated": CHECK.figures(result(ok=g, errors=errors), 20.0)}
        if saas is not None:
            one["saas"] = CHECK.figures(result(ok=saas[index]), 20.0)
        measured.append(one)
    return measured


class StackThroughputTest(unittest.TestCase):
    def test_figures_of_a_bombardier_result(self):
        f = CHECK.figures(result(ok=50_000, taken=10.0, p99_us=4200, errors=3), 19.0)
        self.assertEqual(f["rps"], 5000.0)
        self.assertEqual(f["p99_ms"], 4.2)
        self.assertEqual(f["errors"], 3)
        self.assertAlmostEqual(f["cpu_us_per_request"], 380.0)
        self.assertAlmostEqual(f["server_cpus"], 1.9)

    def test_the_ratio_compares_medians(self):
        measured = rounds([1000, 1200, 900], [500, 700, 100], saas=[200, 300, 250])
        self.assertAlmostEqual(CHECK.ratio(measured), 500 / 1000)
        self.assertAlmostEqual(CHECK.ratio(measured, "saas"), 250 / 1000)

    def test_a_ratio_below_its_limit_or_an_error_blocks(self):
        measured = rounds([1000, 1000, 1000], [600, 600, 600], saas=[300, 300, 300])
        self.assertEqual(CHECK.verdict(measured, {"generated": 0.55, "saas": 0.25}), [])
        self.assertEqual(CHECK.verdict(measured, None), [])
        self.assertEqual(CHECK.verdict(measured, {"generated": 0.65, "saas": 0.25}),
                         ["Blank generated/minimal ratio 0.600 is below the limit 0.650"])
        self.assertEqual(CHECK.verdict(measured, {"generated": 0.55, "saas": 0.35}),
                         ["SaaS signed-in dashboard/minimal ratio 0.300 is below the limit 0.350"])
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
        # A request with the CSRF cookie gets no new one.
        self.assertEqual(CHECK.missing_security(headers, cookieless=False), ["x-frame-options"])

    def test_the_measured_saas_page_is_the_signed_in_dashboard(self):
        headers = {name: "x" for name in CHECK.SECURITY_HEADERS}
        page = f"<h1>Welcome, {CHECK.ACCOUNT_NAME}</h1>"
        self.assertEqual(CHECK.dashboard_problems(200, headers, page), [])
        self.assertEqual(CHECK.dashboard_problems(200, headers, "<h1>Sign in</h1>"), ["the account's dashboard"])
        del headers["referrer-policy"]
        self.assertEqual(CHECK.dashboard_problems(503, headers, ""),
                         ["status 503", "the account's dashboard", "referrer-policy"])

    def test_the_summary_names_each_ratio_limit_and_suggestion(self):
        measured = rounds([1000], [600], saas=[300])
        text = CHECK.summary(measured, {"generated": 0.5, "saas": 0.2}, [])
        self.assertIn("Blank generated/minimal ratio (medians): **0.600**. Limit: 0.500. "
                      "Suggested limit: 0.510.", text)
        self.assertIn("SaaS signed-in dashboard/minimal ratio (medians): **0.300**. Limit: 0.200. "
                      "Suggested limit: 0.255.", text)
        self.assertIn("| 1 | saas | 30 |", text)
        self.assertIn("Passed.", text)
        self.assertIn("**Failed:** slow", CHECK.summary(measured, None, ["slow"]))
        self.assertNotIn("Limit:", CHECK.summary(measured, None, []))

    def test_the_summary_names_the_cpu_model(self):
        self.assertEqual(CHECK.cpu_model("processor\t: 0\nmodel name\t: AMD EPYC 7763 64-Core Processor\n"),
                         "AMD EPYC 7763 64-Core Processor")
        self.assertEqual(CHECK.cpu_model(""), "unknown CPU")
        self.assertIn("CPU: Test CPU", CHECK.summary(rounds([1000], [600]), None, [], "Test CPU"))

    def test_the_budget_holds_a_ratio_below_one_for_each_stack(self):
        budget = CHECK.limits(tomllib.loads(CHECK.BUDGET.read_text()))
        self.assertEqual(set(budget), {"generated", "saas"})
        for limit in budget.values():
            self.assertGreater(limit, 0)
            self.assertLess(limit, 1)


if __name__ == "__main__":
    unittest.main()
