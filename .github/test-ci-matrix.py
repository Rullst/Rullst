#!/usr/bin/env python3
"""Evaluate ci.yml's matrix, job guards and concurrency group for each event.

The expressions are read from the workflow and evaluated with a small
implementation of the GitHub Actions expression language, so a change to the
`full-matrix` label logic is tested as written, not as a copy.
"""

from __future__ import annotations

import json
import math
from pathlib import Path
import re
import unittest


WORKFLOW = Path(__file__).resolve().parent / "workflows" / "ci.yml"
LINUX = ["ubuntu-latest"]
ALL_PLATFORMS = ["ubuntu-latest", "macos-latest", "windows-latest"]
PR_GATE_SHARDS = ["workspace", "cli-standard"]
ALL_SHARDS = ["workspace", "cli-standard", "cli-profiles-basic", "cli-profiles-relational",
              "cli-profiles-polyglot", "cli-saas-foundation", "cli-saas-product"]
# The ci.yml contexts among the 20 required status checks of the main ruleset
# (`gh api repos/Rullst/Rullst/rulesets/24175738`). Every PR run must report
# each of them, or a newer run would leave a required check missing.
REQUIRED_CI_CONTEXTS = {
    "Code Quality & Format",
    "Run Tests (ubuntu-latest / workspace)",
    "Run Tests (ubuntu-latest / cli-standard)",
    "Check MSRV (Rust 1.96.0)",
    "ORM strict-sqlite",
    "ORM strict-postgres",
    "ORM strict-mysql",
    "ORM differential databases",
    "Redis live contracts",
    "Generated release access boundaries",
    "Facade shared-local recovery composition",
    "Versioned deterministic AI evals",
}


class Expression:
    """A recursive-descent evaluator for the operators and functions ci.yml uses."""

    TOKEN = re.compile(r"\s*(?:(?P<string>'(?:[^']|'')*')|(?P<number>\d+(?:\.\d+)?)|"
                       r"(?P<op>==|!=|<=|>=|&&|\|\||[!<>().,\[\]*])|(?P<name>[A-Za-z_][A-Za-z0-9_-]*))")

    def __init__(self, source: str, context: dict):
        self.tokens = self.tokenize(source)
        self.position = 0
        self.context = context

    @classmethod
    def tokenize(cls, source: str) -> list[tuple[str, str]]:
        tokens, index = [], 0
        while index < len(source):
            if source[index:].strip() == "":
                break
            match = cls.TOKEN.match(source, index)
            if match is None:
                raise ValueError(f"cannot tokenize {source[index:]!r}")
            kind = match.lastgroup
            tokens.append((kind, match.group(kind)))
            index = match.end()
        return tokens

    def peek(self) -> tuple[str, str] | None:
        return self.tokens[self.position] if self.position < len(self.tokens) else None

    def take(self, value: str | None = None) -> tuple[str, str]:
        token = self.peek()
        if token is None or (value is not None and token[1] != value):
            raise ValueError(f"expected {value!r}, found {token!r}")
        self.position += 1
        return token

    def evaluate(self):
        value = self.disjunction()
        if self.peek() is not None:
            raise ValueError(f"unexpected {self.peek()!r}")
        return value

    def disjunction(self):
        value = self.conjunction()
        while self.peek() == ("op", "||"):
            self.take()
            right = self.conjunction()
            value = value if truthy(value) else right
        return value

    def conjunction(self):
        value = self.equality()
        while self.peek() == ("op", "&&"):
            self.take()
            right = self.equality()
            value = right if truthy(value) else value
        return value

    def equality(self):
        value = self.comparison()
        while self.peek() in (("op", "=="), ("op", "!=")):
            operator = self.take()[1]
            right = self.comparison()
            value = loose_equal(value, right) == (operator == "==")
        return value

    def comparison(self):
        value = self.unary()
        while self.peek() in (("op", "<"), ("op", "<="), ("op", ">"), ("op", ">=")):
            operator = self.take()[1]
            left, right = number(value), number(self.unary())
            value = {"<": left < right, "<=": left <= right,
                     ">": left > right, ">=": left >= right}[operator]
        return value

    def unary(self):
        if self.peek() == ("op", "!"):
            self.take()
            return not truthy(self.unary())
        return self.postfix()

    def postfix(self):
        value = self.primary()
        while self.peek() in (("op", "."), ("op", "[")):
            if self.take()[1] == ".":
                if self.peek() == ("op", "*"):
                    self.take()
                    value = list(value) if isinstance(value, list) else \
                        list(value.values()) if isinstance(value, dict) else []
                    value = Filtered(value)
                else:
                    value = member(value, self.take()[1])
            else:
                key = self.disjunction()
                self.take("]")
                value = member(value, key)
        return list(value) if isinstance(value, Filtered) else value

    def primary(self):
        kind, text = self.take()
        if kind == "string":
            return text[1:-1].replace("''", "'")
        if kind == "number":
            return float(text)
        if text == "(":
            value = self.disjunction()
            self.take(")")
            return value
        if kind != "name":
            raise ValueError(f"unexpected {text!r}")
        if text in ("true", "false"):
            return text == "true"
        if text == "null":
            return None
        if self.peek() == ("op", "("):
            self.take()
            arguments = []
            while self.peek() != ("op", ")"):
                arguments.append(self.disjunction())
                if self.peek() == ("op", ","):
                    self.take()
            self.take(")")
            return call(text, arguments)
        return self.context.get(text)


class Filtered(list):
    """The result of `.*`: a later `.name` maps over the elements."""


def member(value, key):
    if isinstance(value, Filtered):
        return Filtered(member(item, key) for item in value)
    if isinstance(value, dict):
        for name, item in value.items():
            if isinstance(key, str) and name.lower() == key.lower():
                return item
        return None
    if isinstance(value, list) and isinstance(key, float) and key.is_integer() and 0 <= key < len(value):
        return value[int(key)]
    return None


def truthy(value) -> bool:
    if isinstance(value, float) and math.isnan(value):
        return False
    return value not in (None, False, 0, "")


def number(value) -> float:
    if value is None:
        return 0.0
    if isinstance(value, bool):
        return float(value)
    if isinstance(value, (int, float)):
        return float(value)
    if isinstance(value, str):
        try:
            return float(value.strip()) if value.strip() else 0.0
        except ValueError:
            return math.nan
    return math.nan


def loose_equal(left, right) -> bool:
    if isinstance(left, str) and isinstance(right, str):
        return left.lower() == right.lower()
    if type(left) is type(right) and not isinstance(left, (list, dict)):
        return left == right
    if isinstance(left, (list, dict)) or isinstance(right, (list, dict)):
        return left is right
    return number(left) == number(right)


def call(name: str, arguments: list):
    if name == "contains":
        search, item = arguments
        if isinstance(search, list):
            return any(loose_equal(element, item) for element in search)
        return str(item if item is not None else "").lower() in str(search if search is not None else "").lower()
    if name == "format":
        template, *values = arguments
        rendered = re.sub(r"\{(\d+)\}", lambda match: str(values[int(match.group(1))]),
                          template.replace("{{", "\0").replace("}}", "\1"))
        return rendered.replace("\0", "{").replace("\1", "}")
    if name == "fromJSON":
        return json.loads(arguments[0])
    if name == "cancelled":
        return False
    if name in ("always", "success"):
        return True
    raise ValueError(f"unsupported function {name}")


def evaluate(template: str, context: dict):
    """`${{ expr }}` alone yields the value; inside other text, a string."""
    whole = re.fullmatch(r"\$\{\{(.*)\}\}", template.strip(), re.DOTALL)
    if whole is not None and "${{" not in whole.group(1):
        return Expression(whole.group(1), context).evaluate()

    def render(match):
        value = Expression(match.group(1), context).evaluate()
        return "" if value is None else ("true" if value is True else "false" if value is False else str(value))
    return re.sub(r"\$\{\{(.*?)\}\}", render, template)


def jobs() -> dict[str, str]:
    text = WORKFLOW.read_text(encoding="utf-8").split("\njobs:\n", 1)[1]
    entries = re.split(r"(?m)^  ([a-z][a-z0-9-]+):\n", text)
    return dict(zip(entries[1::2], entries[2::2]))


def field(body: str, name: str, indent: int = 4) -> str | None:
    """A scalar job field, folding a `>-` block into one line."""
    prefix = " " * indent + name + ":"
    lines = body.splitlines()
    for index, line in enumerate(lines):
        if line.startswith(prefix + " ") or line == prefix:
            value = line[len(prefix):].strip()
            if value != ">-":
                return value
            folded = []
            for continuation in lines[index + 1:]:
                if not continuation.startswith(" " * (indent + 2)):
                    break
                folded.append(continuation.strip())
            return " ".join(folded)
    return None


def context(event: str, *, base: str | None = None, labels: tuple[str, ...] = (), draft: bool = False,
            ref: str | None = None, platform: str | None = None, shard: str | None = None,
            label: str | None = None, number_: int = 7) -> dict:
    payload: dict = {}
    if event == "pull_request":
        payload = {"action": "labeled" if label else "synchronize", "number": number_,
                   "pull_request": {"number": number_, "draft": draft,
                                    "labels": [{"name": name} for name in labels]}}
        if label:
            payload["label"] = {"name": label}
    github = {"event_name": event, "workflow": "Rust CI", "event": payload,
              "base_ref": base or "", "ref": ref or (f"refs/pull/{number_}/merge" if base else "refs/heads/main")}
    inputs = {"platform": platform, "shard": shard} if event == "workflow_dispatch" else {}
    return {"github": github, "inputs": inputs,
            "needs": {"scope": {"result": "success", "outputs": {"runtime_required": "true"}}}}


def test_matrix(ctx: dict) -> tuple[list[str], list[str]]:
    body = jobs()["test"]
    return (evaluate(field(body, "os", 8), ctx), evaluate(field(body, "shard", 8), ctx))


def reported_contexts(ctx: dict) -> set[str]:
    """Check-run names of the jobs whose guards pass for this event."""
    names = set()
    for job, body in jobs().items():
        guard = field(body, "if")
        if guard is not None and not truthy(Expression(guard, ctx).evaluate()):
            continue
        name = field(body, "name") or job
        if job == "test":
            platforms, shards = test_matrix(ctx)
            names |= {f"Run Tests ({os} / {shard})" for os in platforms for shard in shards}
        elif (axis := re.search(r"\$\{\{ matrix\.([a-z]+) \}\}", name)) is not None:
            values = re.search(rf"(?m)^        {axis.group(1)}: \[([^\]]+)\]$", body).group(1)
            names |= {name.replace(axis.group(0), value.strip()) for value in values.split(",")}
        else:
            names.add(name)
    return names


class MatrixTests(unittest.TestCase):
    def test_pull_requests_to_main_run_the_linux_gate_without_the_label(self):
        for labels in ((), ("dependencies", "rust"), ("full-matrix-later",)):
            with self.subTest(labels=labels):
                self.assertEqual(test_matrix(context("pull_request", base="main", labels=labels)),
                                 (LINUX, PR_GATE_SHARDS))
        unrelated = context("pull_request", base="main", labels=("documentation",), label="documentation")
        self.assertEqual(test_matrix(unrelated), (LINUX, PR_GATE_SHARDS))

    def test_full_matrix_label_runs_every_platform_and_shard(self):
        added = context("pull_request", base="main", labels=("full-matrix",), label="full-matrix")
        later_push = context("pull_request", base="main", labels=("rust", "full-matrix"))
        other_label_added = context("pull_request", base="main", labels=("full-matrix", "documentation"),
                                    label="documentation")
        for ctx in (added, later_push, other_label_added):
            with self.subTest(event=ctx["github"]["event"]):
                self.assertEqual(test_matrix(ctx), (ALL_PLATFORMS, ALL_SHARDS))

    def test_other_events_keep_their_matrices(self):
        expected = {
            "push main": (context("push", ref="refs/heads/main"), (LINUX, ALL_SHARDS)),
            "nightly": (context("schedule", ref="refs/heads/main"), (ALL_PLATFORMS, ALL_SHARDS)),
            "PR to v12": (context("pull_request", base="v12"), (ALL_PLATFORMS, ALL_SHARDS)),
            "manual all": (context("workflow_dispatch", platform="all", shard="all"), (ALL_PLATFORMS, ALL_SHARDS)),
            "manual diagnostic": (context("workflow_dispatch", platform="windows-latest", shard="cli-standard"),
                                  (["windows-latest"], ["cli-standard"])),
        }
        for name, (ctx, matrix) in expected.items():
            with self.subTest(event=name):
                self.assertEqual(test_matrix(ctx), matrix)

    def test_every_pull_request_run_reports_all_required_contexts(self):
        for labels, label in (((), None), (("full-matrix",), "full-matrix"), (("documentation",), "documentation"),
                              (("full-matrix",), None)):
            with self.subTest(labels=labels, label=label):
                ctx = context("pull_request", base="main", labels=labels, label=label)
                self.assertLessEqual(REQUIRED_CI_CONTEXTS, reported_contexts(ctx))
        labeled = reported_contexts(context("pull_request", base="main", labels=("full-matrix",), label="full-matrix"))
        self.assertIn("Run Tests (windows-latest / cli-saas-product)", labeled)
        self.assertIn("Run Tests (macos-latest / workspace)", labeled)
        # The label widens the OS matrix only: release-only and post-merge
        # jobs keep their guards.
        for job in ("Packaged distribution and installed CLI", "Public feature boundaries (0/4)",
                    "Threat-model release-negative minimum (3/4)", "Native CLI distribution",
                    "Evidence quality scorecard (observational)"):
            self.assertNotIn(job, labeled)
        self.assertIn("Public feature boundaries (3/4)", reported_contexts(context("push", ref="refs/heads/main")))

    def test_labeled_runs_share_the_pull_request_group_and_schedules_do_not(self):
        text = WORKFLOW.read_text(encoding="utf-8")
        group = re.search(r"(?m)^concurrency:\n(?:  #.*\n)*  group: (.+)$", text).group(1)
        synchronize = evaluate(group, context("pull_request", base="main"))
        labeled = evaluate(group, context("pull_request", base="main", labels=("full-matrix",), label="full-matrix"))
        self.assertEqual(synchronize, "Rust CI-7")
        self.assertEqual(labeled, synchronize)
        self.assertEqual(evaluate(group, context("pull_request", base="main", number_=8)), "Rust CI-8")
        self.assertEqual(evaluate(group, context("push", ref="refs/heads/main")), "Rust CI-refs/heads/main")
        self.assertEqual(evaluate(group, context("schedule", ref="refs/heads/main")), "Rust CI-refs/heads/main-nightly")
        self.assertRegex(text, r"(?m)^  pull_request:\n    branches: \[main, v12, v13\]\n(?:    #.*\n)*"
                               r"    types: \[[^\]]*\blabeled\b[^\]]*\]")

    def test_evaluator_follows_github_semantics(self):
        ctx = {"github": {"event": {"pull_request": {"labels": [{"name": "Full-Matrix"}]}}}}
        self.assertTrue(evaluate("${{ contains(github.event.pull_request.labels.*.name, 'full-matrix') }}", ctx))
        self.assertFalse(evaluate("${{ contains(github.event.missing.labels.*.name, 'full-matrix') }}", ctx))
        self.assertEqual(evaluate("${{ format('[\"{0}\"]', 'x') }}", {}), '["x"]')
        self.assertEqual(evaluate("${{ false && 'a' || 'b' }}", {}), "b")
        self.assertEqual(evaluate("${{ 'a' && 'c' || 'b' }}", {}), "c")
        self.assertTrue(evaluate("${{ !(1 == 2) && 'Main' == 'main' }}", {}))
        self.assertEqual(evaluate("x-${{ null }}-${{ true }}", {}), "x--true")


if __name__ == "__main__":
    unittest.main()
