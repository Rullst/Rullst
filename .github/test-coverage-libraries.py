#!/usr/bin/env python3
"""A well-covered CLI must not conceal an uncovered v13 framework library."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


class FrameworkCoverage(unittest.TestCase):
    def check(self, package, covered):
        def entry(name, count, hits):
            return {'filename': '/repository/' + name + '/src/lib.rs',
                    'summary': {'lines': {'count': count, 'covered': hits}}}
        report = {'data': [{'files': [entry('rullst-core', 100, 100),
                                     entry('cargo-rullst', 1800, 1800),
                                     entry(package, 100, covered)],
                            'totals': {'lines': {'count': 2000, 'covered': 1900 + covered}}}]}
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'summary.json'
            path.write_text(json.dumps(report))
            return subprocess.run([sys.executable, str(Path(__file__).with_name('check-coverage-threshold.py')), str(path)],
                                  text=True, capture_output=True, timeout=10)

    def test_new_libraries_participate_in_the_framework_floor(self):
        for package in ('rullst-privacy', 'rullst-supervision', 'rullst-media', 'rullst-labs'):
            with self.subTest(package=package):
                result = self.check(package, 0)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('Framework libraries', result.stdout)
                self.assertIn('is below 90%', result.stderr)

    def test_covered_library_passes_both_existing_floors(self):
        result = self.check('rullst-labs', 100)
        self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == '__main__':
    unittest.main()
