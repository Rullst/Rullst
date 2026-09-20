#!/usr/bin/env python3
"""Manual Linux pilot: pinned tools, source-linked projection and failing controls."""
from __future__ import annotations
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import resource
import signal
import stat
import subprocess
import time
import zipfile

ROOT = Path(__file__).resolve().parent.parent
CONFIG = ROOT / '.github/verus-toolchain.json'
MAX_LOG = 2 * 1024 * 1024
CASES = ('production', 'restricted_weakening', 'elevated_weakening', 'low_denial')


def digest(path: Path) -> str:
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def read_json(path: Path, limit: int = MAX_LOG) -> dict:
    if path.is_symlink() or not path.is_file() or path.stat().st_size > limit:
        raise ValueError('invalid bounded evidence file')
    value = json.loads(path.read_text())
    if not isinstance(value, dict):
        raise ValueError('evidence must be an object')
    return value


def inventory(archive: Path, config: dict) -> list[zipfile.ZipInfo]:
    if archive.stat().st_size != config['archive_bytes'] or digest(archive) != config['archive_sha256']:
        raise ValueError('Verus archive identity mismatch')
    with zipfile.ZipFile(archive) as bundle:
        entries = bundle.infolist()
    if len(entries) > config['maximum_archive_entries'] or sum(e.file_size for e in entries) > config['maximum_expanded_bytes']:
        raise ValueError('Verus archive budget exceeded')
    seen = set()
    for entry in entries:
        path = PurePosixPath(entry.filename)
        mode = entry.external_attr >> 16
        if (path.is_absolute() or '..' in path.parts or '\\' in entry.filename
                or not path.parts or path.parts[0] != config['archive_root']
                or entry.filename in seen or stat.S_ISLNK(mode)
                or entry.flag_bits & 1):
            raise ValueError('unsupported archive member')
        seen.add(entry.filename)
    return entries


def install(archive: Path, output: Path, config: dict) -> None:
    entries = inventory(archive, config)
    if output.exists() or output.is_symlink():
        raise ValueError('installation requires a fresh private directory')
    output.mkdir(mode=0o700, parents=True)
    with zipfile.ZipFile(archive) as bundle:
        bundle.extractall(output)
    for entry in entries:
        path = output / entry.filename
        if path.is_file():
            path.chmod((entry.external_attr >> 16) & 0o777 or 0o644)


def verify_bundle(archive: Path, directory: Path, config: dict) -> None:
    entries = inventory(archive, config)
    with zipfile.ZipFile(archive) as bundle:
        for entry in entries:
            target = directory / PurePosixPath(entry.filename).relative_to(config['archive_root'])
            if target.is_symlink():
                raise ValueError('linked verifier bundle member')
            if entry.is_dir():
                if not target.is_dir():
                    raise ValueError('missing verifier bundle directory')
                continue
            if not target.is_file() or target.stat().st_size != entry.file_size:
                raise ValueError('missing/changed verifier bundle member')
            with bundle.open(entry) as stream:
                expected = hashlib.file_digest(stream, 'sha256').hexdigest()
            if digest(target) != expected:
                raise ValueError('verifier bundle differs from pinned archive')


def linkage(projection: Path) -> dict:
    receipt = read_json(projection / 'linkage.json')
    if receipt.get('schema') != 'rullst.verus-linkage.v1' or receipt.get('entry_point') != 'AgePolicy::permits' or receipt.get('runtime_rewrites') != []:
        raise ValueError('unreviewed production linkage')
    sources = dict(receipt['module_source_sha256'])
    sources[receipt['production_file']] = receipt['source_sha256']
    sources['rullst-privacy/verification/age-policy.spec.rs'] = receipt['specification_sha256']
    expected = {'rullst-privacy/Cargo.toml', 'rullst-privacy/src/lib.rs', 'rullst-privacy/src/age_assurance/mod.rs', 'rullst-privacy/src/age_assurance/policy.rs', 'rullst-privacy/verification/age-policy.spec.rs'}
    if set(sources) != expected:
        raise ValueError('unreviewed source linkage inventory')
    for name, sha in sources.items():
        if digest(ROOT / name) != sha:
            raise ValueError('production or specification changed after projection')
    if [case['name'] for case in receipt['cases']] != list(CASES):
        raise ValueError('missing/reordered proof or negative control')
    for case in receipt['cases']:
        if case['file'] != case['name'] + '.rs':
            raise ValueError('unexpected proof path')
        path = projection / case['file']
        if path.is_symlink() or path.stat().st_size > 65536 or digest(path) != case['sha256']:
            raise ValueError('changed or unbounded proof input')
    return receipt


def check_result(result: dict, production: bool, config: dict) -> None:
    verification = result.get('verification-results', {})
    version = result.get('verus', {})
    if version.get('version') != config['release'] or version.get('commit') != config['commit']:
        raise ValueError('unexpected verifier identity')
    if not verification.get('is-verifying-entire-crate') or verification.get('encountered-vir-error'):
        raise ValueError('partial verification or invalid proof program')
    expected = (True, 1, 0) if production else (False, 0, 1)
    actual = (verification.get('success'), verification.get('verified'), verification.get('errors'))
    if actual != expected:
        raise ValueError('proof or deliberate failing control did not behave as required')
    if verification.get('encountered-error') != (not production):
        raise ValueError('inconsistent verification error state')
    functions = [f for module in result['times-ms']['smt']['smt-run-module-times'] for f in module.get('function-breakdown', [])]
    selected = [f for f in functions if f.get('function') == 'rullst_age_policy_pilot::AgePolicy::permits']
    if len(selected) != 1 or selected[0].get('success') != production:
        raise ValueError('expected production decision was not verified')


def child_limits() -> None:
    resource.setrlimit(resource.RLIMIT_AS, (2 * 1024**3, 2 * 1024**3))
    resource.setrlimit(resource.RLIMIT_CPU, (45, 45))
    resource.setrlimit(resource.RLIMIT_FSIZE, (MAX_LOG, MAX_LOG))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))


def prove(bundle: Path, archive: Path, projection: Path, output: Path) -> None:
    started = time.monotonic()
    config = read_json(CONFIG)
    verify_bundle(archive, bundle, config)
    receipt = linkage(projection)
    if output.exists() or output.is_symlink():
        raise ValueError('proof evidence requires a fresh directory')
    output.mkdir(mode=0o700, parents=True)
    env = {k: v for k, v in os.environ.items() if not k.startswith(('VERUS_', 'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS')) and k not in {'RUSTC_BOOTSTRAP', 'LD_PRELOAD', 'LD_LIBRARY_PATH'}}
    env['RUSTUP_TOOLCHAIN'] = config['rust_toolchain']
    env['VERUS_Z3_PATH'] = str(bundle / 'z3')
    solver = subprocess.run([str(bundle / 'z3'), '--version'], env=env, capture_output=True, text=True, check=True, timeout=10).stdout.strip()
    if solver != config['solver_version']:
        raise ValueError('unexpected solver version')
    records = []
    for case in CASES:
        stdout = output / (case + '.json')
        stderr = output / (case + '.stderr')
        usage = output / (case + '.usage')
        command = ['/usr/bin/time', '-f', '%e %M %U %S', '-o', str(usage), str(bundle / 'verus'), str(projection / (case + '.rs')), *config['flags']]
        before = time.monotonic()
        with stdout.open('wb') as out, stderr.open('wb') as err:
            child = subprocess.Popen(command, cwd=output, env=env, stdout=out, stderr=err, start_new_session=True, preexec_fn=child_limits)
            try:
                while child.poll() is None:
                    if time.monotonic() - before > 60 or stdout.stat().st_size > MAX_LOG or stderr.stat().st_size > MAX_LOG:
                        raise ValueError('proof exceeded its execution/output budget')
                    time.sleep(0.05)
            finally:
                if child.poll() is None:
                    os.killpg(child.pid, signal.SIGKILL)
                    child.wait()
        result = read_json(stdout)
        production = case == 'production'
        if (child.returncode == 0) != production:
            raise ValueError('unexpected verifier process status')
        check_result(result, production, config)
        elapsed, peak_kib, user, system = usage.read_text().strip().splitlines()[-1].split()
        records.append({'case': case, 'exit_code': child.returncode, 'expected_success': production, 'wall_seconds': float(elapsed), 'peak_rss_kib': int(peak_kib), 'cpu_user_seconds': float(user), 'cpu_system_seconds': float(system), 'result_sha256': digest(stdout)})
    # The evidence must still describe the same source after every solver run.
    if linkage(projection) != receipt:
        raise ValueError('source changed during verification')
    git = subprocess.run(['git', 'rev-parse', 'HEAD'], cwd=ROOT, capture_output=True, text=True, check=True).stdout.strip()
    dirty = bool(subprocess.run(['git', 'status', '--porcelain'], cwd=ROOT, capture_output=True, text=True, check=True).stdout.strip())
    evidence = {'schema': 'rullst.verus-evidence.v1', 'git_sha': git, 'dirty_worktree': dirty, 'toolchain': config, 'linkage': receipt, 'results': records, 'total_wall_seconds': round(time.monotonic()-started,3), 'scope': 'AgePolicy::permits risk/method predicate only; no age evidence, time, state, authorization or legal-compliance proof'}
    (output / 'evidence.json').write_text(json.dumps(evidence, indent=2)+'\n')
    print(json.dumps({'verified_properties':1,'failed_negative_controls':3,'evidence':str(output/'evidence.json'),'total_seconds':evidence['total_wall_seconds']}))


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=('install', 'verify'))
    parser.add_argument('--archive', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--bundle', type=Path)
    parser.add_argument('--projection', type=Path)
    args = parser.parse_args()
    if args.mode == 'install':
        install(args.archive.resolve(), args.output.absolute(), read_json(CONFIG))
    else:
        if args.bundle is None or args.projection is None:
            parser.error('verify requires --bundle and --projection')
        prove(args.bundle.resolve(), args.archive.resolve(), args.projection.resolve(), args.output.absolute())


if __name__ == '__main__':
    main()
