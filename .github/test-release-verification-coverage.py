#!/usr/bin/env python3
"""Every check release.yml no longer repeats still runs in a required ci.yml job.

release.yml's admit job requires each ci.yml job in the release policy to have
succeeded at the exact SHA. This test keeps .github/release-verification-coverage.json
honest: each recorded command must still be in that job, in a step whose `if`
holds for the recorded matrix, and every rendered job name must be required.
"""

from __future__ import annotations

from itertools import product
import json
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parent.parent
COVERAGE = json.loads((ROOT / ".github/release-verification-coverage.json").read_text())
POLICY = json.loads((ROOT / ".github/release-required-workflows.json").read_text())
CI = (ROOT / ".github/workflows/ci.yml").read_text()
RELEASE = (ROOT / ".github/workflows/release.yml").read_text()
RUNNER_OS = {"ubuntu": "Linux", "macos": "macOS", "windows": "Windows"}
CONDITION = re.compile(r"(matrix\.[a-z_-]+|runner\.os) == '([^']*)'")


def jobs(text: str) -> dict[str, list[str]]:
    """Split a workflow into top-level jobs; Rullst workflows use two-space YAML."""
    lines = text.splitlines()
    result: dict[str, list[str]] = {}
    current = None
    for line in lines[lines.index("jobs:") + 1:]:
        if match := re.fullmatch(r"  ([A-Za-z0-9_-]+):", line):
            current = match[1]
            result[current] = []
        elif current is not None:
            result[current].append(line)
    return result


def job_name(lines: list[str]) -> str:
    name = next(line for line in lines if line.startswith("    name: "))
    return name.removeprefix("    name: ").strip().strip("'\"")


def steps(lines: list[str]) -> list[tuple[str | None, str]]:
    """Return each step's `if` expression and whitespace-normalized text."""
    chunks: list[list[str]] = []
    for line in lines:
        if line.startswith("      - "):
            chunks.append([line])
        elif chunks and (line.startswith("        ") or not line.strip()):
            chunks[-1].append(line)
    result = []
    for chunk in chunks:
        condition = next(
            (line.removeprefix("        if: ").strip() for line in chunk
             if line.startswith("        if: ")),
            None,
        )
        result.append((condition, " ".join(" ".join(chunk).split())))
    return result


def holds(condition: str | None, matrix: dict[str, str]) -> bool:
    if condition is None:
        return True
    clauses = [clause.strip() for clause in condition.split("&&")]
    values = dict(matrix)
    values["runner.os"] = RUNNER_OS[matrix["matrix.os"].split("-")[0]] if "matrix.os" in matrix else ""
    for clause in clauses:
        match = CONDITION.fullmatch(clause)
        if match is None:
            raise AssertionError(f"unsupported step condition {condition!r}; extend this test")
        if values.get(match[1]) != match[2]:
            return False
    return True


def combinations(matrix: dict[str, list[str]]) -> list[dict[str, str]]:
    keys = sorted(matrix)
    return [
        {f"matrix.{key}": value for key, value in zip(keys, values)}
        for values in product(*(matrix[key] for key in keys))
    ]


class CoverageTests(unittest.TestCase):
    def setUp(self):
        self.jobs = jobs(CI)
        self.required = set(next(
            item["required_jobs"] for item in POLICY["workflows"] if item["workflow"] == "ci.yml"
        ))

    def test_each_command_runs_in_a_required_job_at_the_release_sha(self):
        for entry in COVERAGE["covered"]:
            lines = self.jobs[entry["job"]]
            template = job_name(lines)
            job_steps = steps(lines)
            for matrix in combinations(entry["matrix"]):
                name = re.sub(
                    r"\$\{\{ (matrix\.[a-z_-]+) \}\}", lambda match: matrix[match[1]], template
                )
                self.assertIn(name, self.required, f"{entry['job']} renders unrequired {name!r}")
                for command in entry["commands"]:
                    with self.subTest(job=name, command=command):
                        self.assertTrue(
                            any(command in text and holds(condition, matrix)
                                for condition, text in job_steps),
                            f"{command!r} does not run in {name!r}",
                        )

    def test_release_matches_archives_to_the_admitted_consumer_test(self):
        # The packaged-distribution entry only covers the release because the
        # release compares its archives with the ones that job tested.
        self.assertTrue("python3 .github/check-admitted-archives.py" in RELEASE,
                        "release.yml must compare its archives with the admitted test")
        self.assertTrue("name: packaged-checksums" in CI,
                        "ci.yml must keep the tested archive checksums")

    def test_condition_parser_refuses_unknown_forms(self):
        self.assertTrue(holds("matrix.shard == 'workspace'", {"matrix.shard": "workspace"}))
        self.assertFalse(holds(
            "runner.os == 'Linux' && matrix.shard == 'workspace'",
            {"matrix.os": "windows-latest", "matrix.shard": "workspace"},
        ))
        with self.assertRaises(AssertionError):
            holds("matrix.shard != 'workspace'", {"matrix.shard": "workspace"})


if __name__ == "__main__":
    unittest.main()
