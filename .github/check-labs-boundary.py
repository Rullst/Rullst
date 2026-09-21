#!/usr/bin/env python3
"""Validate unpublished Labs process/dependency boundaries, not release admission."""
import json
from pathlib import Path
import sys

metadata = json.loads(Path(sys.argv[1]).read_text())
packages = {package['name']: package for package in metadata['packages']}
labs, runner = (packages[name] for name in ('rullst-labs', 'rullst-labs-runner'))
assert labs['publish'] == [] and runner['publish'] == [], 'release admission needs explicit policy review'
assert labs['features']['default'] == [], 'Labs must remain opt-in'
allowed = {'thiserror', 'serde', 'serde_json', 'sha2', 'hex', 'zeroize', 'sqlx', 'ring', 'tokio'}
for dependency in labs['dependencies']:
    if dependency['kind'] != 'dev':
        assert dependency['name'] in allowed, 'application-side Labs cannot gain an execution engine'
        if dependency['name'] in {'sqlx', 'ring', 'tokio'}:
            assert dependency['optional'], 'default Labs contracts must not require IO or crypto backends'
for package in packages.values():
    assert not any(dep['name'] == 'rullst-labs-runner' for dep in package['dependencies']), 'runner must never become a framework/application dependency'
assert not any('lib' in target['kind'] for target in runner['targets']), 'executor must remain a separately deployed binary'
print('Labs contracts and separately deployed executor boundaries verified.')
