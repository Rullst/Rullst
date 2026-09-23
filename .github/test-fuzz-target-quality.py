#!/usr/bin/env python3
"""Reject obvious inert harnesses; behavioral contracts remain separate."""

import json
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]


def inert(source: str) -> bool:
    # This deliberately detects only a narrow, known stub shape. It is not a
    # Rust parser or a proof of semantic coverage for arbitrary harness code.
    source = re.sub(r"/\*.*?\*/|//[^\n]*", "", source, flags=re.DOTALL)
    return re.search(r"fuzz_target!\s*\(\s*\|[^|]+\|\s*\{\s*"
                     r"(?:let\s+_\s*=\s*[A-Za-z_][A-Za-z0-9_]*\s*;\s*)*\}\s*\)", source) is not None


class QualityTests(unittest.TestCase):
    def test_empty_and_discard_only_entries_are_rejected(self):
        self.assertTrue(inert("fuzz_target!(|data: &[u8]| {});"))
        self.assertTrue(inert("fuzz_target!(|data: &[u8]| { /* later */ let _ = data; });"))
        self.assertFalse(inert("fuzz_target!(|data: &[u8]| { let _ = parse(data); });"))

    def test_inventory_contains_no_trivially_inert_target(self):
        for entry in json.loads((ROOT / ".github/fuzz-targets.json").read_text()):
            path = ROOT / entry["dir"] / "fuzz_targets" / (entry["target"] + ".rs")
            with self.subTest(target=entry["target"]):
                self.assertFalse(inert(path.read_text()), str(path))

    def test_repaired_entries_dispatch_their_executable_contracts(self):
        for target, function in {"auth_session": "session", "config_parser": "config",
                                 "multitenant_resolver": "tenants", "ws_payload": "realtime"}.items():
            source = (ROOT / "rullst/fuzz/fuzz_targets" / (target + ".rs")).read_text()
            self.assertIn("rullst_fuzz::" + function + "(", source)


if __name__ == "__main__":
    unittest.main()
