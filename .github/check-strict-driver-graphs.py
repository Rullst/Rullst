#!/usr/bin/env python3
"""Inspect standalone consumers: workspace dev-dependencies mask driver leaks."""

import json
import pathlib
import shutil
import subprocess
import tempfile

root = pathlib.Path(__file__).resolve().parent.parent
drivers = {"sqlx-postgres", "sqlx-mysql", "sqlx-sqlite"}
with tempfile.TemporaryDirectory(prefix="rullst-strict-drivers-") as directory:
    for package in ("rullst-orm", "rullst-core", "rullst-studio", "rullst-nexus", "rullst"):
        for driver in ("postgres", "mysql", "sqlite"):
            path = pathlib.Path(directory) / f"{package}-{driver}"
            (path / "src").mkdir(parents=True)
            features = [f"strict-{driver}"]
            if package == "rullst":
                features += ["studio", "nexus"]
            (path / "src/lib.rs").write_text("// Independent consumer graph.\n")
            (path / "Cargo.toml").write_text(
                '[package]\nname = "strict-driver-consumer"\nversion = "0.0.0"\nedition = "2024"\n'
                f'[workspace]\n[dependencies]\n{package} = {{ path = {json.dumps(str(root / package))}, '
                f'default-features = false, features = {json.dumps(features)} }}\n'
            )
            shutil.copyfile(root / "Cargo.lock", path / "Cargo.lock")
            result = subprocess.run(
                ["cargo", "tree", "--offline", "--manifest-path", str(path / "Cargo.toml"),
                 "--edges", "normal,build", "--prefix", "none", "--format", "{p}"],
                capture_output=True, text=True, check=True,
            )
            selected = {line.split()[0] for line in result.stdout.splitlines()} & drivers
            expected = {f"sqlx-{driver}"}
            if selected != expected:
                raise SystemExit(f"{package} strict-{driver}: expected {expected}, found {selected}")
            print(f"{package} strict-{driver}: {', '.join(sorted(selected))}")
