#!/usr/bin/env python3
"""Mandatory isolated Labs acceptance on an owned disposable systemd service.

Run as an unprivileged dedicated service with Delegate=yes. This script only
moves its own PID into its own service's controller subgroup. It never changes
global sysctls or runs student code directly. Unsupported isolation is failure.
"""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import selectors
import shutil
import sqlite3
import subprocess
import tempfile
import time

SCOPE = {'tenant': 'school', 'course': 'rust'}
STRESS = '#[allow(long_running_const_eval)] const COST: i64 = { let mut n=0i64; while n<2000000000 {n+=1;} n }; pub fn solve(_:i64,_:i64)->i64 { COST }'


def delegated_jobs():
    if os.getuid() == 0 or os.environ.get('RULLST_LABS_HOSTED_ACCEPTANCE') != '1':
        raise RuntimeError('requires the explicit unprivileged disposable acceptance service')
    relative = Path('/proc/self/cgroup').read_text().strip().removeprefix('0::').lstrip('/')
    root = Path('/sys/fs/cgroup') / relative
    if not root.name.startswith('rullst-labs-acceptance-') or not root.name.endswith('.service'):
        raise RuntimeError('refusing to modify a cgroup outside the owned acceptance service')
    if root.joinpath('cgroup.procs').read_text().split() != [str(os.getpid())]:
        raise RuntimeError('service root contains a process not owned by this test')
    control = root / 'controller'
    control.mkdir()
    control.joinpath('cgroup.procs').write_text(str(os.getpid()))
    root.joinpath('cgroup.subtree_control').write_text('+memory +pids +cpu')
    jobs = root / 'jobs'
    jobs.mkdir()
    jobs.joinpath('cgroup.subtree_control').write_text('+memory +pids +cpu')
    jobs.joinpath('cgroup.max.descendants').write_text('32')
    return jobs


class Application:
    def __init__(self, executable, config):
        initialized = subprocess.run([str(executable), 'initialize', str(config)], capture_output=True, timeout=10)
        if initialized.returncode:
            raise RuntimeError('application job-plane initialization failed')
        self.process = subprocess.Popen([str(executable), 'serve', str(config)], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, bufsize=1)
        self.selector = selectors.DefaultSelector()
        self.selector.register(self.process.stdout, selectors.EVENT_READ)

    def request(self, actor, operation, scope=SCOPE):
        self.process.stdin.write(json.dumps({'actor': actor, 'scope': scope, 'operation': operation}) + '\n')
        self.process.stdin.flush()
        if not self.selector.select(12):
            raise RuntimeError('application response deadline exceeded')
        line = self.process.stdout.readline(131073)
        if not line.endswith('\n') or len(line) > 131072:
            raise RuntimeError('application response exceeded its frame contract')
        return json.loads(line)

    def success(self, actor, operation):
        reply = self.request(actor, operation)
        if 'ok' not in reply:
            raise RuntimeError('authorized application operation failed')
        return reply['ok']

    def close(self):
        self.process.stdin.close()
        try:
            self.process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait(timeout=5)
        self.selector.close()


def exercise(name, wall=10):
    return {'scope': SCOPE, 'id': name, 'revision': 'v1', 'cases': [{'id': 'private-one', 'input': [3, 5], 'expected': 8}, {'id': 'private-two', 'input': [-6, 11], 'expected': 5}], 'limits': {'wall_seconds': wall, 'fuel_per_case': 1000000, 'memory_pages': 64}}


def submission(name, source, exercise_id='sum'):
    return {'id': name, 'exercise': {'id': exercise_id, 'revision': 'v1'}, 'source': source, 'ttl_seconds': 300}


def run_runner(runner, config):
    result = subprocess.run([str(runner), 'run-once', str(config)], capture_output=True, timeout=90)
    if len(result.stdout) > 32768 or len(result.stderr) > 8192:
        raise RuntimeError('controller output exceeded its contract')
    return result


def compiler_running(groups):
    for group in groups.iterdir():
        if not group.is_dir():
            continue
        try:
            for pid in (group / 'cgroup.procs').read_text().split():
                if b'/work/submission.rs' in (Path('/proc') / pid / 'cmdline').read_bytes():
                    return True
        except FileNotFoundError:
            continue
    return False


def accept(args, directory, groups):
    spec = importlib.util.spec_from_file_location('prepare_labs', Path(__file__).with_name('prepare-labs-fixture.py'))
    prepare = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(prepare)
    config_path = prepare.prepare(directory, args.runner, args.toolchain, args.launcher, groups, args.controller)
    config = json.loads(config_path.read_text())
    runner = directory / 'controller' if args.controller is not None else Path(config['linux']['rootfs']) / 'runner'
    # The actual controller runs from its verified private fixture copy, not a
    # group-writable Cargo target file left by a developer's umask.
    try:
        for maximum in ('max', '0', '33'):
            (groups / 'cgroup.max.descendants').write_text(maximum)
            refused = subprocess.run([str(runner), 'doctor', str(config_path)], capture_output=True, timeout=40)
            assert refused.returncode != 0, 'unsupported shared group capacity must be refused'
    finally:
        (groups / 'cgroup.max.descendants').write_text('32')
    preflight = subprocess.run([str(runner), 'doctor', str(config_path)], capture_output=True, timeout=40)
    if preflight.returncode:
        allowed = {'labs-preflight:configuration', 'labs-preflight:execution-boundary', 'labs-preflight:launch', 'labs-preflight:cgroup', 'labs-preflight:seccomp', 'labs-preflight:worker-probes', 'labs-preflight:landlock', 'labs-preflight:compiler-domain', 'labs-preflight:namespace-launcher'}
        allowed.update('labs-preflight:' + phase for phase in ('privileges', 'uid-map', 'limits', 'mounts', 'network', 'workspace', 'descriptors', 'environment', 'compiler', 'namespaces'))
        allowed.update('labs-preflight:' + phase for phase in ('namespace-permission', 'namespace-create', 'launcher-id-map', 'launcher-exec', 'launcher-options', 'launcher-mount', 'launcher-loopback', 'launcher-userns-lock', 'launcher-privileges', 'launcher-layout'))
        allowed.update('labs-preflight:landlock-' + phase for phase in ('create', 'rules', 'restrict', 'enforcement', 'proc-denial', 'cgroup-denial'))
        for line in preflight.stderr.decode('utf-8', errors='replace').splitlines():
            if line in allowed:
                print(line, flush=True)
        raise RuntimeError('mandatory real Linux isolation preflight failed; no submissions executed')
    doctor = json.loads(preflight.stdout)
    app = Application(args.app, directory / 'application.json')
    controllers = []
    checks = ['actual-isolation-preflight', 'compiler-parent-descriptors-denied', 'kernel-group-capacity-required']
    try:
        for name, wall in [('sum', 10), ('stress', 5)]:
            app.success('teacher', {'Register': {'exercise': exercise(name, wall)}})
        source = 'pub fn solve(a:i64,b:i64)->i64 { a+b }'
        first = app.success('alice', {'Submit': {'submission': submission('correct', source)}})
        assert app.success('alice', {'Submit': {'submission': submission('correct', source)}}) == first
        assert 'error' in app.request('bob', {'Status': {'id': 'correct'}})
        assert 'error' in app.request('alice', {'Status': {'id': 'correct'}}, {'tenant': 'other-school', 'course': 'rust'})
        assert run_runner(runner, config_path).returncode == 0
        correct = app.success('alice', {'Status': {'id': 'correct'}})
        if correct['state'] != 'Completed' or correct.get('result', {}).get('Graded', {}).get('passed') != 2:
            # Only this fixed public addition fixture is diagnosed. Never print
            # arbitrary worker stderr, submitted source or fixture credentials.
            print('trusted-addition-fixture-feedback:', json.dumps({'state': correct['state'], 'result': correct['result']}, ensure_ascii=True)[:16384], flush=True)
        assert correct['state'] == 'Completed' and correct['result']['Graded']['passed'] == 2
        assert 'Experimental' in correct['result']['Graded']['evidence']
        assert not any(secret in json.dumps(correct) for secret in ('private-one', 'expected', 'a+b'))
        checks += ['rust-compilation-and-exact-grading', 'tenant-and-learner-authorization', 'persistent-idempotency']

        cases = [
            ('wrong-answer', 'pub fn solve(a:i64,b:i64)->i64 { a-b }', 'WrongAnswer'),
            ('fuel', 'pub fn solve(_:i64,_:i64)->i64 { loop {} }', {'Trapped': 'Fuel'}),
            # Rust's allocator aborts after memory.grow returns failure; the
            # resulting unreachable instruction carries no authenticated OOM
            # reason. Do not relabel every guest abort as a memory exception.
            ('allocation-abort', 'pub fn solve(_:i64,_:i64)->i64 { let data=std::hint::black_box(vec![1u8;67108864]); data[0] as i64 }', {'Trapped': 'Guest'}),
            ('memory-access', 'pub fn solve(_:i64,_:i64)->i64 { unsafe { std::ptr::read_volatile(std::hint::black_box(8388608usize) as *const u8) as i64 } }', {'Trapped': 'Memory'}),
            ('recursion', '#[inline(never)] fn recurse(n:u64)->u64 { if n==0 {0} else {std::hint::black_box(recurse(n-1)).wrapping_add(n)} } pub fn solve(_:i64,_:i64)->i64 { recurse(std::hint::black_box(10000)) as i64 }', {'Trapped': 'Stack'}),
        ]
        for name, code, feedback in cases:
            app.success('alice', {'Submit': {'submission': submission(name, code)}})
            assert run_runner(runner, config_path).returncode == 0
            view = app.success('alice', {'Status': {'id': name}})
            assert view['state'] == 'Completed' and 'Graded' in view['result'], (name, view['state'])
            assert view['result']['Graded']['passed'] == 0, name
            # These are closed grader feedback enums for fixed public fixtures,
            # never raw diagnostics, learner source, expected values or keys.
            assert view['result']['Graded']['cases'] == [feedback, feedback], (name, view['result']['Graded']['cases'])
            checks.append(name)
            print('passed:', name, flush=True)

        for name, code in [
            ('compiler-errors', 'pub fn solve(_:i64,_:i64)->i64 { missing_symbol }'),
            ('host-files-denied', 'const SECRET: &str=include_str!("/etc/passwd"); pub fn solve(_:i64,_:i64)->i64 { SECRET.len() as i64 }'),
            ('proc-status-denied', 'const SECRET: &[u8]=include_bytes!("/proc/self/status"); pub fn solve(_:i64,_:i64)->i64 { SECRET.len() as i64 }'),
            ('cgroup-files-denied', 'const SECRET: &str=include_str!("/limits/cgroup.procs"); pub fn solve(_:i64,_:i64)->i64 { SECRET.len() as i64 }'),
            ('environment-denied', 'const SECRET: &str=env!("RULLST_LABS_FORBIDDEN_SECRET"); pub fn solve(_:i64,_:i64)->i64 { SECRET.len() as i64 }'),
            ('imports-denied', '#[link(wasm_import_module="host")] unsafe extern "C" { fn forbidden(a:i64)->i64; } pub fn solve(a:i64,_:i64)->i64 { unsafe { forbidden(a) } }'),
            ('compiler-output-bounded', 'compile_error!("' + 'x' * 12000 + '"); pub fn solve(_:i64,_:i64)->i64 { 0 }'),
        ]:
            app.success('alice', {'Submit': {'submission': submission(name, code)}})
            assert run_runner(runner, config_path).returncode == 0
            view = app.success('alice', {'Status': {'id': name}})
            assert view['state'] == 'Failed' and 'Rejected' in view['result'], (name, view['state'])
            diagnostics = view['result']['Rejected']['diagnostics']
            assert diagnostics is None or len(diagnostics.encode('utf-8')) <= 8192, name
            if name == 'compiler-errors':
                text = view['result']['Rejected']['diagnostics']
                assert 'submission.rs' in text and '\x1b' not in text
            if name == 'compiler-output-bounded':
                assert view['result']['Rejected']['failure'] == 'ResourceLimit'
            checks.append(name)
            print('passed:', name, flush=True)

        # Reset guest state between cases and independently prove the actual
        # memory-growth bound, rather than inferring it from allocator aborts.
        for name, code in [
            ('fresh-guest-state', 'static mut COUNT:i64=0; pub fn solve(a:i64,b:i64)->i64 { unsafe { COUNT+=1; a+b+COUNT-1 } }'),
            ('memory-growth-bound', 'pub fn solve(a:i64,b:i64)->i64 { let previous=core::arch::wasm32::memory_grow::<0>(1024); if previous==usize::MAX && core::arch::wasm32::memory_size::<0>()<=64 { a+b } else { 0 } }'),
        ]:
            app.success('alice', {'Submit': {'submission': submission(name, code)}})
            assert run_runner(runner, config_path).returncode == 0
            view = app.success('alice', {'Status': {'id': name}})
            assert view['state'] == 'Completed' and 'Graded' in view['result'], (name, view['state'])
            assert view['result']['Graded']['passed'] == 2, (name, view['result']['Graded']['cases'])
            checks.append(name)
            print('passed:', name, flush=True)

        # No application cancellation or test-owned kill: the controller's own
        # five-second deadline must fence and clean a compiler that stays busy.
        app.success('alice', {'Submit': {'submission': submission('compile-deadline', STRESS, 'stress')}})
        started = time.monotonic()
        child = subprocess.Popen([str(runner), 'run-once', str(config_path)], stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        controllers.append(child)
        observed = False
        while child.poll() is None:
            observed = observed or compiler_running(groups)
            if time.monotonic() - started > 20:
                raise RuntimeError('compiler wall deadline did not stop the owned controller')
            time.sleep(0.05)
        _, stderr = child.communicate(timeout=5)
        assert observed and child.returncode != 0, 'real compilation must reach its deadline'
        assert stderr.strip() == b'lab permission or execution expired', 'expected only the closed deadline error'
        assert time.monotonic() - started >= 5, 'a startup error cannot prove deadline enforcement'
        final = app.success('alice', {'Status': {'id': 'compile-deadline'}})
        assert final['state'] == 'Cancelled' and not final['cleanup_pending'] and final['result'] is None
        assert not any(path.is_dir() for path in groups.iterdir())
        checks.append('compiler-wall-deadline-and-cleanup')
        print('passed: compiler-wall-deadline-and-cleanup', flush=True)

        # A deliberately expensive constant expression is compiled only by the
        # isolated worker. Observe a real compiler in its bounded group, cancel
        # through the application, then require whole-group cleanup.
        app.success('alice', {'Submit': {'submission': submission('cancel', STRESS, 'stress')}})
        child = subprocess.Popen([str(runner), 'run-once', str(config_path)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        controllers.append(child)
        deadline = time.monotonic() + 30
        while True:
            view = app.success('alice', {'Status': {'id': 'cancel'}})
            if view['state'] == 'Running' and compiler_running(groups):
                break
            if child.poll() is not None or time.monotonic() > deadline:
                raise RuntimeError('could not observe the bounded compiler cancellation window')
            time.sleep(0.05)
        cancelled = app.success('alice', {'Cancel': {'id': 'cancel', 'revision': view['revision']}})
        assert cancelled['state'] == 'Cancelled'
        child.wait(timeout=15)
        final = app.success('alice', {'Status': {'id': 'cancel'}})
        assert final['state'] == 'Cancelled' and not final['cleanup_pending'] and final['result'] is None
        checks.append('cancel-running-compiler-and-cleanup')
        assert not any(path.is_dir() for path in groups.iterdir())

        # Kill only this test-owned controller during real compilation, then
        # let the authoritative lease expire before restart reconciliation.
        app.success('alice', {'Submit': {'submission': submission('worker-loss', STRESS, 'stress')}})
        child = subprocess.Popen([str(runner), 'run-once', str(config_path)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        controllers.append(child)
        deadline = time.monotonic() + 30
        while not compiler_running(groups):
            if child.poll() is not None or time.monotonic() > deadline:
                raise RuntimeError('could not observe the bounded worker-loss window')
            time.sleep(0.05)
        child.kill()
        child.wait(timeout=5)
        with sqlite3.connect(directory / 'jobs.sqlite') as database:
            lease_until = database.execute("SELECT lease_until FROM labs_jobs WHERE id='worker-loss'").fetchone()[0]
        delay = max(0, lease_until - time.time() + 1)
        assert delay <= 26
        time.sleep(delay)
        # Exhaust the actual kernel group capacity with empty test-owned
        # placeholders. Recovery of the authenticated abandoned job must free
        # its existing group BEFORE a new preflight needs another slot.
        placeholders = []
        existing = sum(path.is_dir() for path in groups.iterdir())
        assert existing == 1
        for index in range(32 - existing):
            placeholder = groups / f'capacity-fixture-{index}'
            placeholder.mkdir()
            placeholders.append(placeholder)
        overflow = groups / 'capacity-overflow'
        try:
            overflow.mkdir()
        except OSError:
            pass  # The exact kernel errno is not the capacity contract.
        else:
            overflow.rmdir()
            raise RuntimeError('kernel descendant capacity was not enforced')
        assert run_runner(runner, config_path).returncode == 0
        lost = app.success('alice', {'Status': {'id': 'worker-loss'}})
        assert lost['state'] == 'Cancelled' and not lost['cleanup_pending'] and lost['result'] is None
        for placeholder in placeholders:
            placeholder.rmdir()
        assert not any(path.is_dir() for path in groups.iterdir())
        checks.append('controller-loss-restart-fencing-and-cleanup')
        checks.append('full-cgroup-capacity-recovery')

        # Two independent controllers compete for one real durable job.
        app.success('alice', {'Submit': {'submission': submission('concurrent', source)}})
        children = [subprocess.Popen([str(runner), 'run-once', str(config_path)], stdout=subprocess.PIPE, stderr=subprocess.PIPE) for _ in range(2)]
        controllers.extend(children)
        results = [child.communicate(timeout=90) for child in children]
        assert all(child.returncode == 0 for child in children)
        assert sum(b'job-finished' in output for output, _ in results) == 1
        assert app.success('alice', {'Status': {'id': 'concurrent'}})['result']['Graded']['passed'] == 2
        checks.append('independent-controller-single-lease')
        assert not any(path.is_dir() for path in groups.iterdir())

        # Confirm source is no longer retained for completed/cancelled jobs.
        with sqlite3.connect(directory / 'jobs.sqlite') as database:
            assert database.execute('SELECT COUNT(*) FROM labs_jobs WHERE content IS NOT NULL').fetchone()[0] == 0
        checks.append('terminal-source-removal')
        args.evidence.write_text(json.dumps({'status': 'passed', 'profile': config['linux']['profile'], 'preflight': doctor, 'checks': checks, 'controller_measurement_only': args.controller is not None, 'live_providers_used': False, 'independent_isolation_review': 'outstanding'}, indent=2) + '\n')
    finally:
        # A failed assertion must not leave a controller touching the fixture
        # while the outer cleanup tears down its groups and deletes its files.
        for child in controllers:
            if child.poll() is None:
                child.kill()
        for child in controllers:
            child.wait(timeout=5)
        app.close()
    print('isolated Labs checks passed:', len(checks), flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for field in ('runner', 'app', 'toolchain', 'launcher', 'evidence'):
        parser.add_argument('--' + field, required=True, type=Path)
    parser.add_argument('--controller', type=Path,
                        help='trusted instrumented controller; the worker and its environment remain unchanged')
    args = parser.parse_args()
    groups = delegated_jobs()
    parent = Path(tempfile.mkdtemp(prefix='rullst-labs-acceptance-'))
    try:
        accept(args, parent / 'fixture', groups)
    finally:
        # These are only groups created under this test-owned empty subtree.
        for path in groups.iterdir():
            if path.is_dir() and path.name.startswith('rullst-labs-'):
                (path / 'cgroup.kill').write_text('1')
        deadline = time.monotonic() + 5
        while any(path.is_dir() for path in groups.iterdir()) and time.monotonic() < deadline:
            for path in groups.iterdir():
                if path.is_dir() and path.name.startswith(('rullst-labs-', 'capacity-fixture-')) and 'populated 0' in (path / 'cgroup.events').read_text():
                    path.rmdir()
            time.sleep(0.05)
        if any(path.is_dir() for path in groups.iterdir()):
            raise RuntimeError('owned groups remain; service teardown must complete before fixture cleanup')
        shutil.rmtree(parent)


if __name__ == '__main__':
    main()
