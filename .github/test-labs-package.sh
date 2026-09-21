#!/usr/bin/env bash
# Exercise extracted contracts and a separate application consumer. Student code
# is never compiled here; real execution belongs to the mandatory isolation job.
set -euo pipefail
version="${1:?usage: test-labs-package.sh VERSION PACKAGE_DIR}"
package_dir="$(cd "${2:?missing archive directory}" && pwd -P)"
repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
work_dir="$(mktemp -d "${TMPDIR:-/tmp}/rullst-labs-package.XXXXXX")"
trap 'rm -rf -- "$work_dir"' EXIT
python3 - "$package_dir" "$version" "$work_dir" "$repository_root" <<'PY'
import json, os, sys, tarfile, tomllib
from pathlib import Path
archives, version, temporary, repository = sys.argv[1:]
archives, temporary, repository = map(Path, (archives, temporary, repository))
inventory = json.loads((repository / '.github/release-order.json').read_text())
for package in ('rullst-labs', 'rullst-labs-runner'):
    assert package not in inventory
    name = f'{package}-{version}'
    archive_path = archives / f'{name}.crate'
    assert 0 < archive_path.stat().st_size <= 10 * 1024 * 1024
    with tarfile.open(archive_path, 'r:gz') as archive:
        members = archive.getmembers()
        assert len(members) <= 2000
        assert sum(m.size for m in members) <= 50 * 1024 * 1024
        for member in members:
            parts = Path(member.name).parts
            assert parts and parts[0] == name and '..' not in parts
            assert member.isfile() or member.isdir()
        archive.extractall(temporary, filter='data')
    source = temporary / name
    # Cargo archives normalize mtimes. Refresh only metadata so a reused local
    # target cannot mistake changed extracted source for an older cached build.
    for path in source.rglob('*'):
        if path.is_file():
            os.utime(path, None)
    manifest = tomllib.loads((source / 'Cargo.toml').read_text())
    assert manifest['package']['name'] == package
    assert manifest['package']['version'] == version and manifest['package']['publish'] is False
    assert (source / 'LICENSE').read_bytes() == (repository / 'LICENSE').read_bytes()
labs = temporary / f'rullst-labs-{version}'
runner = temporary / f'rullst-labs-runner-{version}'
with (runner / 'Cargo.toml').open('a') as manifest:
    manifest.write('\n[patch.crates-io]\nrullst-labs={path=' + json.dumps(str(labs)) + '}\n')
consumer = temporary / 'consumer'
(consumer / 'src').mkdir(parents=True)
(consumer / 'Cargo.toml').write_text('[package]\nname="labs-archive-consumer"\nversion="0.0.0"\nedition="2024"\npublish=false\n[dependencies]\nrullst-labs={path=' + json.dumps(str(labs)) + ',default-features=false,features=["sqlite"]}\ntokio={version="1",features=["macros","rt"]}\nserde={version="1",features=["derive"]}\nserde_json="1"\nzeroize="1"\n')
(consumer / 'src/main.rs').write_bytes((labs / 'examples/course_app.rs').read_bytes())
(consumer / 'Cargo.lock').write_bytes((labs / 'Cargo.lock').read_bytes())
PY
cargo_bin="${CARGO:-cargo}"
"$cargo_bin" test --manifest-path "$work_dir/rullst-labs-$version/Cargo.toml" --offline --locked --all-features
# The candidate's explicit unpublished edge is resolved from this archive copy;
# all registry dependencies must retain the packaged resolution.
"$cargo_bin" test --manifest-path "$work_dir/rullst-labs-runner-$version/Cargo.toml" --offline --locked
"$cargo_bin" build --manifest-path "$work_dir/consumer/Cargo.toml" --offline
"$cargo_bin" metadata --manifest-path "$work_dir/consumer/Cargo.toml" --locked --offline --format-version 1 > "$work_dir/consumer-metadata.json"
python3 - "$work_dir" <<'PY'
import json, os, subprocess, sys
from pathlib import Path
work = Path(sys.argv[1])
metadata = json.loads((work / 'consumer-metadata.json').read_text())
for package in metadata['packages']:
    if package['source'] is None:
        assert Path(package['manifest_path']).is_relative_to(work)
    assert package['name'] not in ('rullst-labs-runner', 'wasmi', 'seccompiler', 'landlock')
executable = Path(metadata['target_directory']) / 'debug/labs-archive-consumer'
key = work / 'content-key'
descriptor = os.open(key, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
with os.fdopen(descriptor, 'wb') as output:
    output.write(os.urandom(32))
config = work / 'application.json'
config.write_text(json.dumps({'database': str(work / 'jobs.sqlite'), 'namespace': 'archive', 'max_jobs': 8, 'max_exercises': 8, 'content_key': str(key), 'profile': 'Simulation'}))
subprocess.run([str(executable), 'initialize', str(config)], check=True, timeout=15)
scope = {'tenant': 'school', 'course': 'rust'}
exercise = {'scope': scope, 'id': 'sum', 'revision': 'v1', 'cases': [{'id': 'hidden', 'input': [3, 5], 'expected': 8}], 'limits': {'wall_seconds': 10, 'fuel_per_case': 10000, 'memory_pages': 64}}
submission = {'id': 'one', 'exercise': {'id': 'sum', 'revision': 'v1'}, 'source': 'pub fn solve(a:i64,b:i64)->i64 {a+b}', 'ttl_seconds': 300}
operations = [('teacher', {'Register': {'exercise': exercise}}), ('alice', {'Submit': {'submission': submission}}), ('bob', {'Status': {'id': 'one'}}), ('alice', {'Cancel': {'id': 'one', 'revision': 1}})]
requests = ''.join(json.dumps({'actor': actor, 'scope': scope, 'operation': operation}) + '\n' for actor, operation in operations)
result = subprocess.run([str(executable), 'serve', str(config)], input=requests, text=True, capture_output=True, check=True, timeout=15)
responses = [json.loads(line) for line in result.stdout.splitlines()]
assert len(responses) == 4
assert responses[0]['ok']['registered'] is True
assert responses[1]['ok']['state'] == 'Queued' and 'error' in responses[2]
assert responses[3]['ok']['state'] == 'Cancelled'
assert 'expected' not in result.stdout and submission['source'] not in result.stdout
print('Archive-only application submit, ownership and cancellation passed; no execution dependency.')
PY
printf 'Verified extracted Labs contracts, runner refusal and application consumer for %s.\n' "$version"
