#!/usr/bin/env python3
"""A reviewed bootstrap name must belong to the exact release inventory."""
import json
from pathlib import Path
import subprocess
import unittest

ROOT = Path(__file__).resolve().parent


class OwnershipPolicy(unittest.TestCase):
    def accepts(self, names):
        return subprocess.run(
            ['jq', '-e', '--argjson', 'order', '[["rullst", "rullst-privacy"]]',
             '-f', str(ROOT / 'crates-ownership-policy.jq')],
            input=json.dumps({'expected_owner': 'venelouis', 'bootstrap_unregistered': names}),
            text=True, capture_output=True, check=False).returncode == 0

    def test_each_name_is_checked_against_inventory(self):
        for names in ([], ['rullst-privacy'], ['rullst', 'rullst-privacy']):
            self.assertTrue(self.accepts(names))
        for names in (['unreviewed'], ['rullst-privacy', 'unreviewed'],
                      ['rullst-privacy', 'rullst-privacy'], [None], [['rullst']], None, {}):
            self.assertFalse(self.accepts(names))


if __name__ == '__main__':
    unittest.main()
