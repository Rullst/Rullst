#!/usr/bin/env python3
"""Check where the facade builds a TLS stack and that Core installs its provider there.

aws-lc-rs is the rustls provider of every TLS connection Rullst makes, and
`Server` start-up installs it as the rustls process default when Core's `tls`
feature is on (docs/src/security-architecture.md#cryptography-providers).
For every public feature of the `rullst` facade, alone and without default
features, `cargo tree` (normal and build dependencies of the host platform,
the packages Cargo compiles) must show:

- Core's `tls` feature whenever rustls is compiled, so no TLS connection
  misses the provider;
- rustls without its `ring` feature (Rullst never makes ring a rustls
  provider).

The facade features of SQLite-only starters (the Blank and API starters with
SQLite, the Blank starter without a database; see
cargo-rullst/src/generators/project/cargo_toml.rs) must compile no TLS stack
at all.

Usage: check-tls-provider.py [--offline]
"""

from __future__ import annotations

import argparse
from pathlib import Path
import subprocess
import sys
import tomllib

ROOT = Path(__file__).resolve().parent.parent
# The facade features `cargo rullst new` selects for starters that make no TLS
# connection.
SQLITE_ONLY = {
    "Blank or API starter with SQLite": ("orm", "strict-sqlite", "studio"),
    "Blank starter without a database": ("studio",),
}
# Packages of a TLS stack; none may be compiled for a SQLite-only starter.
TLS_PACKAGES = {"rustls", "aws-lc-rs", "aws-lc-sys", "tokio-rustls", "hyper-rustls", "webpki-roots",
                "rustls-webpki", "rustls-platform-verifier", "rustls-native-certs", "native-tls", "openssl"}


def compiled(features: tuple[str, ...], offline: bool) -> dict[str, set[str]]:
    """Each package `cargo tree` compiles for these facade features, with its enabled features."""
    command = ["cargo", "tree", "--locked", "--package", "rullst", "--no-default-features",
               "--edges", "normal,build", "--prefix", "none", "--format", "{p}|{f}"]
    if features:
        command += ["--features", ",".join(features)]
    if offline:
        command.append("--offline")
    output = subprocess.run(command, cwd=ROOT, check=True, text=True, capture_output=True).stdout
    packages: dict[str, set[str]] = {}
    for line in output.splitlines():
        if "|" not in line:
            continue
        package, enabled = line.split("|", 1)
        name = package.split()[0]
        packages.setdefault(name, set()).update(
            feature for feature in enabled.removesuffix(" (*)").split(",") if feature)
    return packages


def problems_of(feature: str, packages: dict[str, set[str]]) -> list[str]:
    problems = []
    if "rustls" in packages and "tls" not in packages.get("rullst-core", set()):
        problems.append(f"`{feature}` compiles rustls without Core's `tls` feature, so `Server` "
                        "start-up would not install aws-lc-rs as the rustls process default")
    if "ring" in packages.get("rustls", set()):
        problems.append(f"`{feature}` enables rustls' `ring` provider")
    return problems


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("--offline", action="store_true")
    args = parser.parse_args()
    manifest = tomllib.loads((ROOT / "rullst" / "Cargo.toml").read_text(encoding="utf-8"))
    features = sorted(set(manifest["features"]) - {"default"})
    problems: list[str] = []
    with_tls = 0
    for feature in features:
        packages = compiled((feature,), args.offline)
        with_tls += "rustls" in packages
        problems += problems_of(feature, packages)
    for starter, selected in SQLITE_ONLY.items():
        stack = sorted(TLS_PACKAGES & compiled(selected, args.offline).keys())
        if stack:
            problems.append(f"{starter} ({', '.join(selected)}) compiles a TLS stack: {', '.join(stack)}")
    for problem in problems:
        print(f"::error title=TLS provider::{problem}", file=sys.stderr)
    if not problems:
        print(f"PASS: {with_tls} of {len(features)} facade features compile rustls, each with Core's `tls`; "
              f"{len(SQLITE_ONLY)} SQLite-only starter profiles compile no TLS stack")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
