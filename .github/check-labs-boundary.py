#!/usr/bin/env python3
"""Validate the unpublished Labs bring-your-own-runner boundary, not release admission.

`rullst-labs` holds trusted contracts and the job plane only. Execution belongs
to an application-owned, separately deployed runner outside this workspace.
"""
import json
from pathlib import Path
import sys

metadata = json.loads(Path(sys.argv[1]).read_text())
packages = {package['name']: package for package in metadata['packages']}
labs = packages['rullst-labs']
assert labs['publish'] == [], 'release admission needs explicit policy review'
assert labs['features']['default'] == [], 'Labs must remain opt-in'
allowed = {'thiserror', 'serde', 'serde_json', 'sha2', 'hex', 'zeroize', 'sqlx', 'ring', 'tokio'}
for dependency in labs['dependencies']:
    if dependency['kind'] != 'dev':
        assert dependency['name'] in allowed, 'application-side Labs cannot gain an execution engine'
        if dependency['name'] in {'sqlx', 'ring', 'tokio'}:
            assert dependency['optional'], 'default Labs contracts must not require IO or crypto backends'
for target in labs['targets']:
    if 'bin' in target['kind']:
        raise AssertionError('Labs must not ship an executor binary')
# The runner candidate was removed from 13.0. Neither it nor an isolation or
# execution engine may return to the workspace through another member.
removed = 'rullst-labs-runner'
engines = {removed, 'wasmi', 'wasmtime', 'wasmer', 'landlock', 'seccompiler', 'bollard'}
workspace = set(metadata['workspace_members'])
for package in packages.values():
    assert package['name'] != removed, 'the runner candidate is not a workspace package in 13.0'
    if package['id'] in workspace:
        names = {dep['name'] for dep in package['dependencies']}
        assert not names & engines, f"{package['name']} must not depend on an execution engine"
print('Labs contracts verified without an executor dependency or runner crate.')
