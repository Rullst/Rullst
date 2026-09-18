#!/usr/bin/env python3
"""Fail-closed contracts for the two observed apksigner report formats."""

import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location(
    "android_signing", Path(__file__).with_name("check-android-signing.py")
)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class SigningReportTests(unittest.TestCase):
    digest = "ab" * 32

    def report(self, label="Signer #1", digest=None):
        return (
            "Verifies\nNumber of signers: 1\n"
            f"{label} certificate SHA-256 digest: {digest or self.digest}\n"
            f"{label} public key SHA-256 digest: {'cd' * 32}\n"
        )

    def test_old_and_scheme_labelled_signers_match_exact_certificate(self):
        for label in ["Signer #1", "V2 Signer:"]:
            module.verify_report(self.report(label), self.digest)
            module.verify_report(self.report(label, self.digest.upper()), self.digest)

    def test_wrong_or_missing_certificate_cannot_use_public_key_digest(self):
        for report in [
            self.report(digest="00" * 32),
            "Verifies\nNumber of signers: 1\n"
            f"Signer #1 public key SHA-256 digest: {self.digest}\n",
            self.report("Source Stamp Signer"),
            self.report("Unknown Signer"),
            self.report().replace(self.digest, self.digest + "00"),
            self.report() + f"Signer #2 certificate SHA-256 digest: {self.digest}\n",
        ]:
            with self.subTest(report=report), self.assertRaises(ValueError):
                module.verify_report(report, self.digest)

    def test_signer_inventory_must_be_exactly_one(self):
        for report in [
            self.report().replace("Number of signers: 1", "Number of signers: 2"),
            self.report().replace("Number of signers: 1\n", ""),
            self.report() + "Number of signers: 1\n",
        ]:
            with self.assertRaises(ValueError):
                module.verify_report(report, self.digest)


if __name__ == "__main__":
    unittest.main()
