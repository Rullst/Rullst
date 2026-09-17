#!/usr/bin/env python3
"""Stage native CLI files and a bounded identity/digest inventory; never install."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[1]
MAX_BINARY = 128 * 1024 * 1024
VERSION = re.compile(r"[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z]+(?:[.-][0-9A-Za-z]+)*)?")
SHA = re.compile(r"[0-9a-f]{40}")
REPOSITORY = re.compile(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+")
BINARIES = ("cargo-rullst", "rullst")


def targets() -> dict[str, str]:
    entries = json.loads((ROOT / ".github/cli-artifact-targets.json").read_text())
    return {entry["target"]: entry["runner"] for entry in entries}


def names(version: str, target: str) -> list[str]:
    if not VERSION.fullmatch(version) or target not in targets():
        raise ValueError("unsupported artifact version or target")
    suffix = ".exe" if target.endswith("windows-msvc") else ""
    return [f"{binary}-{version}-{target}{suffix}" for binary in BINARIES]


def regular(path: Path, maximum: int) -> bytes:
    info = path.lstat()
    if not stat.S_ISREG(info.st_mode) or not 0 < info.st_size <= maximum:
        raise ValueError("artifact must be a bounded nonempty regular file")
    with path.open("rb") as stream:
        body = stream.read(maximum + 1)
    if len(body) != info.st_size:
        raise ValueError("artifact changed or exceeded its size bound")
    return body


def verify(directory: Path, version: str, target: str, commit: str, repository: str,
           require_release: bool = False) -> dict:
    expected = names(version, target)
    if not SHA.fullmatch(commit) or not REPOSITORY.fullmatch(repository):
        raise ValueError("invalid source identity")
    manifest_name = f"cli-manifest-{target}.json"
    checksum_name = f"cli-checksums-{target}.txt"
    manifest = json.loads(regular(directory / manifest_name, 16 * 1024))
    if (manifest.get("schema") != "rullst.cli-artifacts.v1"
            or manifest.get("version") != version or manifest.get("target") != target
            or manifest.get("source_commit") != commit or manifest.get("repository") != repository
            or manifest.get("build_runner") != targets()[target]
            or manifest.get("release_tag") not in (None, f"v{version}")
            or (require_release and manifest.get("release_tag") != f"v{version}")):
        raise ValueError("artifact identity does not match the expected candidate")
    if {entry.name for entry in directory.iterdir()} != set(expected + [manifest_name, checksum_name]):
        raise ValueError("unexpected or missing artifact files")
    records = manifest.get("files")
    if not isinstance(records, list) or len(records) != len(expected):
        raise ValueError("incomplete executable inventory")
    hashes = {}
    for binary, filename, record in zip(BINARIES, expected, records):
        if not isinstance(record, dict) or record.get("name") != filename or record.get("executable") != binary:
            raise ValueError("executable inventory identity mismatch")
        body = regular(directory / filename, MAX_BINARY)
        digest = hashlib.sha256(body).hexdigest()
        if type(record.get("bytes")) is not int or record["bytes"] != len(body) or record.get("sha256") != digest:
            raise ValueError("executable digest or size mismatch")
        hashes[filename] = digest
    hashes[manifest_name] = hashlib.sha256(regular(directory / manifest_name, 16 * 1024)).hexdigest()
    checksums = "".join(f"{digest}  {name}\n" for name, digest in sorted(hashes.items()))
    if regular(directory / checksum_name, 4 * 1024).decode("ascii") != checksums:
        raise ValueError("checksum inventory mismatch")
    return manifest


def stage(binary_dir: Path, output: Path, version: str, target: str, commit: str,
          repository: str, release_tag: str | None) -> None:
    filenames = names(version, target)
    if not SHA.fullmatch(commit) or not REPOSITORY.fullmatch(repository):
        raise ValueError("invalid source identity")
    if release_tag not in (None, f"v{version}"):
        raise ValueError("release tag does not match package version")
    suffix = ".exe" if target.endswith("windows-msvc") else ""
    # Only trusted build outputs are executed here; the attestation job must
    # never call this script or run downloaded candidate executables.
    sources = [binary_dir / (binary + suffix) for binary in BINARIES]
    for source in sources:
        regular(source, MAX_BINARY)
        result = subprocess.run([str(source.resolve()), "--version"], check=True,
                                capture_output=True, text=True, timeout=15,
                                env={**os.environ, "RULLST_DISABLE_UPDATE_CHECK": "true"})
        if result.stdout.strip().split() not in (["rullst", version], ["cargo-rullst", version]):
            raise ValueError("native executable reports an unexpected version")
    output.mkdir(parents=True, exist_ok=False)
    records = []
    for binary, filename, source in zip(BINARIES, filenames, sources):
        destination = output / filename
        with source.open("rb") as reader, destination.open("xb") as writer:
            shutil.copyfileobj(reader, writer, length=1024 * 1024)
        destination.chmod(0o755)
        body = regular(destination, MAX_BINARY)
        records.append({"name": filename, "executable": binary, "bytes": len(body),
                        "sha256": hashlib.sha256(body).hexdigest()})
    manifest = {"schema": "rullst.cli-artifacts.v1", "version": version, "target": target,
                "source_commit": commit, "repository": repository, "release_tag": release_tag,
                "build_runner": targets()[target], "files": records}
    manifest_name = f"cli-manifest-{target}.json"
    (output / manifest_name).write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8", newline="\n")
    hashes = {record["name"]: record["sha256"] for record in records}
    hashes[manifest_name] = hashlib.sha256((output / manifest_name).read_bytes()).hexdigest()
    (output / f"cli-checksums-{target}.txt").write_text(
        "".join(f"{digest}  {name}\n" for name, digest in sorted(hashes.items())), encoding="ascii", newline="\n")
    verify(output, version, target, commit, repository, require_release=release_tag is not None)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True, choices=targets())
    parser.add_argument("--binary-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    version = tomllib.loads((ROOT / "cargo-rullst/Cargo.toml").read_text())["package"]["version"]
    commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    if commit != os.environ.get("GITHUB_SHA"):
        raise ValueError("checkout does not match the workflow SHA")
    host = subprocess.check_output(["rustc", "-vV"], text=True)
    if f"host: {args.target}" not in host.splitlines():
        raise ValueError("native smoke requires the declared Rust host target")
    release_tag = None
    if os.environ.get("GITHUB_REF", "").startswith("refs/tags/"):
        if os.environ.get("GITHUB_EVENT_NAME") != "push":
            raise ValueError("release artifacts require the tag-only publication workflow")
        release_tag = os.environ["GITHUB_REF"].removeprefix("refs/tags/")
    stage(args.binary_dir, args.output, version, args.target, commit,
          os.environ["GITHUB_REPOSITORY"], release_tag)


if __name__ == "__main__":
    main()
