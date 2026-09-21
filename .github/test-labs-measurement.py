#!/usr/bin/env python3
"""Exercise the CI-only profiling hook using a tiny, trusted Rust fixture.

No learner code, namespace, service or framework build is launched. The ordinary
instrumented control proves the runtime environment mutation that must be absent
from the worker; the same hooked binary must still emit valid host profile data.
"""
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parent


class ControllerMeasurement(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temporary = tempfile.TemporaryDirectory(prefix='rullst-profile-hook-')
        cls.addClassCleanup(cls.temporary.cleanup)
        cls.root = Path(cls.temporary.name)
        available = shutil.disk_usage(cls.root).free
        if available < 15 * 1024**3 or available - 32 * 1024**2 < 12 * 1024**3:
            raise RuntimeError('insufficient headroom for the bounded profiling fixture')
        source = cls.root / 'probe.rs'
        source.write_text('fn main() { println!("{}", std::env::vars_os().count()); }\n')
        hook = cls.root / 'profile-runtime.o'
        subprocess.run(['cc', '-std=c11', '-Wall', '-Wextra', '-Werror', '-c',
                        str(ROOT / 'labs-profile-runtime.c'), '-o', str(hook)],
                       check=True, capture_output=True, timeout=30)
        linker = cls.root / 'linker'
        shutil.copyfile(ROOT / 'labs-profile-linker.sh', linker)
        linker.chmod(0o755)
        for name, extra in [('ordinary', []), ('host-only', ['-C', 'linker-flavor=gcc', '-C', 'linker=' + str(linker)])]:
            subprocess.run(['rustc', '--edition=2024', '-C', 'instrument-coverage',
                            str(source), '-o', str(cls.root / name), *extra],
                           check=True, timeout=60)

    def execute(self, name, environment):
        directory = self.root / self.id().rsplit('.', 1)[-1]
        directory.mkdir()
        result = subprocess.run([str(self.root / name)], cwd=directory,
                                env=environment, text=True, capture_output=True,
                                check=True, timeout=10)
        return directory, result

    def test_control_exposes_automatic_runtime_side_effects(self):
        directory, result = self.execute('ordinary', {})
        self.assertEqual(result.stdout.strip(), '1')
        self.assertTrue(list(directory.glob('*.profraw')))

    def test_cleared_worker_environment_is_unchanged_and_writes_nothing(self):
        directory, result = self.execute('host-only', {})
        self.assertEqual(result.stdout.strip(), '0')
        self.assertEqual(result.stderr, '')
        self.assertEqual(list(directory.iterdir()), [])

    def test_empty_profile_path_does_not_initialize_runtime(self):
        directory, result = self.execute('host-only', {'LLVM_PROFILE_FILE': ''})
        self.assertEqual(result.stdout.strip(), '1')
        self.assertEqual(result.stderr, '')
        self.assertEqual(list(directory.iterdir()), [])

    def test_explicit_host_path_collects_nonempty_profile(self):
        profile = self.root / 'trusted-host.profraw'
        directory, result = self.execute('host-only', {'LLVM_PROFILE_FILE': str(profile)})
        self.assertEqual(result.stdout.strip(), '2')
        self.assertEqual(result.stderr, '')
        self.assertGreater(profile.stat().st_size, 0)
        self.assertEqual(list(directory.iterdir()), [])


if __name__ == '__main__':
    unittest.main()
