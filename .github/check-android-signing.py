#!/usr/bin/env python3
"""Match apksigner's already-verified signer report to the disposable CI cert."""

import argparse
import hashlib
from pathlib import Path
import re


# Android Build Tools now identifies v2 signers by scheme instead of ordinal.
# Unknown formats fail until reviewed; a public-key or source-stamp digest is
# never interchangeable with the APK signing certificate digest.
CERTIFICATE = re.compile(
    r"(?:Signer #1|V2 Signer:) certificate SHA-256 digest: ([0-9a-fA-F]{64})"
)


def verify_report(report: str, expected_digest: str) -> None:
    if re.fullmatch(r"[0-9a-f]{64}", expected_digest) is None:
        raise ValueError("invalid expected certificate digest")
    lines = report.splitlines()
    signers = [line for line in lines if line.startswith("Number of signers:")]
    if signers != ["Number of signers: 1"]:
        raise ValueError("expected exactly one APK signer")
    certificates = [line for line in lines if "certificate SHA-256 digest:" in line]
    if not certificates:
        raise ValueError("missing APK signing certificate digest")
    for line in certificates:
        match = CERTIFICATE.fullmatch(line)
        if match is None or match.group(1).lower() != expected_digest:
            raise ValueError("unexpected APK certificate or signer report format")


def read_bounded(path: Path) -> bytes:
    with path.open("rb") as stream:
        data = stream.read(65537)
    if not data or len(data) > 65536:
        raise ValueError("invalid certificate/report size")
    return data


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("certificate", type=Path)
    parser.add_argument("report", type=Path)
    args = parser.parse_args()
    try:
        expected = hashlib.sha256(read_bounded(args.certificate)).hexdigest()
        verify_report(read_bounded(args.report).decode("utf-8"), expected)
    except (OSError, UnicodeError, ValueError) as error:
        raise SystemExit(f"Android signing evidence rejected: {error}") from error
    print(f"APK certificate matches the disposable key: {expected}")


if __name__ == "__main__":
    main()
