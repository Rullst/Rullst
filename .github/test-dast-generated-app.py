#!/usr/bin/env python3
"""Tests for .github/dast-generated-app.py without a build or a network."""

from __future__ import annotations

import contextlib
import http.server
import importlib.util
import io
import os
from pathlib import Path
import tempfile
import threading
import tomllib
import unittest
from unittest import mock
import urllib.parse


SCRIPT = Path(__file__).with_name("dast-generated-app.py")
SPEC = importlib.util.spec_from_file_location("dast_generated_app", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot import {SCRIPT}")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

CRATES = {"rullst", "rullst-orm", "rullst-auth", "rullst-capital", "rullst-security", "rullst-core"}
CHECKOUT = Path("/checkout")
PRERELEASE_MANIFEST = """[package]
name = "zap-saas"
version = "0.1.0"
edition = "2024"

[dependencies]
rullst = { path = "/checkout/rullst", default-features = false, features = ["orm", "auth"] }
serde = { version = "1.0", features = ["derive"] }
rullst-orm = { path = '/checkout/rullst-orm', default-features = false, features = ["strict-sqlite"] }
rullst-auth = { path = "/checkout/rullst-auth" }
rullst-security = { path = "/checkout/rullst-security" }
local-helper = { path = "crates/helper" }

[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
"""


class RegistryPatchTests(unittest.TestCase):
    def test_prerelease_path_dependencies_become_exact_registry_requirements(self):
        manifest = tomllib.loads(MODULE.registry_patch(PRERELEASE_MANIFEST, CRATES, "13.0.0-alpha.4", CHECKOUT))
        dependencies = manifest["dependencies"]
        self.assertEqual(dependencies["rullst"], {"version": "=13.0.0-alpha.4", "default-features": False,
                                                  "features": ["orm", "auth"]})
        self.assertEqual(dependencies["rullst-orm"]["version"], "=13.0.0-alpha.4")
        self.assertEqual(dependencies["rullst-auth"], {"version": "=13.0.0-alpha.4"})
        self.assertEqual(dependencies["local-helper"], {"path": "crates/helper"})
        self.assertEqual(manifest["patch"]["crates-io"], {
            name: {"path": f"/checkout/{name}"}
            for name in ("rullst", "rullst-auth", "rullst-orm", "rullst-security")})

    def test_stable_registry_dependencies_are_patched_unchanged(self):
        stable = PRERELEASE_MANIFEST
        for name in ("rullst", "rullst-auth", "rullst-security"):
            stable = stable.replace(f'{name} = {{ path = "/checkout/{name}"', f'{name} = {{ version = "13.0.0"')
        stable = stable.replace("rullst-orm = { path = '/checkout/rullst-orm'", 'rullst-orm = { version = "13.0.0"')
        manifest = tomllib.loads(MODULE.registry_patch(stable, CRATES, "13.0.0", CHECKOUT))
        self.assertEqual(manifest["dependencies"]["rullst"]["version"], "13.0.0")
        self.assertEqual(sorted(manifest["patch"]["crates-io"]),
                         ["rullst", "rullst-auth", "rullst-orm", "rullst-security"])

    def test_rejects_manifests_it_cannot_patch_safely(self):
        for manifest, message in (
            (PRERELEASE_MANIFEST.replace("rullst = {", "other = {"), "does not depend on rullst"),
            (PRERELEASE_MANIFEST + "\n[patch.crates-io]\n", "already has"),
            (PRERELEASE_MANIFEST.replace('rullst-auth = { path = "/checkout/rullst-auth" }',
                                         'rullst-auth = "13.0.0-alpha.4"'), "rullst-auth is not a registry"),
        ):
            with self.subTest(message=message), self.assertRaisesRegex(MODULE.Failure, message):
                MODULE.registry_patch(manifest, CRATES, "13.0.0-alpha.4", CHECKOUT)


class Starter(http.server.BaseHTTPRequestHandler):
    """The registration contract of the SaaS starter, with production cookies."""

    csrf = "csrf-token-value"
    session = "encrypted-session-value"

    def log_message(self, *_args):
        pass

    def cookies(self) -> dict[str, str]:
        pairs = (item.strip().partition("=") for item in self.headers.get("Cookie", "").split(";") if item.strip())
        return {name: value for name, _, value in pairs}

    def answer(self, status: int, body: str = "", headers: tuple[tuple[str, str], ...] = ()):
        self.send_response(status)
        for name, value in headers:
            self.send_header(name, value)
        self.send_header("Content-Length", str(len(body.encode())))
        self.end_headers()
        self.wfile.write(body.encode())

    def do_GET(self):
        if self.path == "/register":
            self.answer(200, "<form>", (("Set-Cookie", f"rullst_csrf={self.csrf}; Path=/; Secure; SameSite=Lax"),))
        elif self.path == "/dashboard" and self.cookies().get("rullst_session") == self.session:
            self.answer(200, "Welcome, ZAP Scanner")
        else:
            self.answer(303, "", (("Location", "/login"),))

    def do_POST(self):
        length = int(self.headers.get("Content-Length", "0"))
        form = urllib.parse.parse_qs(self.rfile.read(length).decode())
        valid = (self.path == "/register" and form.get("_token") == [self.csrf]
                 and self.cookies().get("rullst_csrf") == self.csrf
                 and len(form.get("password", [""])[0]) >= 12 and form.get("name") == ["ZAP Scanner"])
        if not valid:
            self.answer(403, "invalid")
            return
        self.answer(303, "", (("Location", "/dashboard"),
                              ("Set-Cookie", f"rullst_session={self.session}; Path=/; HttpOnly; Secure")))


class SessionTests(unittest.TestCase):
    def setUp(self):
        self.server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Starter)
        thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        thread.start()
        self.addCleanup(self.server.server_close)
        self.addCleanup(self.server.shutdown)
        temp = tempfile.TemporaryDirectory(prefix="rullst-dast-session-")
        self.addCleanup(temp.cleanup)
        self.environment = Path(temp.name) / "env"
        self.environment.write_text("")
        self.base = f"http://127.0.0.1:{self.server.server_address[1]}"

    def run_session(self) -> tuple[int, str]:
        output = io.StringIO()
        with (mock.patch.dict(os.environ, {"GITHUB_ENV": str(self.environment)}),
              contextlib.redirect_stdout(output), contextlib.redirect_stderr(output)):
            result = MODULE.main(["session", self.base])
        return result, output.getvalue()

    def test_secure_production_cookies_are_kept_over_http_and_masked(self):
        result, output = self.run_session()
        self.assertEqual(result, 0, output)
        self.assertEqual(self.environment.read_text(),
                         "SAAS_ZAP_COOKIE=rullst_session=encrypted-session-value; rullst_csrf=csrf-token-value\n")
        self.assertIn("::add-mask::encrypted-session-value", output)
        self.assertIn("::add-mask::csrf-token-value", output)

    def test_check_session_requires_the_dashboard_after_the_scan(self):
        result, output = self.run_session()
        self.assertEqual(result, 0, output)
        exported = self.environment.read_text().strip().split("=", 1)[1]
        with mock.patch.dict(os.environ, {"SAAS_ZAP_COOKIE": exported}), \
                contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(MODULE.main(["check-session", self.base]), 0)
        revoked = exported.replace("encrypted-session-value", "revoked")
        errors = io.StringIO()
        with mock.patch.dict(os.environ, {"SAAS_ZAP_COOKIE": revoked}), contextlib.redirect_stderr(errors):
            self.assertEqual(MODULE.main(["check-session", self.base]), 1)
        self.assertIn("no longer opens /dashboard (303 -> '/login')", errors.getvalue())

    def test_a_rejected_registration_exports_nothing(self):
        with mock.patch.object(Starter, "do_POST", lambda handler: handler.answer(403, "invalid")):
            result, output = self.run_session()
        self.assertEqual(result, 1)
        self.assertIn("POST /register answered 403", output)
        self.assertEqual(self.environment.read_text(), "")

    def test_rejects_a_non_http_base_url(self):
        self.base = "https://127.0.0.1:3000"
        result, output = self.run_session()
        self.assertEqual(result, 1)
        self.assertIn("http://host:port", output)


if __name__ == "__main__":
    unittest.main()
