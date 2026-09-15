#!/usr/bin/env python3
"""Keep the executable upgrade allowlist aligned with published packages."""

import json
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]


class UpgradePackageInventory(unittest.TestCase):
    def test_allowlist_matches_release_inventory_without_duplicates(self):
        published = json.loads((ROOT / ".github/release-order.json").read_text())
        source = (ROOT / "cargo-rullst/src/generators/build/upgrade/manifest.rs").read_text()
        declaration = re.search(r"const RULLST_PACKAGES: &\[&str\] = &\[(.*?)\];", source, re.S)
        self.assertIsNotNone(declaration, "the checked allowlist declaration must remain recognizable")
        managed = re.findall(r'"([a-z0-9-]+)"', declaration.group(1))
        self.assertEqual(len(managed), len(set(managed)), "duplicate managed package")
        self.assertEqual(set(managed), set(published), "published packages must participate in upgrades")


if __name__ == "__main__":
    unittest.main()
