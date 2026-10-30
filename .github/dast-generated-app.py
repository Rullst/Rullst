#!/usr/bin/env python3
"""Prepare generated applications for .github/workflows/dast-zap.yml.

registry-patch PROJECT
    Give the project's direct framework dependencies the registry form a
    published CLI writes, and resolve them from this checkout through
    `[patch.crates-io]` (as .github/check-vm-deploy.py does with the packaged
    archives). A prerelease CLI built from a checkout writes path dependencies
    and a stable one writes registry versions; the build is the same for both,
    so it keeps working at a stable version before publication.

session BASE_URL
    Register a disposable account on a running SaaS starter, check that the
    session opens the dashboard, and append the Cookie header for an
    authenticated ZAP baseline to $GITHUB_ENV as SAAS_ZAP_COOKIE, masked.

check-session BASE_URL
    Check that SAAS_ZAP_COOKIE still opens the dashboard, so the scan that
    used it was signed in from start to end.
"""

from __future__ import annotations

import argparse
import http.client
import json
import os
from pathlib import Path
import re
import secrets
import sys
import tomllib
import urllib.parse

ROOT = Path(__file__).resolve().parent.parent
LOOPBACK_HOSTS = ("127.0.0.1", "localhost", "::1")
DASHBOARD = "/dashboard"
USER_AGENT = "Mozilla/5.0 (Rullst DAST session)"
ACCOUNT_NAME = "ZAP Scanner"
DEPENDENCY = re.compile(r'^(?P<name>[A-Za-z0-9_-]+)\s*=\s*\{\s*path\s*=\s*(?:"[^"]*"|\'[^\']*\')')


class Failure(Exception):
    """A preparation step failed; the message says which."""


def release_crates() -> set[str]:
    order = json.loads((ROOT / ".github/release-order.json").read_text(encoding="utf-8"))
    return {crate for crate in order if crate != "cargo-rullst"}


def version_requirement(version: str) -> str:
    """The requirement the CLI writes (generators/project/cargo_toml.rs)."""
    return f"={version}" if "-" in version else version


def registry_patch(manifest: str, crates: set[str], version: str, checkout: Path) -> str:
    """Registry-shaped framework dependencies plus a patch to `checkout`."""
    if "[patch.crates-io]" in manifest:
        raise Failure("the generated manifest already has a [patch.crates-io] section")
    requirement = json.dumps(version_requirement(version))
    lines = []
    for line in manifest.splitlines(keepends=True):
        match = DEPENDENCY.match(line)
        if match is not None and match.group("name") in crates:
            line = f"{match.group('name')} = {{ version = {requirement}" + line[match.end():]
        lines.append(line)
    rewritten = "".join(lines)
    dependencies = tomllib.loads(rewritten).get("dependencies", {})
    direct = sorted(name for name in dependencies if name in crates)
    if "rullst" not in direct:
        raise Failure("the generated manifest does not depend on rullst")
    for name in direct:
        spec = dependencies[name]
        if not isinstance(spec, dict) or "path" in spec or "version" not in spec:
            raise Failure(f"{name} is not a registry dependency after the rewrite")
    patch = ["", "# Added by .github/dast-generated-app.py: this checkout's unpublished",
             "# crates replace the registry versions the manifest names.", "[patch.crates-io]"]
    patch += [f"{name} = {{ path = {json.dumps(str(checkout / name))} }}" for name in direct]
    return rewritten.rstrip("\n") + "\n" + "\n".join(patch) + "\n"


def command_registry_patch(project: Path) -> None:
    manifest_path = project / "Cargo.toml"
    version = tomllib.loads((ROOT / "rullst/Cargo.toml").read_text(encoding="utf-8"))["package"]["version"]
    manifest = registry_patch(manifest_path.read_text(encoding="utf-8"), release_crates(), version, ROOT)
    manifest_path.write_text(manifest, encoding="utf-8")
    print(f"{manifest_path}: framework crates {version_requirement(version)} patched to {ROOT}")


def set_cookies(response: http.client.HTTPResponse) -> dict[str, str]:
    cookies = {}
    for header in response.headers.get_all("Set-Cookie") or []:
        name, _, rest = header.partition("=")
        cookies[name.strip()] = rest.split(";", 1)[0].strip()
    return cookies


def request(base: urllib.parse.SplitResult, method: str, path: str, cookies: dict[str, str],
            form: dict[str, str] | None = None) -> tuple[int, dict[str, str], dict[str, str], str]:
    """One request without redirects. Production cookies are `Secure`, so they
    are kept by hand rather than by a cookie jar that would drop them on HTTP."""
    # Only the app this job started on the runner's loopback is ever contacted.
    if base.hostname not in LOOPBACK_HOSTS:
        raise Failure(f"refusing to contact {base.hostname!r}: only loopback hosts are scanned")
    connection = http.client.HTTPConnection(base.hostname, base.port, timeout=30)  # NOSONAR
    headers = {"User-Agent": USER_AGENT}
    if cookies:
        headers["Cookie"] = "; ".join(f"{name}={value}" for name, value in cookies.items())
    body = None
    if form is not None:
        headers["Content-Type"] = "application/x-www-form-urlencoded"
        body = urllib.parse.urlencode(form)
    try:
        connection.request(method, path, body=body, headers=headers)
        response = connection.getresponse()
        text = response.read(2 * 1024 * 1024).decode(errors="replace")
    except OSError as error:
        raise Failure(f"{method} {path} failed: {error}") from error
    finally:
        connection.close()
    location = response.headers.get("Location", "")
    return response.status, set_cookies(response), {"location": location}, text


def command_session(base_url: str) -> None:
    base = urllib.parse.urlsplit(base_url)
    if base.scheme != "http" or not base.hostname or not base.port:
        raise Failure("BASE_URL must be an http://host:port URL")
    cookies: dict[str, str] = {}
    status, issued, _, _ = request(base, "GET", "/register", cookies)
    cookies.update(issued)
    if status != 200 or not cookies.get("rullst_csrf"):
        raise Failure(f"GET /register answered {status} without a CSRF cookie")
    form = {"_token": cookies["rullst_csrf"], "name": ACCOUNT_NAME,
            "email": f"zap-{secrets.token_hex(6)}@example.com", "password": secrets.token_urlsafe(24)}
    status, issued, headers, _ = request(base, "POST", "/register", cookies, form)
    cookies.update(issued)
    if status not in (302, 303) or headers["location"] != DASHBOARD or not cookies.get("rullst_session"):
        raise Failure(f"POST /register answered {status} -> {headers['location']!r} without a session")
    status, issued, _, body = request(base, "GET", DASHBOARD, cookies)
    cookies.update(issued)
    if status != 200 or ACCOUNT_NAME not in body:
        raise Failure(f"GET {DASHBOARD} with the new session answered {status}")
    session = {name: cookies[name] for name in ("rullst_session", "rullst_csrf")}
    for value in session.values():
        print(f"::add-mask::{value}")
    header = "; ".join(f"{name}={value}" for name, value in session.items())
    environment = os.environ.get("GITHUB_ENV")
    if not environment:
        raise Failure("GITHUB_ENV is not set; the session is only exported in GitHub Actions")
    with open(environment, "a", encoding="utf-8") as handle:
        handle.write(f"SAAS_ZAP_COOKIE={header}\n")
    print("Registered a disposable account; its session opens /dashboard.")


def command_check_session(base_url: str) -> None:
    base = urllib.parse.urlsplit(base_url)
    header = os.environ.get("SAAS_ZAP_COOKIE", "")
    cookies = {name: value for name, _, value in
               (item.strip().partition("=") for item in header.split(";") if "=" in item)}
    if not cookies.get("rullst_session"):
        raise Failure("SAAS_ZAP_COOKIE holds no session")
    status, _, headers, body = request(base, "GET", DASHBOARD, cookies)
    if status != 200 or ACCOUNT_NAME not in body:
        raise Failure(f"the scanned session no longer opens {DASHBOARD} ({status} -> {headers['location']!r})")
    print("The session used by the scan still opens /dashboard.")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("registry-patch").add_argument("project", type=Path)
    commands.add_parser("session").add_argument("base_url")
    commands.add_parser("check-session").add_argument("base_url")
    args = parser.parse_args(argv)
    try:
        if args.command == "registry-patch":
            command_registry_patch(args.project)
        elif args.command == "session":
            command_session(args.base_url)
        else:
            command_check_session(args.base_url)
    except Failure as error:
        print(f"::error title=DAST preparation::{error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
