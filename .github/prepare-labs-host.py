#!/usr/bin/env python3
"""Render a reviewed distro AppArmor profile for a private hosted-test launcher.

No policy is loaded by this helper. The CI job alone provisions its disposable
Ubuntu VM. Input comes from the authenticated Ubuntu apparmor-profiles package;
a changed upstream profile requires review rather than automatic acceptance.
Reference: https://documentation.ubuntu.com/security/security-features/privilege-restriction/apparmor/
"""
import argparse
import hashlib
from pathlib import Path

REVIEWED_SHA256 = '11d39094f044f0cda0febb3ad517b830301da6b2ce929664af09ee9e4dd264f9'


def render(source, destination):
    original = source.read_bytes()
    if hashlib.sha256(original).hexdigest() != REVIEWED_SHA256:
        raise ValueError('distro namespace profile changed; review required')
    text = original.decode('utf-8')
    # Keep all upstream rules, including the child capability denial. Give both
    # profiles unique names and attach only to our root-owned launcher copy.
    text = text.replace('unpriv_bwrap', 'rullst_labs_ci_child')
    text = text.replace('profile bwrap /usr/bin/bwrap', 'profile rullst_labs_ci /usr/lib/rullst-labs-ci/bwrap')
    text = text.replace('-> bwrap//&', '-> rullst_labs_ci//&')
    # Do not import any unrelated host-local policy into the reviewed fixture.
    text = '\n'.join(line for line in text.splitlines() if 'include if exists <local/' not in line) + '\n'
    with destination.open('x', encoding='utf-8') as output:
        output.write(text)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source', type=Path)
    parser.add_argument('destination', type=Path)
    args = parser.parse_args()
    render(args.source, args.destination)
