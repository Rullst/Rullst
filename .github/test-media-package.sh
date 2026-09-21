#!/usr/bin/env bash
# Exercise only extracted bytes, with no path dependency on workspace source.
set -euo pipefail
version="${1:?usage: test-media-package.sh VERSION PACKAGE_DIR}"
package_dir="$(cd "${2:?missing archive directory}" && pwd -P)"
repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
work_dir="$(mktemp -d "${TMPDIR:-/tmp}/rullst-media-package.XXXXXX")"
trap 'rm -rf -- "$work_dir"' EXIT
python3 - "$package_dir" "$version" "$work_dir" "$repository_root" <<'PY'
import json, sys, tarfile, tomllib
from pathlib import Path
archives, version, temporary, repository = map(Path, sys.argv[1:])
version = str(version)
assert 'rullst-media' not in json.loads((repository / '.github/release-order.json').read_text())
name = f'rullst-media-{version}'
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
manifest = tomllib.loads((source / 'Cargo.toml').read_text())
assert manifest['package']['name'] == 'rullst-media'
assert manifest['package']['version'] == version and manifest['package']['publish'] is False
assert (source / 'LICENSE').read_bytes() == (repository / 'LICENSE').read_bytes()
assert 'web/bunny-upload.mjs' in (source / 'src/lib.rs').read_text()
consumer = temporary / 'consumer'
(consumer / 'src').mkdir(parents=True)
(consumer / 'Cargo.toml').write_text('[package]\nname="media-archive-consumer"\nversion="0.0.0"\nedition="2024"\npublish=false\n[dependencies]\nrullst-media={path=' + json.dumps(str(source)) + ',default-features=false,features=["bunny","sqlite"]}\ntokio={version="1",features=["macros","rt-multi-thread"]}\ntempfile="3"\n')
(consumer / 'src/main.rs').write_bytes((source / 'examples/private_course_video.rs').read_bytes())
# Start from the packaged resolution, not unrelated newer cached registry entries.
# Cargo prunes unused development dependencies for this independent consumer.
(consumer / 'Cargo.lock').write_bytes((source / 'Cargo.lock').read_bytes())
PY
"${CARGO:-cargo}" run --manifest-path "$work_dir/consumer/Cargo.toml" --offline
RULLST_MEDIA_PACKAGE_MODULE="$work_dir/rullst-media-$version/web/bunny-upload.mjs" \
  node "$repository_root/.github/media-upload-tests.mjs"
printf 'Verified extracted rullst-media %s lifecycle and browser module.\n' "$version"
