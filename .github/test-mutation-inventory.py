#!/usr/bin/env python3
"""Execute the workflow preflight against changing and invalid inventories."""

from __future__ import annotations

import hashlib
import json
import os
import re
import subprocess
import tempfile
import textwrap
import unittest
from pathlib import Path


WORKFLOW = Path(__file__).parent / "workflows" / "mutants.yml"
MATCH = re.search(
    r"      - name: Discover and validate the exact candidate inventory\n"
    r".*?        run: \|\n(.*?)(?=\n      - name:)",
    WORKFLOW.read_text(), re.DOTALL,
)
if MATCH is None:
    raise RuntimeError("Cannot locate the mutation workflow preflight.")
PREFLIGHT = textwrap.dedent(MATCH.group(1))


class MutationInventoryTests(unittest.TestCase):
    def run_inventory(self, inventory: object, expected: str = "") -> tuple[int, dict[str, str]]:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            fixture = root / "fixture.json"
            fixture.write_text(json.dumps(inventory))
            bin_dir = root / "bin"
            bin_dir.mkdir()
            cargo = bin_dir / "cargo"
            cargo.write_text('#!/bin/sh\ncat "$MUTATION_FIXTURE"\n')
            cargo.chmod(0o700)
            output = root / "outputs"
            env = dict(os.environ, PATH=f"{bin_dir}:{os.environ['PATH']}",
                       MUTATION_FIXTURE=str(fixture), GITHUB_OUTPUT=str(output),
                       MUTATION_MODE="full", MUTATION_FILE="",
                       EXPECTED_INVENTORY_SHA256=expected)
            result = subprocess.run(
                ["bash", "-euo", "pipefail", "-c", PREFLIGHT],
                cwd=root, env=env, text=True, capture_output=True, check=False,
            )
            outputs = dict(line.split("=", 1) for line in output.read_text().splitlines()) \
                if output.exists() else {}
            return result.returncode, outputs

    def test_new_campaign_discovers_current_inventory_instead_of_historical_count(self) -> None:
        inventory = [{"name": f"crate/src/lib.rs: mutant {index}"} for index in range(17549)]
        status, outputs = self.run_inventory(inventory)
        self.assertEqual(status, 0)
        self.assertEqual(outputs["inventory_count"], "17549")
        self.assertEqual(outputs["inventory_sha256"],
                         hashlib.sha256(json.dumps(inventory).encode()).hexdigest())

    def test_recovery_requires_exact_historical_digest(self) -> None:
        inventory = [{"name": "original mutant"}]
        digest = hashlib.sha256(json.dumps(inventory).encode()).hexdigest()
        self.assertEqual(self.run_inventory(inventory, digest)[0], 0)
        self.assertNotEqual(self.run_inventory([{"name": "changed mutant"}], digest)[0], 0)

    def test_rejects_empty_duplicate_or_malformed_inventory(self) -> None:
        for inventory in ([], {}, [{"name": ""}], [{"name": 1}], [{}],
                          [{"name": "same"}, {"name": "same"}]):
            with self.subTest(inventory=inventory):
                status, outputs = self.run_inventory(inventory)
                self.assertNotEqual(status, 0)
                self.assertEqual(outputs, {})


if __name__ == "__main__":
    unittest.main()
