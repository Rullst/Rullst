#!/usr/bin/env python3
"""Prepare only an owned disposable Labs fixture from trusted installed tools.

No student source is compiled or executed here. No package download, global
configuration change, application key, container socket or live service is used.
"""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess


def prepare(directory, runner, toolchain, launcher, cgroups):
    directory = directory.absolute()
    if directory.exists():
        raise ValueError("fixture destination must not exist")
    runner, toolchain, launcher = (p.resolve(strict=True) for p in (runner, toolchain, launcher))
    if not (toolchain / 'lib/rustlib/wasm32-unknown-unknown/lib').is_dir():
        raise ValueError('install the pinned Rust 1.96.0 wasm32 target before preparing the fixture')
    version = subprocess.check_output([str(toolchain / 'bin/rustc'), '--version'], timeout=10, text=True)
    if not version.startswith('rustc 1.96.0 '):
        raise ValueError('unexpected Rust toolchain')
    files = {Path('runner'): runner, Path('toolchain/bin/rustc'): toolchain / 'bin/rustc'}
    linker = toolchain / 'lib/rustlib/x86_64-unknown-linux-gnu/bin/rust-lld'
    files[Path('toolchain/lib/rustlib/x86_64-unknown-linux-gnu/bin/rust-lld')] = linker
    for source in sorted((toolchain / 'lib').glob('*.so*')):
        files[Path('toolchain/lib') / source.name] = source.resolve(strict=True)
    stdlib = toolchain / 'lib/rustlib/wasm32-unknown-unknown/lib'
    for source in sorted(stdlib.iterdir()):
        if source.is_file():
            files[Path('toolchain/lib/rustlib/wasm32-unknown-unknown/lib') / source.name] = source.resolve(strict=True)
    # ldd is invoked only on the trusted toolchain/runner installed by the test
    # operator. It is never applied to submitted code or a generated artifact.
    for source in [runner, toolchain / 'bin/rustc', linker, *sorted((toolchain / 'lib').glob('*.so*'))]:
        with source.open('rb') as file:
            if file.read(4) != b'\x7fELF':
                continue  # A shipped LLVM linker script is data, not a loader target.
        result = subprocess.run(['ldd', str(source)], capture_output=True, text=True, timeout=10, check=False)
        if result.returncode != 0:
            raise ValueError('trusted executable runtime could not be resolved')
        for name in re.findall(r'(?:=>\s+|^\s*)(/[^\s]+)', result.stdout, re.MULTILINE):
            dependency = Path(name).resolve(strict=True)
            if dependency.is_relative_to(toolchain):
                continue
            relative = Path('runtime/lib64' if 'ld-linux' in dependency.name else 'runtime/lib') / Path(name).name
            previous = files.get(relative)
            if previous is not None and previous != dependency:
                raise ValueError('ambiguous runtime library identity')
            files[relative] = dependency
    total = sum(path.stat().st_size for path in files.values())
    available = shutil.disk_usage(directory.parent).free
    if available < 15 * 1024**3 or available - total < 12 * 1024**3:
        raise ValueError('insufficient user-available disk headroom for the disposable fixture')
    directory.mkdir(mode=0o700)
    rootfs = directory / 'rootfs'
    for relative, source in files.items():
        target = rootfs / relative
        target.parent.mkdir(mode=0o755, parents=True, exist_ok=True)
        shutil.copyfile(source, target)
        target.chmod(0o755 if relative in (Path('runner'), Path('toolchain/bin/rustc'), Path('toolchain/lib/rustlib/x86_64-unknown-linux-gnu/bin/rust-lld')) else 0o644)
    for path in [rootfs, *rootfs.rglob('*')]:
        if path.is_dir():
            path.chmod(0o755)
    for name in ('content-key', 'receipt-seed'):
        fd = os.open(directory / name, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        with os.fdopen(fd, 'wb') as output:
            output.write(os.urandom(32))
    described = subprocess.check_output([str(runner), 'describe-profile', str(rootfs), str(launcher), str(cgroups), str(directory / 'receipt-seed')], timeout=30)
    linux = json.loads(described)
    config = {'linux': linux, 'plane': {'database': str(directory / 'jobs.sqlite'), 'namespace': 'isolated-acceptance', 'max_jobs': 128, 'max_exercises': 32, 'content_key': str(directory / 'content-key'), 'receipt_seed': str(directory / 'receipt-seed')}}
    config_path = directory / 'runner.json'
    config_path.write_text(json.dumps(config, indent=2) + '\n')
    config_path.chmod(0o600)
    application = {key: value for key, value in config['plane'].items() if key != 'receipt_seed'}
    application['profile'] = linux['profile']
    (directory / 'application.json').write_text(json.dumps(application, indent=2) + '\n')
    (directory / 'application.json').chmod(0o600)
    # The manifest contains only public tool identities, never fixture key bytes.
    (directory / 'tools.json').write_text(json.dumps(linux['profile'], indent=2) + '\n')
    return config_path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for field in ('directory', 'runner', 'toolchain', 'launcher', 'cgroups'):
        parser.add_argument('--' + field, required=True, type=Path)
    args = parser.parse_args()
    config = prepare(args.directory, args.runner, args.toolchain, args.launcher, args.cgroups)
    print(config)


if __name__ == '__main__':
    main()
