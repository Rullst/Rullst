#!/usr/bin/env python3
"""Verify measurement custody without launching a compiler, worker or service."""
import importlib.util
from pathlib import Path
import stat
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('prepare', Path(__file__).with_name('prepare-labs-fixture.py'))
prepare = importlib.util.module_from_spec(spec)
spec.loader.exec_module(prepare)


class ControllerMeasurement(unittest.TestCase):
    def test_private_copy_cannot_change_or_enter_worker_tree(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            fixture = root / 'fixture'
            fixture.mkdir(mode=0o700)
            worker_tree = fixture / 'rootfs'
            worker_tree.mkdir(mode=0o755)
            worker = worker_tree / 'runner'
            worker.write_bytes(b'trusted uninstrumented worker')
            source = root / 'measured-build'
            source.write_bytes(b'trusted measured controller')
            source.chmod(0o777)  # Developer build mode must not be preserved.
            controller = prepare.copy_controller(fixture, source)
            self.assertFalse(controller.is_relative_to(worker_tree))
            self.assertEqual(controller.read_bytes(), source.read_bytes())
            self.assertEqual(stat.S_IMODE(controller.stat().st_mode), 0o755)
            self.assertEqual(stat.S_IMODE(fixture.stat().st_mode), 0o700)
            source.write_bytes(b'later build')
            self.assertEqual(controller.read_bytes(), b'trusted measured controller')
            self.assertEqual(worker.read_bytes(), b'trusted uninstrumented worker')
            self.assertEqual(list(worker_tree.iterdir()), [worker])

    def test_existing_destination_or_link_is_never_overwritten(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / 'source'
            source.write_bytes(b'measurement')
            target = root / 'controller'
            target.write_bytes(b'previous')
            with self.assertRaises(FileExistsError):
                prepare.copy_controller(root, source)
            self.assertEqual(target.read_bytes(), b'previous')
            target.unlink()
            target.symlink_to(source)
            with self.assertRaises(FileExistsError):
                prepare.copy_controller(root, source)
            self.assertEqual(source.read_bytes(), b'measurement')


if __name__ == '__main__':
    unittest.main()
