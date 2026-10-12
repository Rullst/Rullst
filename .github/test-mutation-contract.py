#!/usr/bin/env python3
"""Fake docker and cargo fixtures for the owned-server mutation mode."""

from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import os
import subprocess
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path


HERE = Path(__file__).parent
HELPER = HERE / "mutation_contract.py"
SPEC = importlib.util.spec_from_file_location("mutation_contract", HELPER)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot import {HELPER}")
CONTRACT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CONTRACT)

class Fixture(unittest.TestCase):
    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.bin = self.root / "bin"
        self.bin.mkdir()
        self.calls = self.root / "calls.jsonl"

    def tool(self, name: str, body: str) -> None:
        path = self.bin / name
        # An absolute interpreter: a fake python3 must not resolve to itself.
        path.write_text(f"#!{sys.executable}\nimport json, os, sys\n"
                        f"open({str(self.calls)!r}, 'a').write(json.dumps([{name!r}] + sys.argv[1:]) + '\\n')\n"
                        + textwrap.dedent(body))
        path.chmod(0o700)

    def recorded(self) -> list[list[str]]:
        if not self.calls.exists():
            return []
        return [json.loads(line) for line in self.calls.read_text().splitlines()]

    def env(self, **extra: str) -> dict[str, str]:
        return dict(os.environ, PATH=f"{self.bin}:{os.environ['PATH']}", **extra)


class RunnerTests(Fixture):
    def runner(self, spec: dict[str, str], **extra: str) -> subprocess.CompletedProcess[str]:
        self.tool("binary", "")
        self.tool("inner", "os.execvp(sys.argv[1], sys.argv[1:])\n")
        env = self.env(RULLST_MUTANTS_RESET=json.dumps(spec),
                       RULLST_MUTANTS_INNER_RUNNER=json.dumps(["inner"]), **extra)
        return subprocess.run([sys.executable, str(HELPER), "binary", "--exact", "a"],
                              env=env, text=True, capture_output=True, check=False)

    def test_postgres_reset_recreates_state_before_the_binary_starts(self) -> None:
        self.tool("docker", "print('t' if 'pg_settings' in ' '.join(sys.argv) else '1')\n")
        result = self.runner(CONTRACT.postgres("owned", "rullst_x_contract"))
        self.assertEqual(result.returncode, 0, result.stderr)
        calls = self.recorded()
        self.assertEqual([call[0] for call in calls], ["docker", "docker", "inner", "binary"])
        commands = [arg for arg in calls[0][calls[0].index("postgres") + 1:] if arg != "--command"]
        self.assertEqual(commands[-6:], [
            CONTRACT.TERMINATE, 'DROP DATABASE IF EXISTS "rullst_x_contract"', CONTRACT.DROP_ROLES,
            "ALTER SYSTEM RESET ALL", "SELECT pg_catalog.pg_reload_conf()",
            'CREATE DATABASE "rullst_x_contract"'])
        self.assertEqual(calls[1][-1], CONTRACT.APPLIED)
        self.assertEqual(calls[3], ["binary", "--exact", "a"])

    def test_redis_reset_flushes_with_the_password_from_the_environment(self) -> None:
        self.tool("docker", "print('OK')\n")
        result = self.runner(CONTRACT.redis("owned", "PASSWORD_FIXTURE"), PASSWORD_FIXTURE="pw")
        self.assertEqual(result.returncode, 0, result.stderr)
        calls = self.recorded()
        self.assertEqual(calls[0], ["docker", "exec", "--env", "REDISCLI_AUTH=pw", "owned",
                                    "redis-cli", "FLUSHALL", "SYNC"])
        self.assertEqual(calls[-1][0], "binary")

    def test_failed_reset_never_starts_the_binary_and_is_recorded(self) -> None:
        failures = self.root / "failures.log"
        for body in ("sys.exit(1)\n", "print('ERR')\n"):
            with self.subTest(body=body):
                self.calls.unlink(missing_ok=True)
                self.tool("docker", body)
                result = self.runner(CONTRACT.redis("owned", "PASSWORD_FIXTURE"),
                                     PASSWORD_FIXTURE="pw", RULLST_MUTANTS_RESET_FAILURES=str(failures))
                self.assertEqual(result.returncode, 101)
                self.assertNotIn("binary", [call[0] for call in self.recorded()])
        self.assertEqual(failures.read_text().splitlines(), ["binary: CalledProcessError",
                                                             "binary: RuntimeError"])

    def test_database_names_are_plain_identifiers(self) -> None:
        for name in ("", "a b", 'x"; DROP', "Upper"):
            with self.subTest(name=name), self.assertRaises(ValueError):
                CONTRACT.postgres("owned", name)


class RunTests(Fixture):
    def run_contract(self, baseline: str | None, status: int, shard: str | None = "1/3",
                     reset_failure: bool = False) -> int:
        self.tool("rustc", "print('rustc 1.99.0\\nhost: x86_64-unknown-linux-gnu')\n")
        log = self.root / "mutants.out" / "log"
        # cargo-mutants moves an earlier mutants.out aside before it starts.
        body = (f"import shutil\nshutil.rmtree({str(log.parent)!r}, ignore_errors=True)\n"
                f"os.makedirs({str(log)!r})\n")
        if baseline is not None:
            body += f"open({str(log / 'baseline.log')!r}, 'w').write({baseline!r})\n"
        if reset_failure:
            body += "open(os.environ['RULLST_MUTANTS_RESET_FAILURES'], 'a').write('a: X\\n')\n"
        body += ("open(os.environ['CALLS_ENV'], 'w').write(json.dumps(dict(os.environ)))\n"
                 f"sys.exit({status})\n")
        self.tool("cargo", body)
        env = self.env(CARGO_MUTANTS_OUTPUT=str(self.root), CALLS_ENV=str(self.root / "env.json"),
                       CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER="prlimit --as=1 --")
        self.stderr = io.StringIO()
        with contextlib.redirect_stderr(self.stderr):
            return CONTRACT.run(package="crate", features="a,b", files=["crate/src/x/**"],
                                targets=["--lib", "--test=t"], tests=["m::one", "two"], regex="re",
                                shard=shard, env=env, reset_spec=CONTRACT.postgres("c", "d"),
                                timeout=7)

    def test_command_runner_and_informational_status(self) -> None:
        # A child sharing stdout may print before libtest's `ok`.
        baseline = "test m::one ... child output\nok\ntest two ... ok\n"
        self.assertEqual(self.run_contract(baseline, 2), 2)
        call = self.recorded()[-1]
        self.assertEqual(call[:4], ["cargo", "mutants", "--no-config", "--package"])
        for part in (["--features", "a,b"], ["--timeout", "7"], ["--file", "crate/src/x/**"],
                     ["--re", "re"], ["--shard", "1/3"], ["--cargo-arg=--lib", "--cargo-arg=--test=t"]):
            with self.subTest(part=part):
                self.assertIn(part, [call[i:i + len(part)] for i in range(len(call))])
        self.assertIn("--no-default-features", call)
        self.assertEqual(call[call.index("--"):], ["--", "--", "--ignored", "--exact",
                                                    "--test-threads=1", "m::one", "two"])
        env = json.loads((self.root / "env.json").read_text())
        self.assertEqual(env["CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER"],
                         f"{sys.executable} {HELPER.resolve()}")
        self.assertEqual(json.loads(env["RULLST_MUTANTS_INNER_RUNNER"]), ["prlimit", "--as=1", "--"])
        self.assertEqual(json.loads(env["RULLST_MUTANTS_RESET"]),
                         {"kind": "postgres", "container": "c", "database": "d"})

    def test_rejects_unrun_contracts_missing_baselines_and_reset_failures(self) -> None:
        self.assertEqual(self.run_contract("test two ... ok\n", 0), 1)
        self.assertIn("did not run these contract tests: m::one", self.stderr.getvalue())
        self.assertEqual(self.run_contract(None, 0, shard=None), 1)
        self.assertIn("ran no baseline", self.stderr.getvalue())
        self.assertEqual(self.run_contract("test m::one ... ok\ntest two ... ok\n", 3,
                                           reset_failure=True), 1)
        self.assertIn("could not be reset", self.stderr.getvalue())
        self.assertEqual(self.run_contract("test m::one ... ok\ntest two ... ok\n", 3), 3)

    def test_baseline_and_tool_failures_pass_through(self) -> None:
        for status in (1, 4, 70):
            with self.subTest(status=status):
                self.assertEqual(self.run_contract(None, status), status)

    def test_rejects_malformed_shards(self) -> None:
        for shard in ("1", "1/0", "a/b", "1/3/4"):
            with self.subTest(shard=shard), self.assertRaises(ValueError):
                self.run_contract("", 0, shard=shard)


if __name__ == "__main__":
    unittest.main()
